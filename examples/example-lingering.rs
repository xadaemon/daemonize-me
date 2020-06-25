extern crate daemonize_me;
use daemonize_me::Daemon;
use nix::unistd::{getgid, getuid};
use std::fs::File;

fn main() {
    let stdout = File::create("info-linger.log").unwrap();
    let stderr = File::create("err-linger.log").unwrap();
    let uid = getuid();
    let gid = getgid();
    println!("sid: {}, pid: {}", uid, gid);
    let daemon = Daemon::new()
        .pid_file("example.pid", Some(false))
        .user("daemon")
        .group("daemon")
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
    loop {}
}
