//! `cooee machine check <m> [--yes]`: one machine's setup (#150).

use crate::{
    cli::{Failed, Flag, Invocation},
    machine,
};

pub(super) const USAGE: &str = "machine check <m> [--yes]";
pub(super) const FLAGS: &[Flag] = &[Flag::Switch("--yes")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        ["check", m] => Ok(machine::run(&inv.config()?, m, inv.on("--yes"))?),
        _ => Err(Failed::Usage),
    }
}
