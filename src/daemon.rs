use crate::ffi::{set_proc_name, PasswdRecord};
use crate::process::{redirect_stdio, PidType, ProcessInfo, Stdio};
use crate::user::UserInfo;
use crate::Result;
use nix::errno::Errno;
use nix::sys::stat::{umask, Mode};
#[cfg(target_os = "macos")]
use nix::unistd::{
    chdir, chown, close, dup2, fork, getpid, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid,
};
#[cfg(not(target_os = "macos"))]
use nix::unistd::{
    chdir, chown, fork, getpid, initgroups, setgid, setsid, setuid, ForkResult, Gid, Pid, Uid,
};
use std::ffi::{CString, OsStr, OsString};
use std::fs::File;
use std::io::prelude::*;
use std::mem::MaybeUninit;
use std::path::Path;
use std::process::exit;
use std::sync::Once;

#[derive(Debug)]
pub struct Daemon {
    name: Option<OsString>,
    user_info: UserInfo,
    process_info: ProcessInfo,
}

pub fn get_daemon_instance() -> &'static Daemon {
    static mut CONF: MaybeUninit<Daemon> = MaybeUninit::uninit();
    static ONCE: Once = Once::new();

    ONCE.call_once(|| unsafe {
        CONF.as_mut_ptr().write(Daemon::new());
    });

    unsafe { &*CONF.as_ptr() }
}

macro_rules! multiple_daemon_setters {
    ($name:expr => $value:expr) => {{
        let daemon_instance = get_daemon_instance();
        daemon_instance.$name($value);
    }};
    ($name:expr => $value:expr, $($names:expr => $values:expr),+) => {
        multiple_daemon_setters! { $name => $value }
        multiple_daemon_setters! { ($names => $values),+ }
    };
}

// TODO: Improve documentation
impl Daemon {
    pub fn new() -> Self {
        Daemon {
            user_info: UserInfo::new(),
            process_info: ProcessInfo::new(),
            name: None,
        }
    }

    /// This is a setter to give your daemon a pid file
    /// # Arguments
    /// * `path` - path to the file suggested `/var/run/my_program_name.pid`
    /// * `chmod` - if set a chmod of the file to the user and group passed will be attempted (**this being true makes setting an user and group mandatory**)
    pub fn pid_file<T: AsRef<Path>>(mut self, path: T, chmod: Option<bool>) -> Self {
        self.process_info.pid = Some(PidType::File(path.as_ref().to_owned()));
        self.process_info.chown_pid_file = chmod.unwrap_or(false);
        self
    }
    /// This is a setter to give your daemon a pid number
    /// # Arguments
    /// * `num` - number of process(pid), create a path to file suggested `/var/run/num.pid`
    /// * `chmod` - if set a chmod of the file to the user and group passed will be attempted (**this being true makes setting an user and group mandatory**)
    pub fn pid_num(mut self, num: u32, chmod: Option<bool>) -> Self {
        self.process_info.pid = Some(PidType::Num(num));
        self.process_info.chown_pid_file = chmod.unwrap_or(false);
        self
    }
    /// As the last step the code will change the working directory to this one defaults to `/`
    pub fn work_dir<T: AsRef<Path>>(mut self, path: T) -> Self {
        self.process_info.chdir = path.as_ref().to_owned();
        self
    }
    /// The code will attempt to drop privileges with `setuid` to the provided user
    pub fn user_info<T: Into<UserInfo>>(mut self, user: T) -> Self {
        self.user_info = user.into();
        self
    }
    pub fn user(&self) -> Option<&UserInfo> {
        Some(&self.user_info)
    }

    pub fn umask(mut self, mask: u16) -> Self {
        self.process_info.umask = mask;
        self
    }

    pub fn stdin<T: Into<Stdio>>(mut self, stdio: T) -> Self {
        self.process_info.stdin = stdio.into();
        self
    }

    pub fn stdout<T: Into<Stdio>>(mut self, stdio: T) -> Self {
        self.process_info.stdout = stdio.into();
        self
    }

    pub fn stderr<T: Into<Stdio>>(mut self, stdio: T) -> Self {
        self.process_info.stderr = stdio.into();
        self
    }

    pub fn name(mut self, name: &OsStr) -> Self {
        self.name = Some(OsString::from(name));
        self
    }

    /// Using the parameters set, daemonize the process
    pub fn start(self) -> Result<()> {
        let pid: Pid;

        let process_info = self.process_info;
        let user_info = self.user_info;
        let name = self.name;

        // resolve options to concrete values to please the borrow checker
        let has_pid = process_info.have_pid();
        let pid_file_path = match process_info.pid {
            Some(PidType::File(path)) => path.clone(),
            Some(PidType::Num(num)) => {
                let mut construct_pid_path = Path::new("/var/run/").to_path_buf();
                construct_pid_path.push(format!("{}.pid", num));
                construct_pid_path
            }
            None => Path::new("").to_path_buf(),
        };
        // Set up stream redirection as early as possible
        redirect_stdio(
            &process_info.stdin,
            &process_info.stdout,
            &process_info.stderr,
        )?;
        if (process_info.chown_pid_file && user_info.is_some_none()) || user_info.is_some_none() {
            return Err(Errno::UnknownErrno);
        }
        // Fork and if the process is the parent exit gracefully
        // if the  process is the child just continue execution
        match fork() {
            Ok(ForkResult::Parent { child: _ }) => exit(0),
            Ok(ForkResult::Child) => (),
            Err(_) => return Err(Errno::last()),
        }
        if let Some(proc_name) = &name {
            match set_proc_name(proc_name.as_ref()) {
                Ok(()) => (),
                Err(e) => return Err(e),
            }
        }
        // Set the umask either to 0o027 (rwxr-x---) or provided value
        let umask_mode = match Mode::from_bits(process_info.umask as _) {
            Some(mode) => mode,
            None => return Err(Errno::last()),
        };
        umask(umask_mode);
        // Set the sid so the process isn't session orphan
        if let Err(_) = setsid() {
            return Err(Errno::last());
        };
        if let Err(_) = chdir::<Path>(process_info.chdir.as_path()) {
            return Err(Errno::last());
        };
        pid = getpid();
        if has_pid {
            let pid_file = &pid_file_path;
            match File::create(pid_file) {
                Ok(mut fp) => {
                    if let Err(_) = fp.write_all(pid.to_string().as_ref()) {
                        return Err(Errno::last());
                    }
                }
                Err(_) => return Err(Errno::last()),
            };
        }
        let user = Uid::from_raw(user_info.user);
        let uname = match PasswdRecord::get_record_by_id(user.as_raw()) {
            Ok(record) => record.pw_name,
            Err(_) => return Err(Errno::last()),
        };

        let gr = Gid::from_raw(user_info.group);
        if process_info.chown_pid_file && has_pid {
            match chown(&pid_file_path, Some(user), Some(gr)) {
                Ok(_) => (),
                Err(_) => return Err(Errno::last()),
            };
        }

        match setgid(gr) {
            Ok(_) => (),
            Err(_) => return Err(Errno::last()),
        };
        #[cfg(not(target_os = "macos"))]
        {
            let u_cstr = match CString::new(uname) {
                Ok(cstr) => cstr,
                Err(_) => return Err(Errno::last()),
            };
            match initgroups(&u_cstr, gr) {
                Ok(_) => (),
                Err(_) => return Err(Errno::last()),
            };
        }
        match setuid(user) {
            Ok(_) => (),
            Err(_) => return Err(Errno::last()),
        }
        let chdir_path = process_info.chdir.to_owned();
        match chdir::<Path>(chdir_path.as_ref()) {
            Ok(_) => (),
            Err(_) => return Err(Errno::last()),
        };
        Ok(())
    }
}
