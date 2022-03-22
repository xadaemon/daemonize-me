# daemonize-me [![Rust](https://github.com/CardinalBytes/daemonize-me/workflows/Rust/badge.svg)](https://github.com/CardinalBytes/daemonize-me/actions) [![Crates.io](https://img.shields.io/crates/v/daemonize-me)](https://crates.io/crates/daemonize-me) [![Crates.io](https://img.shields.io/crates/d/daemonize-me)](https://crates.io/crates/daemonize-me) [![Crates.io](https://img.shields.io/crates/l/daemonize-me)](https://github.com/CardinalBytes/daemonize-me/blob/master/LICENSE)
Rust library to ease the task of creating daemons, I have drawn heavy inspiration from [Daemonize by knsd](https://github.com/knsd/daemonize).

# Current release
1.0-LTS track: 1.0.1

# Basic usage
Add it to your cargo.toml this will add the whole 1.0.x (LTS) series as compatible as per semver
```
daemonize-me = "1.0"
```
Example:
```rust
extern crate daemonize_me;
use daemonize_me::{Daemon, Group, User};
use std::convert::TryFrom;
use std::fs::File;

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
}
```

## OS support
I will try to keep support for linux, freebsd and macos

| os | tier |
| --- | --- |
| linux | tier 1 |
| freebsd, netbsd | tier 2 |
| macos, unix, *nix | tier 3 |
| Anything non unix | not supported |

For tier 1 any code that breaks the tests and or ci/cd is blocking for a release, 
tier 2 compilation errors are release blocking, tier 3 are supported on a best effort basis,
and build failure as well as test failures are not blocking.

note on custom/hobby OS support, if your os implements the syscalls used in lib.rs with behavior that is equivalent then this library is likely to work but it's even less of a guarantee.

## Supported Versions

Odd numbered minor versions receive patches and fixes only until the next odd numbered release, even numbered releases are considered LTS and will get fixes until the next even release happens - about 6 months.

# License

Licensed under either of

* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* BSD 3 Clause License

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.
