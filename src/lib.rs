#![deny(warnings)]
#![deny(clippy::complexity)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::needless_pass_by_value)]
#![deny(clippy::trivially_copy_pass_by_ref)]

extern crate libc;
extern crate nix;

mod stdio;

mod daemon;
mod ffi;
mod group;
mod user;

mod errors;

pub use crate::daemon::{Daemon, DaemonStatus};
pub use crate::errors::{DaemonError, Result};
pub use crate::group::Group;
pub use crate::user::User;
