use crate::{
    cli::{Failed, Flag, Invocation},
    doctor::doctor,
};

pub(super) const USAGE: &str = "doctor";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [] => Ok(doctor(&inv.config()?)?),
        _ => Err(Failed::Usage),
    }
}
