# daemonize-me [![Rust](https://github.com/CardinalBytes/daemonize-me/workflows/Rust/badge.svg)](https://github.com/CardinalBytes/daemonize-me/actions) [![Crates.io](https://img.shields.io/crates/v/daemonize-me)](https://crates.io/crates/daemonize-me) [![Crates.io](https://img.shields.io/crates/d/daemonize-me)](https://crates.io/crates/daemonize-me) [![Crates.io](https://img.shields.io/crates/l/daemonize-me)](https://github.com/CardinalBytes/daemonize-me/blob/master/LICENSE)
Rust library to ease the task of creating daemons, I have drawn heavy inspiration from [Daemonize by knsd](https://github.com/knsd/daemonize).

# 2.0 development track
I thank you for your interest in the development track , but beware it comes with caveats
like any development track bugs are expected and breaking changes are allowed happen.

**DO NOT USE IN PRODUCTION, USE 1.0.0 FOR ANYTHING SERIOUS**

# 2.0 timeline:
This is an expected timeline for release of the 2.0 track, all months are 2022
* jan - feb: development work and stabilization
* mar: freeze and release towards the end of the month

# Basic usage
Add it to your cargo.toml this will add the whole 1.0.x series as compatible as per semver
```
daemonize-me = "{check the version}"
```


## OS support
Support is given for linux, freebsd and macos

| os | tier |
| --- | --- |
| linux | tier 1 |
| freebsd, netbsd | tier 2 |
| macos, unix, *nix | tier 3 |
| Anything non unix | not supported |

For tier 1 any code that breaks the tests and or ci/cd is blocking for a release, 
tier 2 compilation errors are release blocking, tier 3 are supported on a best effort basis,
and build failure as well as test failures are not blocking.

Note on custom/hobby OS support: if your os implements the syscalls used in lib.rs with behavior that is equivalent then this library is likely to work but it's even less of a guarantee.

## Supported Versions

In the development track every version is unsupported and won't receive backport fixes.

# License

Licensed under either of

* Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
* BSD 3 Clause License

# Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be dual licensed as above, without any additional terms or conditions.
