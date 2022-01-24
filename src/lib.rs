pub mod daemon;
pub mod ffi;
pub mod process;
pub mod user;

use nix::errno::Errno;
pub type Result<T> = std::result::Result<T, Errno>;

#[cfg(test)]
mod tests {
    // TODO: Improve testing coverage

    use std::convert::TryFrom;

    use crate::{daemon::Daemon, user::UserInfo};
    #[test]
    fn test_uname_to_uid_resolution() {
        let daemon = Daemon::new().user_info(UserInfo::try_from("root").unwrap());
        assert!(daemon.user().unwrap().is_some());
        let uid = daemon.user().unwrap().user;
        assert_eq!(uid, 0)
    }
}
