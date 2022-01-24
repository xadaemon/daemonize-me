use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;

use crate::Result;
use nix::errno::Errno;
use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, getpid, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid,
};
#[cfg(not(target_os = "macos"))]
use nix::unistd::{close, dup2};
use std::os::unix::io::AsRawFd;
use std::path::Path;
#[derive(Debug)]
pub struct ProcessInfo {
    pub chown_pid_file: bool,
    pub chdir: PathBuf,
    pub pid: Option<PidType>,
    pub umask: u16,
    pub stdin: Stdio, // stdin is practically always null
    pub stdout: Stdio,
    pub stderr: Stdio,
}

impl ProcessInfo {
    pub fn have_pid(&self) -> bool {
        self.pid.is_some()
    }
    pub fn have_chown_pid_file(&self) -> bool {
        self.chown_pid_file
    }
    pub fn new() -> Self {
        ProcessInfo {
            chown_pid_file: false,
            chdir: PathBuf::new(),
            pid: None,
            umask: 0,
            stdin: Stdio::devnull(),
            stdout: Stdio::devnull(),
            stderr: Stdio::devnull(),
        }
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
    pub fn devnull() -> Self {
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

#[derive(Debug)]
pub enum PidType {
    Num(u32),
    File(PathBuf),
}

fn close_and_dub(fd_old: i32, fd_new: i32) -> Result<()> {
    match close(fd_old) {
        Ok(_) => (),
        Err(_) => return Err(Errno::last()),
    };
    match dup2(fd_old, fd_new) {
        Ok(_) => (),
        Err(_) => return Err(Errno::last()),
    };
    Ok(())
}

pub fn redirect_stdio(stdin: &Stdio, stdout: &Stdio, stderr: &Stdio) -> Result<()> {
    let devnull_fd = match open(
        Path::new("/dev/null"),
        OFlag::O_APPEND,
        Mode::from_bits(OFlag::O_RDWR.bits() as _).unwrap(),
    ) {
        Ok(fd) => fd,
        Err(_) => return Err(Errno::last()),
    };

    let proc_stream = |fd, stdio: &Stdio| match &stdio.inner {
        StdioImp::Devnull => close_and_dub(fd, devnull_fd),
        StdioImp::RedirectToFile(file) => close_and_dub(fd, file.as_raw_fd()),
    };

    proc_stream(libc::STDIN_FILENO, stdin)?;
    proc_stream(libc::STDOUT_FILENO, stdout)?;
    proc_stream(libc::STDERR_FILENO, stderr)?;

    Ok(())
}
