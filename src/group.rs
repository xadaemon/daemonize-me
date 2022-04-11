pub use std::convert::TryFrom;

use crate::{DaemonError, Result};
use crate::ffi::GroupRecord;

/// Expects: either the group name or a gid
/// if the name is provided it will be resolved to an id
#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone)]
pub struct Group {
    pub id: u32,
    pub name: String
}

impl<'uname> TryFrom<&'uname str> for Group {
    type Error = DaemonError;

    fn try_from(gname: &'uname str) -> Result<Group> {
        match GroupRecord::lookup_record_by_name(gname) {
            Ok(record) => Ok(Group {
                id: record.gr_gid,
                name: record.gr_name
            }),
            Err(_) => Err(DaemonError::InvalidGroup),
        }
    }
}

impl TryFrom<&String> for Group {
    type Error = DaemonError;

    fn try_from(gname: &String) -> Result<Group> {
        match GroupRecord::lookup_record_by_name(gname.as_str()) {
            Ok(record) => Ok(Group {
                id: record.gr_gid,
                name: record.gr_name
            }),
            Err(_) => Err(DaemonError::InvalidGroup),
        }
    }
}

impl TryFrom<u32> for Group {
    type Error = DaemonError;

    fn try_from(gid: u32) -> Result<Group> {
        let record = GroupRecord::lookup_record_by_id(gid)?;
        Ok(Group {
            id: record.gr_gid,
            name: record.gr_name
        })
    }
}

