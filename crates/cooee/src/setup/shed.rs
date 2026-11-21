use cooee_core::{SHED_MIN_PROTOCOL, Shed, ShedError, shed_fix};

use super::step;

pub(super) fn shed() -> Result<(), String> {
    match Shed::new("shed".into(), None).hello() {
        Ok(()) => {
            step(&format!("shed protocol {SHED_MIN_PROTOCOL}+"), true);
            Ok(())
        }
        Err(e @ ShedError::Version { .. }) => Err(format!("{e}\nnothing was changed")),
        Err(e) => Err(format!("{e}\nfix: {}\nnothing was changed", shed_fix())),
    }
}
