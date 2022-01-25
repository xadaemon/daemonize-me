use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::exit;

use nix::sys::stat::{Mode, umask};
#[cfg(not(target_os = "macos"))]
use nix::unistd::{
    chdir, chown, fork, ForkResult, getpid, Gid, initgroups, Pid, setgid, setsid,
    setuid, Uid,
};
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, ForkResult, getpid, Gid, Pid, setgid, setsid, setuid, Uid,
};

use crate::{DaemonError, Result};
use crate::ffi::{PasswdRecord, set_proc_name};
use crate::group::Group;
use crate::stdio::{redirect_stdio, Stdio};
use crate::user::User;

/// Basic daemonization consists of:
/// forking the process, getting a new sid, setting the umask, changing the standard io streams
/// to files and finally dropping privileges.
///
/// Options:
/// * user [optional], if set will drop privileges to the specified user **NOTE**: This library is strict and makes no assumptions if you provide a user you must provide a group
/// * group [optional(**see note on user**)], if set will drop privileges to specified group
/// * umask [optional], umask for the process defaults to 0o027
/// * pid_file [optional], if set a pid file will be created default is that no file is created *
/// * stdio [optional][**recommended**], this determines where standard output will be piped to since daemons have no console it's highly recommended to set this
/// * stderr [optional][**recommended**], same as above but for standard error
/// * chdir [optional], default is "/"
/// * name [optional], set the daemon process name eg what shows in `ps` default is to not set a process name
///
/// * See the setter function documentation for more details
///
/// **Beware there is no escalation back if dropping privileges**
pub struct Daemon {
    pub chdir: PathBuf,
    pub pid_file: Option<PathBuf>,
    pub chown_pid_file: bool,
    pub user: Option<User>,
    pub group: Option<Group>,
    pub umask: u16,
    pub stdin: Stdio,
    // stdin is practically always null
    pub stdout: Stdio,
    pub stderr: Stdio,
    pub name: Option<OsString>,
}

impl Daemon {
    pub fn new() -> Self {
        Daemon {
            chdir: Path::new("/").to_owned(),
            pid_file: None,
            chown_pid_file: false,
            user: None,
            group: None,
            umask: 0o027,
            stdin: Stdio::devnull(),
            stdout: Stdio::devnull(),
            stderr: Stdio::devnull(),
            name: None,
        }
    }

    /// This is a setter to give your daemon a pid file
    /// # Arguments
    /// * `path` - path to the file suggested `/var/run/my_program_name.pid`
    /// * `chmod` - if set a chmod of the file to the user and group passed will be attempted (**this being true makes setting an user and group mandatory**)
    pub fn pid_file<T: AsRef<Path>>(mut self, path: T, chmod: Option<bool>) -> Self {
        self.pid_file = Some(path.as_ref().to_owned());
        self.chown_pid_file = chmod.unwrap_or(false);
        self
    }
    /// As the last step the code will change the working directory to this one defaults to `/`
    pub fn work_dir<T: AsRef<Path>>(mut self, path: T) -> Self {
        self.chdir = path.as_ref().to_owned();
        self
    }
    /// The code will attempt to drop privileges with `setuid` to the provided user
    pub fn user<T: Into<User>>(mut self, user: T) -> Self {
        self.user = Some(user.into());
        self
    }
    /// The code will attempt to drop privileges with `setgid` to the provided group, you mut provide a group if you provide an user
    pub fn group<T: Into<Group>>(mut self, group: T) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn umask(mut self, mask: u16) -> Self {
        self.umask = mask;
        self
    }

    pub fn stdin<T: Into<Stdio>>(mut self, stdio: T) -> Self {
        self.stdin = stdio.into();
        self
    }

    pub fn stdout<T: Into<Stdio>>(mut self, stdio: T) -> Self {
        self.stdout = stdio.into();
        self
    }

    pub fn stderr<T: Into<Stdio>>(mut self, stdio: T) -> Self {
        self.stderr = stdio.into();
        self
    }

    pub fn name(mut self, name: &OsStr) -> Self {
        self.name = Some(OsString::from(name));
        self
    }

    /// Using the parameters set, daemonize the process
    pub fn start(self) -> Result<()> {
        let pid: Pid;
        // resolve options to concrete values to please the borrow checker
        let has_pid_file = self.pid_file.is_some();
        let pid_file_path = match self.pid_file {
            Some(path) => path.clone(),
            None => Path::new("").to_path_buf(),
        };
        // Set up stream redirection as early as possible
        redirect_stdio(&self.stdin, &self.stdout, &self.stderr)?;
        if self.chown_pid_file && (self.user.is_none() || self.group.is_none()) {
            return Err(DaemonError::InvalidUserGroupPair);
        } else if (self.user.is_some() || self.group.is_some())
            && (self.user.is_none() || self.group.is_none())
        {
            return Err(DaemonError::InvalidUserGroupPair);
        }
        // Fork and if the process is the parent exit gracefully
        // if the  process is the child just continue execution
        // this was made unsafe by the nix upstream in between versions
        // thus the unsafe block is required here
        unsafe {
            match fork() {
                Ok(ForkResult::Parent { child: _ }) => exit(0),
                Ok(ForkResult::Child) => (),
                Err(_) => return Err(DaemonError::Fork),
            }
        }
        if let Some(proc_name) = &self.name {
            match set_proc_name(proc_name.as_ref()) {
                Ok(()) => (),
                Err(e) => return Err(e)
            }
        }
        // Set the umask either to 0o027 (rwxr-x---) or provided value
        let umask_mode = match Mode::from_bits(self.umask as _) {
            Some(mode) => mode,
            None => return Err(DaemonError::InvalidUmaskBits),
        };
        umask(umask_mode);
        // Set the sid so the process isn't session orphan
        if let Err(_) = setsid() {
            return Err(DaemonError::SetSid);
        };
        if let Err(_) = chdir::<Path>(self.chdir.as_path()) {
            return Err(DaemonError::ChDir);
        };
        pid = getpid();
        // create pid file and if configured to, chmod it
        if has_pid_file {
            // chmod of the pid file is deferred to after checking for the presence of the user and group
            let pid_file = &pid_file_path;
            match File::create(pid_file) {
                Ok(mut fp) => {
                    if let Err(_) = fp.write_all(pid.to_string().as_ref()) {
                        return Err(DaemonError::WritePid);
                    }
                }
                Err(_) => return Err(DaemonError::WritePid),
            };
        }
        // Drop privileges and chown the requested files
        if self.user.is_some() && self.group.is_some() {
            let user = match self.user.unwrap() {
                User::Id(id) => Uid::from_raw(id),
            };

            let uname = match PasswdRecord::get_record_by_id(user.as_raw()) {
                Ok(record) => record.pw_name,
                Err(_) => return Err(DaemonError::InvalidUser),
            };

            let gr = match self.group.unwrap() {
                Group::Id(id) => Gid::from_raw(id),
            };

            if self.chown_pid_file && has_pid_file {
                match chown(&pid_file_path, Some(user), Some(gr)) {
                    Ok(_) => (),
                    Err(_) => return Err(DaemonError::ChownPid),
                };
            }

            match setgid(gr) {
                Ok(_) => (),
                Err(_) => return Err(DaemonError::SetGid),
            };
            #[cfg(not(target_os = "macos"))]
                {
                    let u_cstr = match CString::new(uname) {
                        Ok(cstr) => cstr,
                        Err(_) => return Err(DaemonError::SetGid),
                    };
                    match initgroups(&u_cstr, gr) {
                        Ok(_) => (),
                        Err(_) => return Err(DaemonError::InitGroups),
                    };
                }
            match setuid(user) {
                Ok(_) => (),
                Err(_) => return Err(DaemonError::SetUid),
            }
        }
        // chdir
        let chdir_path = self.chdir.to_owned();
        match chdir::<Path>(chdir_path.as_ref()) {
            Ok(_) => (),
            Err(_) => return Err(DaemonError::ChDir),
        };
        // Now this process should be a daemon, return
        Ok(())
    }
}


