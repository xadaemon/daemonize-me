use std::any::Any;
use std::convert::TryFrom;
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
use crate::DaemonError::{InvalidGroup, InvalidUser};
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
/// * before_fork_hook [optional], called before the fork with the current pid as argument
/// * after_fork_parent_hook [optional], called after the fork with the parent pid as argument, can be used to continue some work on the parent after the fork (do not return)
/// * after_fork_child_hook [optional], called after the fork with the parent and child pid as arguments
///
/// * See the setter function documentation for more details
///
/// **Beware there is no escalation back if dropping privileges**
pub struct Daemon<'a> {
    pub(crate) chdir: PathBuf,
    pub(crate) pid_file: Option<PathBuf>,
    pub(crate) chown_pid_file: bool,
    pub(crate) user: Option<User>,
    pub(crate) group: Option<Group>,
    pub(crate) umask: u16,
    // stdin is practically always null
    pub(crate) stdin: Stdio,
    pub(crate) stdout: Stdio,
    pub(crate) stderr: Stdio,
    pub(crate) name: Option<OsString>,
    pub(crate) before_fork_hook: Option<fn(pid: i32)>,
    pub(crate) after_fork_parent_hook: Option<fn(parent_pid: i32, child_pid: i32) -> !>,
    pub(crate) after_fork_child_hook: Option<fn(parent_pid: i32, child_pid: i32) -> ()>,
    pub(crate) after_init_hook_data: Option<&'a dyn Any>,
    pub(crate) after_init_hook: Option<fn(Option<&'a dyn Any>)>,
}

impl<'a> Daemon<'a> {
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
            before_fork_hook: None,
            after_fork_parent_hook: None,
            after_fork_child_hook: None,
            after_init_hook_data: None,
            after_init_hook: None,
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

    pub fn group_copy_user(mut self) -> Result<Self> {
        if let Some(user) = &self.user {
            self.group = Some(Group::try_from(&user.name)?);
            Ok(self)
        } else {
            Err(InvalidUser)
        }
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

    pub fn setup_pre_fork_hook(mut self, pre_fork_hook: fn(pid: i32)) -> Self {
        self.before_fork_hook = Some(pre_fork_hook);
        self
    }

    pub fn setup_post_fork_parent_hook(mut self, post_fork_parent_hook: fn(parent_pid: i32, child_pid: i32) -> !) -> Self {
        self.after_fork_parent_hook = Some(post_fork_parent_hook);
        self
    }

    pub fn setup_post_fork_child_hook(mut self, post_fork_child_hook: fn(parent_pid: i32, child_pid: i32) -> ()) -> Self {
        self.after_fork_child_hook = Some(post_fork_child_hook);
        self
    }

    pub fn setup_post_init_hook(mut self, post_fork_child_hook: fn(ctx: Option<&'a dyn Any>),
                                data: Option<&'a dyn Any>) -> Self {
        self.after_init_hook = Some(post_fork_child_hook);
        self.after_init_hook_data = data;
        self
    }

    /// Using the parameters set, daemonize the process
    pub fn start(self) -> Result<()> {
        let mut pid: Pid;
        let parent_pid = getpid();
        // resolve options to concrete values to please the borrow checker
        let has_pid_file = self.pid_file.is_some();
        let pid_file_path = match self.pid_file {
            Some(path) => path.clone(),
            None => Path::new("").to_path_buf(),
        };

        // If the hook is set call it with the parent pid
        if let Some(hook) = self.before_fork_hook {
            hook(parent_pid.as_raw());
        }

        // Fork and if the process is the parent exit gracefully
        // if the  process is the child just continue execution
        // this was made unsafe by the nix upstream in between versions
        // thus the unsafe block is required here
        unsafe {
            match fork() {
                Ok(ForkResult::Parent { child: cpid }) => {
                    if let Some(hook) = self.after_fork_parent_hook {
                        hook(parent_pid.as_raw(), cpid.as_raw());
                    } else {
                        exit(0)
                    }
                }
                Ok(ForkResult::Child) => {
                    // Set up stream redirection as early as possible
                    redirect_stdio(&self.stdin, &self.stdout, &self.stderr)?;
                    pid = getpid();
                    if let Some(hook) = self.after_fork_child_hook {
                        hook(parent_pid.as_raw(), pid.as_raw());
                    }
                    ()
                }
                Err(_) => return Err(DaemonError::Fork),
            }
        }

        if self.chown_pid_file && (self.user.is_none() || self.group.is_none()) {
            return Err(DaemonError::InvalidUserGroupPair);
        } else if (self.user.is_some() || self.group.is_some())
            && (self.user.is_none() || self.group.is_none())
        {
            return Err(DaemonError::InvalidUserGroupPair);
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
            let user = match self.user {
                Some(user) => Uid::from_raw(user.id),
                None => return Err(InvalidUser),
            };

            let uname = match PasswdRecord::lookup_record_by_id(user.as_raw()) {
                Ok(record) => record.pw_name,
                Err(_) => return Err(DaemonError::InvalidUser),
            };

            let gr = match self.group {
                Some(grp) => Gid::from_raw(grp.id),
                None => return Err(InvalidGroup),
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
        };
        // chdir
        let chdir_path = self.chdir.to_owned();
        match chdir::<Path>(chdir_path.as_ref()) {
            Ok(_) => (),
            Err(_) => return Err(DaemonError::ChDir),
        };

        // Now this process should be a daemon, we run the hook and return or just return
        if let Some(hook) = self.after_init_hook {
            hook(self.after_init_hook_data);
            Ok(())
        } else {
            Ok(())
        }
    }
}
