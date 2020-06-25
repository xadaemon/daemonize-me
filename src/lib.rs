mod ffi;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Expects: either the username or the uid
/// if the name is provided it will be resolved to an id
#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub enum User {
    Name(String),
    Id(u32),
}

impl<'uname> From<&'uname str> for User {
    fn from(uname: &'uname str) -> User {
        User::Name(uname.to_owned())
    }
}

impl From<String> for User {
    fn from(uname: String) -> User {
        User::Name(uname)
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
    Name(String),
    Id(u32),
}

impl<'uname> From<&'uname str> for Group {
    fn from(uname: &'uname str) -> Group {
        Group::Name(uname.to_owned())
    }
}

impl From<String> for Group {
    fn from(uname: String) -> Group {
        Group::Name(uname)
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
}

#[cfg(test)]
mod tests {}
