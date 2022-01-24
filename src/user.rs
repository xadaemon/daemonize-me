use crate::ffi::PasswdRecord;
use crate::Result;
use nix::errno::Errno;
#[cfg(not(target_os = "macos"))]
use std::convert::TryFrom;

#[derive(Default, Clone, Copy)]
pub struct UserInfo {
    pub user: u32,
    pub group: u32,
}

impl UserInfo {
    pub fn is_some_none(&self) -> bool {
        self.user == 0 || self.group == 0
    }
    pub fn is_some(&self) -> bool {
        self.user != 0 && self.group != 0
    }
    pub fn get_user(&self) -> u32 {
        self.user
    }
    pub fn get_group(&self) -> u32 {
        self.group
    }
    pub fn new() -> Self {
        Self::default()
    }
}

impl<'uname> TryFrom<&'uname str> for UserInfo {
    type Error = Errno;

    fn try_from(uname: &'uname str) -> Result<UserInfo> {
        let record = PasswdRecord::get_record_by_name(uname)?;
        Ok(UserInfo {
            user: record.pw_uid,
            group: record.pw_gid,
        })
    }
}

impl TryFrom<String> for UserInfo {
    type Error = Errno;

    fn try_from(uname: String) -> Result<UserInfo> {
        let record = PasswdRecord::get_record_by_name(uname.as_str())?;
        Ok(UserInfo {
            user: record.pw_uid,
            group: record.pw_gid,
        })
    }
}

impl TryFrom<(u32, u32)> for UserInfo {
    type Error = Errno;

    fn try_from(pair_ids: (u32, u32)) -> Result<UserInfo> {
        Ok(UserInfo {
            user: pair_ids.0,
            group: pair_ids.1,
        })
    }
}
