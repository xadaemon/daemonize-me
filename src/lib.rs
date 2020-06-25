extern crate nix;
mod ffi;
use anyhow::{anyhow, Result};
use ffi::{GroupRecord, PasswdRecord};
use nix::sys::stat::{umask, Mode};
use nix::unistd::{chdir, fork, initgroups, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::exit;

/// Expects: either the username or the uid
/// if the name is provided it will be resolved to an id
#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub enum User {
    Id(u32),
}

impl<'uname> From<&'uname str> for User {
    fn from(uname: &'uname str) -> User {
        User::Id(PasswdRecord::get_record_by_name(uname).unwrap().pw_uid)
    }
}

impl From<String> for User {
    fn from(uname: String) -> User {
        User::Id(
            PasswdRecord::get_record_by_name(uname.as_str())
                .unwrap()
                .pw_uid,
        )
    }
}

impl From<u32> for User {
    fn from(uid: u32) -> User {
        User::Id(uid)
    }
}

/// Expects: either the group name or a gid
/// if the name is provided it will be resolved to an id
#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub enum Group {
    Id(u32),
}

impl<'uname> From<&'uname str> for Group {
    fn from(gname: &'uname str) -> Group {
        Group::Id(GroupRecord::get_record_by_name(gname).unwrap().gr_gid)
    }
}

impl From<String> for Group {
    fn from(gname: String) -> Group {
        Group::Id(
            GroupRecord::get_record_by_name(gname.as_str())
                .unwrap()
                .gr_gid,
        )
    }
}

impl From<u32> for Group {
    fn from(uid: u32) -> Group {
        Group::Id(uid)
    }
}

#[derive(Debug)]
enum StdioImp {
    Devnull,
    RedirectToFile(File),
}

/// describes what to do with a standard io stream for a child process.
#[derive(Debug)]
pub struct Stdio {
    inner: StdioImp,
}

impl Stdio {
    fn devnull() -> Self {
        Self {
            inner: StdioImp::Devnull,
        }
    }
}

impl From<File> for Stdio {
    fn from(file: File) -> Self {
        Self {
            inner: StdioImp::RedirectToFile(file),
        }
    }
}

/// Basic daemonization consists of:
/// forking the process, getting a new sid, setting the umask, changing the standard io streams
/// to files and finally dropping privileges.
///
/// Options:
/// * user [optional], if set will drop privileges to the specified user
/// * group [optional], if set will drop privileges to specified group
/// * umask [optional], umask for the process defaults to 0o027
/// * pid_file [optional], if set a pid file will be created default is that no file is created <sup>*</sup>
/// * stdio [optional][**recommended**], this determines where standard output will be piped to since daemons have no console it's highly recommended to set this
/// * stderr [optional][**recommended**], same as above but for standard error
/// * chdir [optional], default is "/"
///
/// <sup>*</sup> See the setter function documentation for more details
///
/// **Beware there is no escalation back if dropping privileges**
/// TODO:
/// * [ ] Add chroot option
/// * [ ] Add before drop lambda
pub struct Daemon {
    chdir: PathBuf,
    pid_file: Option<PathBuf>,
    chown_pid_file: bool,
    user: Option<User>,
    group: Option<Group>,
    umask: u16,
    stdin: Stdio, // stdin is practically always null
    stdout: Stdio,
    stderr: Stdio,
}

// TODO: Stream redirections

// TODO: Improve documentation
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
        }
    }

    /// This is a setter to give your daemon a pid file
    /// # Arguments
    /// * `path` - path to the file suggested `/var/run/myprogramname.pid`
    /// * `chmod` - if set a chmod of the file to the user and group passed will be attempted (this being true makes setting an user and group mandatory)
    pub fn pid_file<T: AsRef<Path>>(mut self, path: T, chmod: Option<bool>) -> Self {
        self.pid_file = Some(path.as_ref().to_owned());
        self.chown_pid_file = chmod.unwrap_or(false);
        self
    }

    pub fn work_dir<T: AsRef<Path>>(mut self, path: T) -> Self {
        self.chdir = path.as_ref().to_owned();
        self
    }

    pub fn user<T: Into<User>>(mut self, user: T) -> Self {
        self.user = Some(user.into());
        self
    }

    pub fn group<T: Into<Group>>(mut self, group: T) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn umask(mut self, mask: u16) -> Self {
        self.umask = mask;
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

    pub fn start(self) -> Result<Daemon> {
        let sid: Pid;
        let pid: Pid;
        if self.chown_pid_file && (self.user.is_none() && self.group.is_none()) {
            return Err(anyhow!(
                "You can't have chmod pid file without user and group"
            ));
        }

        // TODO: Redirect streams here

        match fork() {
            Ok(ForkResult::Parent { child: _ }) => exit(0),
            Ok(ForkResult::Child) => (),
            Err(_) => return Err(anyhow!("Failed to fork")),
        }
        umask(Mode::from_bits(self.umask as u32).unwrap());
        sid = setsid().expect("faield to setsid");
        if let Err(_) = chdir::<Path>(self.chdir.as_path()) {
            return Err(anyhow!("failed to chdir"));
        };

        Ok(self)
    }
}
