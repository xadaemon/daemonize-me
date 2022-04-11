use std::convert::TryFrom;

use crate::{DaemonError, Result};
use crate::ffi::PasswdRecord;

/// Expects: either the username or the uid
/// if the name is provided it will be resolved to an id
#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
}

impl<'uname> TryFrom<&'uname str> for User {
    type Error = DaemonError;

    fn try_from(uname: &'uname str) -> Result<User> {
        match PasswdRecord::lookup_record_by_name(uname) {
            Ok(record) => Ok(User { id: record.pw_uid, name: record.pw_name }),
            Err(_) => Err(DaemonError::InvalidUser),
        }
    }
}

impl TryFrom<&String> for User {
    type Error = DaemonError;

    fn try_from(uname: &String) -> Result<User> {
        match PasswdRecord::lookup_record_by_name(uname.as_str()) {
            Ok(record) => Ok(User { id: record.pw_uid, name: record.pw_name }),
            Err(_) => Err(DaemonError::InvalidUser),
        }
    }
}

impl TryFrom<u32> for User {
    type Error = DaemonError;

    fn try_from(uid: u32) -> Result<User> {
        let record = PasswdRecord::lookup_record_by_id(uid)?;
        Ok(User {
            id: record.pw_uid,
            name: record.pw_name,
        })
    }
}
