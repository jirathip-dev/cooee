//! `cooee accounts`: the provider accounts and their folders.

use std::process::ExitCode;

use crate::{
    accounts,
    cli::{Failed, Flag, Invocation},
};

pub(super) const USAGE: &str = "accounts ls [--json] | accounts env <provider> <name> [--require-signed-in] | accounts add <provider> <name> --folder <path> | accounts rm <provider> <name>";
pub(super) const FLAGS: &[Flag] = &[Flag::Switch("--json"), Flag::Switch("--require-signed-in")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    let (json, require) = (inv.on("--json"), inv.on("--require-signed-in"));
    let code = accounts::run(&inv.config()?, &inv.rest(), json, require);
    if code == ExitCode::SUCCESS {
        Ok(())
    } else {
        Err(Failed::Exit(code))
    }
}
