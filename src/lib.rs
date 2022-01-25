#![deny(warnings)]
#![deny(clippy::complexity)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::trivially_copy_pass_by_ref)]


extern crate libc;
extern crate nix;

use snafu::Snafu;

pub mod ffi;
pub mod user;
pub mod group;
pub mod stdio;
pub mod daemon;

#[derive(Debug, Snafu)]
pub enum DaemonError {
    /// This feature is unavailable or not implemented to your target os
    UnsupportedOnOS,
    /// Unable to fork
    Fork,
    /// Failed to chdir
    ChDir,
    /// Failed to open dev null
    OpenDevNull,
    /// Failed to close the file pointer of a stdio stream
    CloseFp,
    /// Invalid or nonexistent user
    InvalidUser,
    /// Invalid or nonexistent group
    InvalidGroup,
    /// Either group or user was specified but no the other
    InvalidUserGroupPair,
    /// The specified cstr is invalid
    InvalidCstr,
    /// Failed to execute initgroups
    InitGroups,
    /// Failed to set uid
    SetUid,
    /// Failed to set gid
    SetGid,
    /// Failed to chown the pid file
    ChownPid,
    /// Failed to create the pid file
    OpenPid,
    /// Failed to write to the pid file
    WritePid,
    /// Failed to redirect the standard streams
    RedirectStream,
    /// Umask bits are invalid
    InvalidUmaskBits,
    /// Failed to set sid
    SetSid,
    /// Failed to get groups record
    GetGrRecord,
    /// Failed to get passwd record
    GetPasswdRecord,
    /// Failed to set proc name
    SetProcName,
    InvalidProcName,
    #[doc(hidden)]
    __Nonexhaustive,
}

pub type Result<T> = std::result::Result<T, DaemonError>;

#[cfg(test)]
mod tests {
    // TODO: Improve testing coverage
    extern crate nix;

    use super::*;

    #[test]
    fn test_uname_to_uid_resolution() {
        let daemon = Daemon::new().user(User::try_from("root").unwrap());
        assert!(daemon.user.is_some());
        let uid = match daemon.user.unwrap() {
            User::Id(id) => id,
        };
        assert_eq!(uid, 0)
    }
}
