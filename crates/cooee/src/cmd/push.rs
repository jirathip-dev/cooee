use crate::{
    cli::{Failed, Flag, Invocation},
    keychain,
};

pub(super) const USAGE: &str = "push migrate";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        ["migrate"] => Ok(keychain::command(&inv.config()?)?),
        _ => Err(Failed::Usage),
    }
}
