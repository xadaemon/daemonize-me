#![deny(warnings)]
#![deny(clippy::complexity)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::trivially_copy_pass_by_ref)]


extern crate libc;
extern crate nix;

use thiserror::Error;

mod stdio;

mod group;
mod user;
mod daemon;
mod ffi;

pub use crate::group::Group;
pub use crate::user::User;
pub use crate::daemon::Daemon;


#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("This feature is unavailable, or not implemented for your target os")]
    UnsupportedOnOS,
    #[error("Unable to fork")]
    Fork,
    #[error("Failed to chdir")]
    ChDir,
    #[error("Failed to open dev null")]
    OpenDevNull,
    #[error("Failed to close the file pointer of a stdio stream")]
    CloseFp,
    #[error("Invalid or nonexistent user")]
    InvalidUser,
    #[error("Invalid or nonexistent group")]
    InvalidGroup,
    #[error("Either group or user was specified but not the other")]
    InvalidUserGroupPair,
    #[error("The specified cstr is invalid")]
    InvalidCstr,
    #[error("Failed to execute initgroups")]
    InitGroups,
    #[error("Failed to set uid")]
    SetUid,
    #[error("Failed to set gid")]
    SetGid,
    #[error("Failed to chown the pid file")]
    ChownPid,
    #[error("Failed to create the pid file")]
    OpenPid,
    #[error("Failed to write to the pid file")]
    WritePid,
    #[error("Failed to redirect the standard streams")]
    RedirectStream,
    #[error("Umask bits are invalid")]
    InvalidUmaskBits,
    #[error("Failed to set sid")]
    SetSid,
    #[error("Failed to get groups record")]
    GetGrRecord,
    #[error("Failed to get passwd record")]
    GetPasswdRecord,
    #[error("Failed to set proc name")]
    SetProcName,
    #[error("Failed to set proc name")]
    InvalidProcName,
}

pub type Result<T> = std::result::Result<T, DaemonError>;
