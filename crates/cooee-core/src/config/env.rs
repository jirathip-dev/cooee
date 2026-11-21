use std::{env, ffi::OsString};

/// The environment variable `name` (#203: CI's arch-check bans `env::var` outside this module).
#[must_use]
pub fn env_var(name: &str) -> Option<OsString> {
    env::var_os(name)
}
