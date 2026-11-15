//! `cooee theme add|remove|list|check` (docs/themes.md).

use std::path::Path;

use cooee_core::Config;

use crate::cli::{Failed, Flag, Invocation, confirm};

pub(super) const USAGE: &str = "theme add <name | git-url | path> [--rev <r>] [--yes] | theme remove <name> | theme list | theme check <path>...";
pub(super) const FLAGS: &[Flag] = &[Flag::Value("--rev"), Flag::Switch("--yes")];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    let (rev, yes) = (inv.value("--rev"), inv.on("--yes"));
    Ok(theme(&inv.config()?, &inv.rest(), rev, yes, inv.usage())?)
}

fn theme(
    config: &Config,
    args: &[&str],
    rev: Option<&str>,
    yes: bool,
    usage: &str,
) -> Result<(), String> {
    if let ["check", paths @ ..] = args {
        let mut failed = 0;
        for p in paths {
            match cooee_server::check(Path::new(p)) {
                Ok(()) => println!("ok   {p}"),
                Err(fails) => {
                    failed += 1;
                    println!("FAIL {p}\n     {}", fails.join("\n     "));
                }
            }
        }
        return match (paths.len(), failed) {
            (0, _) => Err(usage.to_owned()),
            (_, 0) => Ok(()),
            _ => Err(format!("{failed} theme(s) fail the theme gate")),
        };
    }
    let t = cooee_server::Themes::new(config).map_err(|e| e.to_string())?;
    let e = |e: std::io::Error| e.to_string();
    match args {
        ["add", source] => {
            let pack = t.plan(source, rev).map_err(e)?;
            if pack.listed {
                println!("{}", pack.text);
            } else {
                confirm(&pack.text, yes)?;
            }
            t.add(&pack).map_err(e)?;
            println!("added {}: pick it in Settings › look", pack.name);
        }
        ["remove", name] => {
            t.remove(name).map_err(e)?;
            println!("removed {name}");
        }
        ["list"] => {
            let packs = t.list().map_err(e)?;
            if packs.is_empty() {
                println!(
                    "no themes installed: plain is built in; `cooee theme add <name>` adds one"
                );
            }
            for p in packs {
                let commit = p.commit.get(..7).unwrap_or("-");
                println!("{}  {}  {commit}", p.name, p.source);
            }
        }
        _ => return Err(usage.to_owned()),
    }
    Ok(())
}
