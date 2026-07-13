use std::any::Any;
use std::convert::TryFrom;
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::exit;

use nix::sys::stat::{umask, Mode};
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, getpid, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid,
};
#[cfg(not(target_os = "macos"))]
use nix::unistd::{
    chdir, chown, fork, getpid, initgroups, setgid, setsid, setuid, ForkResult, Gid, Uid,
};

use crate::ffi::{set_proc_name, PasswdRecord};
use crate::group::Group;
use crate::stdio::{redirect_stdio, Stdio};
use crate::user::User;
use crate::DaemonError::{InvalidGroup, InvalidUser, StartNotCalled};
use crate::{DaemonError, Result};

/// Basic daemonization consists of:
/// forking the process, getting a new Session ID (sid), setting the umask, changing the standard io streams
/// to files and finally dropping privileges.
///
/// **NOTE:** Beware there is no escalation back if dropping privileges
#[derive(Debug, Clone)]
pub struct Daemon<'a> {
    pub(crate) chdir: PathBuf,
    pub(crate) pid_file: Option<PathBuf>,
    pub(crate) chown_pid_file: bool,
    pub(crate) user: Option<User>,
    pub(crate) group: Option<Group>,
    pub(crate) umask: u16,
    // stdin is practically always null
    pub(crate) stdin: Box<Stdio<'a>>,
    pub(crate) stdout: Box<Stdio<'a>>,
    pub(crate) stderr: Box<Stdio<'a>>,
    pub(crate) name: Option<OsString>,
    pub(crate) before_fork_hook: Option<fn(pid: i32)>,
    pub(crate) after_fork_parent_hook: Option<fn(parent_pid: i32, child_pid: i32) -> !>,
    pub(crate) after_fork_child_hook: Option<fn(parent_pid: i32, child_pid: i32) -> ()>,
    pub(crate) after_init_hook_data: Option<&'a dyn Any>,
    pub(crate) after_init_hook: Option<fn(Option<&'a dyn Any>)>,
    pub(crate) child_pid: Option<i32>,
    pub(crate) parent_pid: i32,
    pub(crate) is_child: bool,
    pub(crate) has_forked: bool,
    pub(crate) dropped_privileges: bool,
}

pub struct PidPair {
    pub child_pid: i32,
    pub parent_pid: i32,
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
            stdin: Box::new(Stdio::devnull()),
            stdout: Box::new(Stdio::devnull()),
            stderr: Box::new(Stdio::devnull()),
            name: None,
            before_fork_hook: None,
            after_fork_parent_hook: None,
            after_fork_child_hook: None,
            after_init_hook_data: None,
            after_init_hook: None,
            child_pid: None,
            parent_pid: getpid().as_raw(),
            has_forked: false,
            is_child: false,
            dropped_privileges: false,
        }
    }

    /// Give your daemon a pid file
    ///
    /// By default, no pid file is created.
    ///
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
    ///
    /// **NOTE:** If you provide a user, you must also provide a group.
    pub fn user<T: Into<User>>(mut self, user: T) -> Self {
        self.user = Some(user.into());
        self
    }

    /// The code will attempt to drop privileges with `setgid` to the provided group
    ///
    /// **NOTE:** You must provide a group if you provide an user.
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

    /// umask for the process, defaults to `0o027`
    pub fn umask(mut self, mask: u16) -> Self {
        self.umask = mask;
        self
    }

    /// Set this to be able to give inputs via stdio to the child
    pub fn stdin<T: Into<Stdio<'a>>>(mut self, stdio: T) -> Self {
        self.stdin = Box::new(stdio.into());
        self
    }

    /// Determines where standard output will be piped to since daemons have no console attached
    ///
    /// It's highly recommended to set this to a file if you want to see output.
    pub fn stdout<T: Into<Stdio<'a>>>(mut self, stdio: T) -> Self {
        self.stdout = Box::new(stdio.into());
        self
    }

    /// Determines where standard error will be piped to since daemons have no console attached
    ///
    /// It's highly recommended to set this to a file if you want to see output.
    pub fn stderr<T: Into<Stdio<'a>>>(mut self, stdio: T) -> Self {
        self.stderr = Box::new(stdio.into());
        self
    }

    /// Set the daemon process name
    ///
    /// For example, this is what shows up in `ps`.
    pub fn name(mut self, name: &OsStr) -> Self {
        self.name = Some(OsString::from(name));
        self
    }

    /// Hook called before the fork with the current pid as argument
    pub fn setup_pre_fork_hook(mut self, pre_fork_hook: fn(pid: i32)) -> Self {
        self.before_fork_hook = Some(pre_fork_hook);
        self
    }

    /// Hook called after the fork with the parent pid as argument
    ///
    /// Can be used to continue some work on the parent after the fork.
    /// **NOTE:** This hook must not return! For instance, you could call `std::process::exit()`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use daemonize_me::Daemon;
    ///
    /// fn post_fork_parent(ppid: i32, cpid: i32) -> ! {
    ///     println!("Parent pid: {}, Child pid {}", ppid, cpid);
    ///     println!("Exiting parent now");
    ///     std::process::exit(0);
    /// }
    ///
    /// let daemon = Daemon::new()
    ///     .setup_post_fork_parent_hook(post_fork_parent)
    ///     .start();
    /// ```
    pub fn setup_post_fork_parent_hook(
        mut self,
        post_fork_parent_hook: fn(parent_pid: i32, child_pid: i32) -> !,
    ) -> Self {
        self.after_fork_parent_hook = Some(post_fork_parent_hook);
        self
    }

    /// Hook called after the fork with the parent and child pid as arguments
    ///
    /// # Examples
    ///
    /// ```
    /// # use daemonize_me::Daemon;
    ///
    /// fn post_fork_child(ppid: i32, cpid: i32) {
    ///     println!("Parent pid: {}, Child pid {}", ppid, cpid);
    ///     println!("This hook is called in the child");
    ///     // Child hook must return
    ///     return
    /// }
    ///
    /// let daemon = Daemon::new()
    ///     .setup_post_fork_child_hook(post_fork_child)
    ///     .start();
    /// ```
    pub fn setup_post_fork_child_hook(
        mut self,
        post_fork_child_hook: fn(parent_pid: i32, child_pid: i32) -> (),
    ) -> Self {
        self.after_fork_child_hook = Some(post_fork_child_hook);
        self
    }

    /// Hook called right before returning control to the caller, that is, right after `start()`
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::any::Any;
    /// # use daemonize_me::Daemon;
    ///
    /// fn after_init(_: Option<&dyn Any>) {
    ///     println!("Initialized the daemon!");
    ///     return
    /// }
    ///
    /// let daemon = Daemon::new()
    ///     .setup_post_init_hook(after_init, None)
    ///     .start();
    /// ```
    pub fn setup_post_init_hook(
        mut self,
        post_fork_child_hook: fn(ctx: Option<&'a dyn Any>),
        data: Option<&'a dyn Any>,
    ) -> Self {
        self.after_init_hook = Some(post_fork_child_hook);
        self.after_init_hook_data = data;
        self
    }

    pub fn is_child(self) -> bool {
        self.is_child
    }

    pub fn get_parent_pid(self) -> i32 {
        self.parent_pid
    }

    pub fn get_child_pid(self) -> Option<i32> {
        self.child_pid
    }

    pub fn get_pids(&self) -> Result<PidPair> {
        if let Some(cpid) = self.child_pid {
            Ok(PidPair {
                child_pid: cpid,
                parent_pid: self.parent_pid,
            })
        } else {
            Err(StartNotCalled)
        }
    }

    fn do_fork(&mut self) -> Result<()> {
        unsafe {
            match fork() {
                Ok(ForkResult::Parent { child: cpid }) => {
                    self.is_child = false;
                    self.child_pid = Some(cpid.as_raw());

                    if let Some(hook) = self.after_fork_parent_hook {
                        hook(self.parent_pid, cpid.as_raw());
                    } else {
                        exit(0)
                    }
                }
                Ok(ForkResult::Child) => {
                    // Set up stream redirection as early as possible
                    redirect_stdio(&self.stdin, &self.stdout, &self.stderr)?;
                    let pid = getpid();
                    self.is_child = true;
                    self.child_pid = Some(pid.as_raw());

                    if let Some(hook) = self.after_fork_child_hook {
                        hook(self.parent_pid, pid.as_raw());
                    }
                    ()
                }
                Err(_) => return Err(DaemonError::Fork),
            }
            self.has_forked = true;
            Ok(())
        }
    }

    fn do_chdir(&self) -> Result<()> {
        let chdir_path = self.chdir.to_owned();
        match chdir::<Path>(chdir_path.as_ref()) {
            Ok(_) => Ok(()),
            Err(_) => Err(DaemonError::ChDir),
        }
    }

    fn setup_pid_file(&self, pid_file_path: &PathBuf) -> Result<()> {
        let pid_file = &pid_file_path;

        let user = match self.user.clone() {
            Some(user) => Uid::from_raw(user.id),
            None => return Err(InvalidUser),
        };

        let gr = match self.group.clone() {
            Some(grp) => Gid::from_raw(grp.id),
            None => return Err(InvalidGroup),
        };

        match File::create(pid_file) {
            Ok(mut fp) => {
                if let Err(_) = fp.write_all(self.child_pid.unwrap().to_string().as_ref()) {
                    return Err(DaemonError::WritePid);
                }
            }
            Err(_) => return Err(DaemonError::WritePid),
        }

        if self.chown_pid_file && self.pid_file.is_some() {
            match chown::<PathBuf>(pid_file_path, Some(user), Some(gr)) {
                Ok(_) => return Ok(()),
                Err(_) => return Err(DaemonError::ChownPid),
            };
        };
        Ok(())
    }

    fn check_chown_precodnitions(&self) -> Result<()> {
        if self.chown_pid_file && (self.user.is_none() || self.group.is_none()) {
            return Err(DaemonError::InvalidUserGroupPair);
        } else if (self.user.is_some() || self.group.is_some())
            && (self.user.is_none() || self.group.is_none())
        {
            return Err(DaemonError::InvalidUserGroupPair);
        } else {
            Ok(())
        }
    }

    fn setup_privileges(&mut self) -> Result<()> {
        self.check_chown_precodnitions()?;

        let pid_file_path = match self.pid_file.clone() {
            Some(path) => path.clone(),
            None => Path::new("").to_path_buf(),
        };

        if self.pid_file.is_some() {
            self.setup_pid_file(&pid_file_path)?;
        }

        // We did the check in self.check_chown_precondition
        Ok({
            let user = match self.user.clone() {
                Some(user) => Uid::from_raw(user.id),
                None => return Err(InvalidUser),
            };

            let uname = match PasswdRecord::lookup_record_by_id(user.as_raw()) {
                Ok(record) => record.pw_name,
                Err(_) => return Err(DaemonError::InvalidUser),
            };

            let gr = match self.group.clone() {
                Some(grp) => Gid::from_raw(grp.id),
                None => return Err(InvalidGroup),
            };

            // change proc group
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

            // change the proc uid
            match setuid(user) {
                Ok(_) => (),
                Err(_) => return Err(DaemonError::SetUid),
            }
            self.dropped_privileges = true;
        })
    }

    /// Using the parameters set, daemonize the process
    pub fn start(&mut self) -> Result<PidPair> {
        // self pid is set on the constructor

        // If the hook is set call it with the parent pid
        if let Some(hook) = self.before_fork_hook {
            hook(self.parent_pid);
        }

        // Fork and if the process is the parent exit gracefully
        // if the  process is the child just continue execution
        // this was made unsafe by the nix upstream in between versions
        // thus the unsafe block is required here
        self.do_fork()?;

        if let Some(proc_name) = &self.name {
            match set_proc_name(proc_name.as_ref()) {
                Ok(()) => (),
                Err(e) => return Err(e),
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
        // Do the final chdir before dropping privileges
        if let Err(_) = chdir::<Path>(self.chdir.as_path()) {
            return Err(DaemonError::ChDir);
        };

        // create pid file and if configured to, chmod it
        // Drop privileges and chown the requested files
        self.setup_privileges()?;

        // chdir
        self.do_chdir()?;

        let pid_pair = if let Ok(pair) = self.get_pids() {
            pair
        } else {
            unreachable!("This call should be impossible to fail, check if do_fork is updating the pid correctly")
        };

        // Now this process should be a daemon, we run the hook and return or just return
        if let Some(hook) = self.after_init_hook {
            hook(self.after_init_hook_data);
            Ok(pid_pair)
        } else {
            Ok(pid_pair)
        }
    }
}
