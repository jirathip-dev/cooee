//! `cooee setup`: everything from shed to a pairing code.

use crate::{
    cli::{Failed, Flag, Invocation},
    setup,
};

pub(super) const USAGE: &str = "setup [--port <p>] [--dry-run] [--yes]";
pub(super) const FLAGS: &[Flag] = &[
    Flag::Value("--port"),
    Flag::Switch("--dry-run"),
    Flag::Switch("--yes"),
];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    if !inv.rest().is_empty() {
        return Err(Failed::Usage);
    }
    let Ok(port) = inv.value("--port").map(str::parse).transpose() else {
        return Err(Failed::Usage);
    };
    let path = inv.path().ok_or("no config path: set HOME".to_owned())?;
    let setup = setup::Setup {
        instance: inv.instance().to_owned(),
        port,
        explicit: inv.explicit(),
        dry_run: inv.on("--dry-run"),
        yes: inv.on("--yes"),
    };
    Ok(setup.run(path)?)
}
