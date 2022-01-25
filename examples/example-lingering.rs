extern crate daemonize_me;
pub use daemonize_me::daemon::Daemon;
use nix::unistd::{getgid, getuid};
use std::convert::TryFrom;
use std::fs::File;
use daemonize_me::group::Group;
use daemonize_me::user::User;

fn main() {
    let stdout = File::create("info.log").unwrap();
    let stderr = File::create("err.log").unwrap();
    let daemon = Daemon::new()
        .pid_file("example.pid", Some(false))
        .user(User::try_from("daemon").unwrap())
        .group(Group::try_from("daemon").unwrap())
        .umask(0o000)
        .work_dir(".")
        .stdout(stdout)
        .stderr(stderr)
        .start();

    match daemon {
        Ok(_) => println!("Daemonized with success"),
        Err(e) => eprintln!("Error, {}", e),
    }

    loop {
        // You wil have to kill this process yourself
    }
}
