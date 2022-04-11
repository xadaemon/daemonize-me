# daemonize-me [![Rust](https://github.com/CardinalBytes/daemonize-me/workflows/Rust/badge.svg)](https://github.com/CardinalBytes/daemonize-me/actions) [![Crates.io](https://img.shields.io/crates/v/daemonize-me)](https://crates.io/crates/daemonize-me) [![Crates.io](https://img.shields.io/crates/d/daemonize-me)](https://crates.io/crates/daemonize-me) [![Crates.io](https://img.shields.io/crates/l/daemonize-me)](https://github.com/CardinalBytes/daemonize-me/blob/master/LICENSE)
Rust library to ease the task of creating daemons, I have drawn heavy inspiration from [Daemonize by knsd](https://github.com/knsd/daemonize).

# Current releases and EOL table
| track    | version | EOL     |
|----------|---------|---------|
| 2.0      | 2.0.0   | TBA     |
| 1.0(LTS) | 1.0.2   | 2022-10 |


# Basic usage
Add it to your cargo.toml this will add the whole 2.0.x series as compatible as per semver
```toml
daemonize-me = "2.0"
```
Then look at [example.rs](examples/example.rs)


## OS support
I will try to keep support for linux, freebsd and macos

| os                  | tier          |
|---------------------|---------------|
| linux               | tier 1        |
| freebsd, openbsd    | tier 2        |
| macos, netbsd, unix | tier 3        |
| Anything non unix   | not supported |

For tier 1 any code that breaks the tests and or ci/cd is blocking for a release,
tier 2 compilation errors are release blocking, tier 3 are supported on a best effort basis,
and build failure as well as test failures are not blocking.

note on custom/hobby OS support, if your os implements the syscalls used in lib.rs with behavior that is equivalent then this library is likely to work but it's even less of a guarantee.

# License

Licensed under either of

* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* BSD 3 Clause License

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.