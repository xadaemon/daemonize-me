use std::convert::TryFrom;

use crate::{DaemonError, Result};
use crate::ffi::PasswdRecord;

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
