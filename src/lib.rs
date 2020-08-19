#![deny(warnings)]
#![deny(clippy::complexity)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::trivially_copy_pass_by_ref)]

mod ffi;

extern crate libc;
extern crate nix;
use ffi::{GroupRecord, PasswdRecord};
use nix::fcntl::{open, OFlag};
use nix::sys::stat::{umask, Mode};
#[cfg(not(target_os = "macos"))]
use nix::unistd::{
    chdir, chown, close, dup2, fork, getpid, initgroups, setgid, setsid, setuid, ForkResult, Gid,
    Pid, Uid,
};
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, getpid, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid,
};
use snafu::Snafu;
use std::convert::TryFrom;
use std::ffi::CString;
use std::fmt::Debug;
use std::fs::File;
use std::io::prelude::*;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::exit;

#[derive(Debug, Snafu)]
pub enum DaemonError {
    /// Unable to fork
    Fork,
    /// Failed to chdir
    ChDir,
    /// Failed to open dev null
    OpenDevNull,
    /// Failed to close the file pointer of a stdio stream
    CloseFp,
    /// Invalid or nonexistent user
    InvalidUser,
    /// Invalid or nonexistent group
    InvalidGroup,
    /// Either group or user was specified but no the other
    InvalidUserGroupPair,
    /// The specified cstr is invalid
    InvalidCstr,
    /// Failed to execute initgroups
    InitGroups,
    /// Failed to set uid
    SetUid,
    /// Failed to set gid
    SetGid,
    /// Failed to chown the pid file
    ChownPid,
    /// Failed to create the pid file
    OpenPid,
    /// Failed to write to the pid file
    WritePid,
    /// Failed to redirect the standard streams
    RedirectStream,
    /// Umask bits are invalid
    InvalidUmaskBits,
    /// Failed to set sid
    SetSid,
    /// Failed to get groups record
    GetGrRecord,
    /// Failed to get passwd record
    GetPasswdRecord,
    #[doc(hidden)]
    __Nonexhaustive,
}

pub type Result<T> = std::result::Result<T, DaemonError>;

/// Expects: either the username or the uid
/// if the name is provided it will be resolved to an id
#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub enum User {
    Id(u32),
}

impl<'uname> TryFrom<&'uname str> for User {
    type Error = DaemonError;

    fn try_from(uname: &'uname str) -> Result<User> {
        match PasswdRecord::get_record_by_name(uname) {
            Ok(record) => Ok(User::Id(record.pw_uid)),
            Err(_) => Err(DaemonError::InvalidUser),
        }
    }
}

impl TryFrom<String> for User {
    type Error = DaemonError;

    fn try_from(uname: String) -> Result<User> {
        match PasswdRecord::get_record_by_name(uname.as_str()) {
            Ok(record) => Ok(User::Id(record.pw_uid)),
            Err(_) => Err(DaemonError::InvalidUser),
        }
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

impl<'uname> TryFrom<&'uname str> for Group {
    type Error = DaemonError;

    fn try_from(gname: &'uname str) -> Result<Group> {
        match GroupRecord::get_record_by_name(gname) {
            Ok(record) => Ok(Group::Id(record.gr_gid)),
            Err(_) => Err(DaemonError::InvalidGroup),
        }
    }
}

impl TryFrom<String> for Group {
    type Error = DaemonError;

    fn try_from(gname: String) -> Result<Group> {
        match GroupRecord::get_record_by_name(gname.as_str()) {
            Ok(record) => Ok(Group::Id(record.gr_gid)),
            Err(_) => Err(DaemonError::InvalidGroup),
        }
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
/// * user [optional], if set will drop privileges to the specified user **NOTE**: This library is strict and makes no assumptions if you provide a user you must provide a group  
/// * group [optional(**see note on user**)], if set will drop privileges to specified group
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

fn redirect_stdio(stdin: &Stdio, stdout: &Stdio, stderr: &Stdio) -> Result<()> {
    let devnull_fd = match open(
        Path::new("/dev/null"),
        OFlag::O_APPEND,
        Mode::from_bits(OFlag::O_RDWR.bits() as _).unwrap(),
    ) {
        Ok(fd) => fd,
        Err(_) => return Err(DaemonError::OpenDevNull),
    };
    let proc_stream = |fd, stdio: &Stdio| {
        match close(fd) {
            Ok(_) => (),
            Err(_) => return Err(DaemonError::CloseFp),
        };
        return match &stdio.inner {
            StdioImp::Devnull => match dup2(devnull_fd, fd) {
                Ok(_) => Ok(()),
                Err(_) => Err(DaemonError::RedirectStream),
            },
            StdioImp::RedirectToFile(file) => {
                let raw_fd = file.as_raw_fd();
                match dup2(raw_fd, fd) {
                    Ok(_) => Ok(()),
                    Err(_) => Err(DaemonError::RedirectStream),
                }
            }
        };
    };

    proc_stream(libc::STDIN_FILENO, stdin)?;
    proc_stream(libc::STDOUT_FILENO, stdout)?;
    proc_stream(libc::STDERR_FILENO, stderr)?;

    Ok(())
}

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
    /// Using the parameters set daemonize the process
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
        match fork() {
            Ok(ForkResult::Parent { child: _ }) => exit(0),
            Ok(ForkResult::Child) => (),
            Err(_) => return Err(DaemonError::Fork),
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

#[cfg(test)]
mod tests {
    // TODO: Improve testing coverage
    extern crate nix;
    use super::*;

    #[test]
    fn test_uname_to_uid_resolution() {
        let daemon = Daemon::new().user(User::try_from("root").unwrap());
        assert!(daemon.user.is_some());
        let uid = match daemon.user.unwrap() {
            User::Id(id) => id,
        };
        assert_eq!(uid, 0)
    }
}
