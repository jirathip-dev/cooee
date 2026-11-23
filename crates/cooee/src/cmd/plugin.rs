use cooee_core::Config;

use crate::{
    check,
    cli::{Failed, Flag, Invocation, confirm},
};

pub(super) const USAGE: &str = "plugin add <name | git-url> [--rev <r>] [--yes] | plugin update [<name>] [--yes] | plugin remove <name> [--purge] | plugin list | plugin check <answer.json|cooee-plugin.toml>...";
pub(super) const FLAGS: &[Flag] = &[
    Flag::Value("--rev"),
    Flag::Switch("--yes"),
    Flag::Switch("--purge"),
];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    let (rev, yes, purge) = (inv.value("--rev"), inv.on("--yes"), inv.on("--purge"));
    match inv.rest().as_slice() {
        ["check", f @ ..] => Ok(check::run("plugin", f)?),
        rest => Ok(plugin(&inv.config()?, rest, rev, yes, purge, inv.usage())?),
    }
}

fn update(m: &cooee_server::Manager, names: &[&str], yes: bool) -> Result<(), String> {
    let e = |e: std::io::Error| e.to_string();
    let all: Vec<String> = match names {
        [] => m
            .installed()
            .map_err(e)?
            .into_iter()
            .map(|l| l.name)
            .collect(),
        _ => names.iter().map(|n| (*n).to_owned()).collect(),
    };
    for name in all {
        let plan = m.plan_update(&name).map_err(e)?;
        if m.installed()
            .map_err(e)?
            .iter()
            .any(|l| l.name == name && l.commit == plan.commit)
        {
            cooee_core::say!("{name}: up to date");
            continue;
        }
        confirm(&plan.text, yes)?;
        m.update(&plan)
            .map_err(|x| format!("{name}: {x}; the old build keeps running"))?;
        cooee_core::say!("updated {name} to {}", &plan.commit[..7]);
    }
    Ok(())
}

fn plugin(
    config: &Config,
    args: &[&str],
    rev: Option<&str>,
    yes: bool,
    purge: bool,
    usage: &str,
) -> Result<(), String> {
    let m = cooee_server::Manager::new(config).map_err(|e| e.to_string())?;
    let e = |e: std::io::Error| e.to_string();
    match args {
        ["add", source] => {
            let plan = m.plan(source, rev).map_err(e)?;
            confirm(&plan.text, yes)?;
            m.add(&plan).map_err(e)?;
            cooee_core::say!("added {}: cooee serve runs it", plan.manifest.name);
        }
        ["update", names @ ..] => update(&m, names, yes)?,
        ["remove", name] => {
            m.remove(name, purge).map_err(e)?;
            let kept = if purge {
                ""
            } else {
                " (its checkout stays: --purge removes it)"
            };
            cooee_core::say!("removed {name}{kept}");
        }
        ["list"] => {
            for r in m.list().map_err(e)? {
                let l = &r.locked;
                let source = if l.listed { "index" } else { "url" };
                cooee_core::say!(
                    "{}  {}  {}  {source}  {}  accepts {}",
                    l.name,
                    l.version,
                    &l.commit[..7],
                    r.state,
                    r.accepts
                );
            }
        }
        _ => return Err(usage.to_owned()),
    }
    Ok(())
}
