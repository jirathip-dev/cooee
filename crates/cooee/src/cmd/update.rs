use crate::{
    cli::{Failed, Flag, Invocation},
    update,
};

pub(super) const USAGE: &str = "update [--check]";
pub(super) const FLAGS: &[Flag] = &[Flag::Switch("--check")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    if !inv.rest().is_empty() {
        return Err(Failed::Usage);
    }
    let config = inv.config()?;
    if inv.on("--check") {
        Ok(update::check(&config)?)
    } else {
        Ok(update::run(&config)?)
    }
}
