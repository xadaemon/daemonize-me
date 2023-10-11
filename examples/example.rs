extern crate daemonize_me;

use std::any::Any;
use std::fs::File;
use std::process::exit;

pub use daemonize_me::Daemon;


fn post_fork_parent(ppid: i32, cpid: i32) -> ! {
    println!("Parent pid: {}, Child pid {}", ppid, cpid);
    println!("Parent will keep running after the child is forked, might even go do other tasks");
    println!("Or quit like so, bye :)");
    exit(0);
}

fn post_fork_child(ppid: i32, cpid: i32) {
    println!("Parent pid: {}, Child pid {}", ppid, cpid);
    println!("This hook is called in the child");
    // Child hook must return
    return
}

fn after_init(_: Option<&dyn Any>) {
    println!("Initialized the daemon!");
    return
}

fn main() {
    let stdout = File::create("info.log").unwrap();
    let stderr = File::create("err.log").unwrap();
    let daemon = Daemon::new()
        .pid_file("example.pid", Some(false))
        .umask(0o000)
        .work_dir(".")
        .stdout(stdout)
        .stderr(stderr)
        // Hooks are optional
        .setup_post_fork_parent_hook(post_fork_parent)
        .setup_post_fork_child_hook(post_fork_child)
        .setup_post_init_hook(after_init, None)
        // Start the daemon and calls the hooks
        .start();

    match daemon {
        Ok(_) => println!("Daemonized with success"),
        Err(e) => {
            eprintln!("Error, {}", e);
            exit(-1);
        },
    }

    for i in 0..=10000 {
        println!("{}", i);
    }
}
