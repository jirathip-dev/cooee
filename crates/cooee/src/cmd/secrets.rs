//! `cooee secrets`: checks the secrets in the keychain.

use crate::{
    cli::{Failed, Flag, Invocation},
    keychain,
};

pub(super) const USAGE: &str = "secrets";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [] => Ok(keychain::check(&inv.config()?)?),
        _ => Err(Failed::Usage),
    }
}
