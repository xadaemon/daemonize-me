use std::convert::TryFrom;

use crate::{DaemonError, Result};
use crate::ffi::GroupRecord;

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

