use std::fmt::Debug;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use nix::fcntl::{OFlag, open};
use nix::sys::stat::Mode;
#[cfg(not(target_os = "macos"))]
use nix::unistd::{
    close, dup2,
};
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, ForkResult, getpid, Gid, Pid, setgid, setsid, setuid, Uid,
};

use crate::{DaemonError, Result};

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
    pub(crate) fn devnull() -> Self {
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