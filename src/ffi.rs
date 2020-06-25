extern crate libc;

use anyhow::{anyhow, Context, Result};
use std::ffi::{CStr, CString};

#[repr(C)]
#[allow(dead_code)]
struct group {
    gr_name: *const libc::c_char,
    gr_passwd: *const libc::c_char,
    gr_gid: libc::gid_t,
    gr_mem: *const *const libc::c_char,
}

#[repr(C)]
#[allow(dead_code)]
struct passwd {
    pw_name: *const libc::c_char,
    pw_passwd: *const libc::c_char,
    pw_uid: libc::uid_t,
    pw_gid: libc::gid_t,
    pw_gecos: *const libc::c_char,
    pw_dir: *const libc::c_char,
    pw_shell: *const libc::c_char,
}

extern "C" {
    fn getgrnam(name: *const libc::c_char) -> *const group;
    fn getgrgid(name: libc::gid_t) -> *const group;
    fn getpwnam(name: *const libc::c_char) -> *const passwd;
    fn getpwuid(name: libc::uid_t) -> *const passwd;
}

#[derive(Debug)]
pub struct GroupRecord {
    pub gr_name: String,
    pub gr_passwd: String,
    pub gr_gid: u32,
}

#[derive(Debug)]
pub struct PasswdRecord {
    pub pw_name: String,
    pub pw_passwd: String,
    pub pw_uid: u32,
    pub pw_gid: u32,
    pub pw_gecos: String,
    pub pw_dir: String,
    pub pw_shell: String,
}

impl GroupRecord {
    pub fn get_record_by_name(name: &str) -> Result<GroupRecord> {
        let record_name =
            CString::new(name).with_context(|| format!("Failed to create cstr from {}", name))?;

        unsafe {
            let raw_passwd = getgrnam(record_name.as_ptr());
            if raw_passwd.is_null() {
                return Err(anyhow!("Failed to retrieve the records"));
            } else {
                let gr = &*raw_passwd;
                let sgr = GroupRecord {
                    gr_name: CStr::from_ptr(gr.gr_name).to_str()?.to_string(),
                    gr_passwd: CStr::from_ptr(gr.gr_passwd).to_str()?.to_string(),
                    gr_gid: gr.gr_gid as u32,
                };
                return Ok(sgr);
            }
        };
    }
    pub fn get_record_by_id(gid: u32) -> Result<GroupRecord> {
        let record_id = gid as libc::uid_t;

        unsafe {
            let raw_passwd = getgrgid(record_id);
            if raw_passwd.is_null() {
                return Err(anyhow!("Failed to retrieve the records"));
            } else {
                let gr = &*raw_passwd;
                let sgr = GroupRecord {
                    gr_name: CStr::from_ptr(gr.gr_name).to_str()?.to_string(),
                    gr_passwd: CStr::from_ptr(gr.gr_passwd).to_str()?.to_string(),
                    gr_gid: gr.gr_gid as u32,
                };
                return Ok(sgr);
            }
        };
    }
}

impl PasswdRecord {
    pub fn get_record_by_name(name: &str) -> Result<PasswdRecord> {
        let record_name =
            CString::new(name).with_context(|| format!("Failed to create cstr from {}", name))?;

        unsafe {
            let raw_passwd = getpwnam(record_name.as_ptr());
            if raw_passwd.is_null() {
                return Err(anyhow!("Failed to retrieve the records"));
            } else {
                let pw = &*raw_passwd;
                let pwr = PasswdRecord {
                    pw_name: CStr::from_ptr(pw.pw_name).to_str()?.to_string(),
                    pw_passwd: CStr::from_ptr(pw.pw_passwd).to_str()?.to_string(),
                    pw_uid: pw.pw_uid as u32,
                    pw_gid: pw.pw_gid as u32,
                    pw_gecos: CStr::from_ptr(pw.pw_gecos).to_str()?.to_string(),
                    pw_dir: CStr::from_ptr(pw.pw_dir).to_str()?.to_string(),
                    pw_shell: CStr::from_ptr(pw.pw_shell).to_str()?.to_string(),
                };
                return Ok(pwr);
            }
        };
    }
    pub fn get_record_by_id(uid: u32) -> Result<PasswdRecord> {
        let record_id = uid as libc::uid_t;

        unsafe {
            let raw_passwd = getpwuid(record_id);
            if raw_passwd.is_null() {
                return Err(anyhow!("Failed to retrieve the records"));
            } else {
                let pw = &*raw_passwd;
                let pwr = PasswdRecord {
                    pw_name: CStr::from_ptr(pw.pw_name).to_str()?.to_string(),
                    pw_passwd: CStr::from_ptr(pw.pw_passwd).to_str()?.to_string(),
                    pw_uid: pw.pw_uid as u32,
                    pw_gid: pw.pw_gid as u32,
                    pw_gecos: CStr::from_ptr(pw.pw_gecos).to_str()?.to_string(),
                    pw_dir: CStr::from_ptr(pw.pw_dir).to_str()?.to_string(),
                    pw_shell: CStr::from_ptr(pw.pw_shell).to_str()?.to_string(),
                };
                return Ok(pwr);
            }
        };
    }
}

#[cfg(test)]
mod tests {
    // TODO: Improve testing because of unsafe code
    use super::*;
    #[test]
    /// Asserts if the uid returned for the uname "root" is 0
    fn test_passwd_by_name() {
        let root = PasswdRecord::get_record_by_name("root").unwrap();
        assert_eq!(root.pw_uid, 0)
    }

    #[test]
    /// Asserts if the uname returned by the uid 0 is "root"
    fn test_passwd_by_uid() {
        let root = PasswdRecord::get_record_by_id(0).unwrap();
        assert_eq!(root.pw_name, "root")
    }

    #[test]
    /// Asserts if the uid returned for the uname "root" is 0
    fn test_gr_by_name() {
        let root = GroupRecord::get_record_by_name("root").unwrap();
        assert_eq!(root.gr_gid, 0)
    }

    #[test]
    /// Asserts if the uname returned by the uid 0 is "root"
    fn test_gr_by_gid() {
        let root = GroupRecord::get_record_by_id(0).unwrap();
        assert_eq!(root.gr_name, "root")
    }
}
