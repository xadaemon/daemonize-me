# daemonize-me ![Rust](https://github.com/CardinalBytes/daemonize-me/workflows/Rust/badge.svg)
Rust library to ease the task of creating daemons, I have drawn heavy inspiration from [Daemonize by knsd](https://github.com/knsd/daemonize).

I just reached a mature enough point to call this code usable as it is now doing what it should (it still needs better testing however).
This being said, I'm electing to release the library in a first version as is and keep improving it.

# Basic usage
Example:
```rust
extern crate daemonize_me;
use daemonize_me::Daemon;
use std::fs::File;

fn main() {
    let stdout = File::create("info.log").unwrap();
    let stderr = File::create("err.log").unwrap();
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
}
```