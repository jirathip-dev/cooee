use crate::{
    cli::{Failed, Flag, Invocation},
    setup,
};

pub(super) const USAGE: &str =
    "setup [--port <p>] [--dry-run] [--yes] [--chief-agent <id>] [--chief-machine <name|here>]";
pub(super) const FLAGS: &[Flag] = &[
    Flag::Value("--port"),
    Flag::Switch("--dry-run"),
    Flag::Switch("--yes"),
    Flag::Value("--chief-agent"),
    Flag::Value("--chief-machine"),
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
        chief_agent: inv.value("--chief-agent").map(str::to_owned),
        chief_machine: inv.value("--chief-machine").map(str::to_owned),
    };
    Ok(setup.run(path)?)
}
