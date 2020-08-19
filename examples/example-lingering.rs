extern crate daemonize_me;
use daemonize_me::{Daemon, Group, User};
use nix::unistd::{getgid, getuid};
use std::convert::TryFrom;
use std::fs::File;

fn main() {
    let stdout = File::create("info.log").unwrap();
    let stderr = File::create("err.log").unwrap();
    let uid = getuid();
    let gid = getgid();
    println!("sid: {}, pid: {}", uid, gid);
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
    // use infinite loop to keep process open for inspection
    println!("Hello from the daemon");
    loop {}
}
