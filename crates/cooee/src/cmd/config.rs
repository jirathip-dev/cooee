use crate::cli::{Failed, Flag, Invocation, migrate};

pub(super) const USAGE: &str = "config migrate [--dry-run]";
pub(super) const FLAGS: &[Flag] = &[Flag::Switch("--dry-run")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        ["migrate"] => Ok(migrate(&inv.config()?, inv.on("--dry-run"))?),
        _ => Err(Failed::Usage),
    }
}
