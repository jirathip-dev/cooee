//! `cooee layout check <layout.json>...`: the layout gate.

use crate::{
    check,
    cli::{Failed, Flag, Invocation},
};

pub(super) const USAGE: &str = "layout check <layout.json>...";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        ["check", f @ ..] => Ok(check::run("layout", f)?),
        _ => Err(Failed::Usage),
    }
}
