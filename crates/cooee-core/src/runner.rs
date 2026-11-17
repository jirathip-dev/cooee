//! The one place cooee starts a process (#202): CI's arch-check bans `Command::new` in every other src file.

use std::{ffi::OsStr, process::Command};

/// A `Command` that runs `prog`; every crate's processes start from here.
pub fn command(prog: impl AsRef<OsStr>) -> Command {
    Command::new(prog)
}
