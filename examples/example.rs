extern crate daemonize_me;

use std::convert::TryFrom;
use std::fs::File;

pub use daemonize_me::daemon::Daemon;
use daemonize_me::group::Group;
use daemonize_me::user::User;

fn post_fork_parent(ppid: i32, cpid: i32) -> ! {
    println!("Parent pid: {}, Child pid {}", ppid, cpid);
    println!("Parent will keep running after the child is forked, might even go do other tasks");
    loop {
        // keep parent open
    }
}

fn post_fork_child(ppid: i32, cpid: i32) {
    println!("Parent pid: {}, Child pid {}", ppid, cpid);
    println!("This hook is called in the child");
    // Child hook must return
    return;
}

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
        .setup_post_fork_parent_hook(post_fork_parent)
        .setup_post_fork_child_hook(post_fork_child)
        .start();

    match daemon {
        Ok(_) => println!("Daemonized with success"),
        Err(e) => eprintln!("Error, {}", e),
    }

    for i in 0..=10000 {
        println!("{}", i);
    }
}
