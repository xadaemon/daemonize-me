use std::fmt::Debug;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use nix::fcntl::{open, OFlag};
use nix::sys::stat::Mode;
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, getpid, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid,
};
#[cfg(not(target_os = "macos"))]
use nix::unistd::{close, dup2};

use crate::{DaemonError, Result};

#[derive(Debug, Clone)]
enum StdioImp<'file> {
    Devnull,
    RedirectToFile(&'file File),
}

/// describes what to do with a standard io stream for a child process.
#[derive(Debug, Clone)]
pub struct Stdio<'file> {
    inner: Box<StdioImp<'file>>,
}

impl<'file> Stdio<'file> {
    pub(crate) fn devnull() -> Self {
        Self {
            inner: Box::new(StdioImp::Devnull),
        }
    }
}

impl<'file> From<&'file File> for Stdio<'file> {
    fn from(file: &'file File) -> Self {
        Self {
            inner: Box::new(StdioImp::RedirectToFile(file)),
        }
    }
}

// impl<'file> From<File> for Stdio<'file> {}

pub(crate) fn redirect_stdio(stdin: &Stdio, stdout: &Stdio, stderr: &Stdio) -> Result<()> {
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
        return match &stdio.inner.as_ref() {
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
