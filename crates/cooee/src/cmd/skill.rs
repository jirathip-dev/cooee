use cooee_core::{Config, agents, library};

use crate::cli::{Failed, Flag, Invocation};

pub(super) const USAGE: &str = "skill add <persona> <name> | skill list";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    Ok(skill(&inv.config()?, &inv.rest(), inv.usage())?)
}

fn skill(config: &Config, args: &[&str], usage: &str) -> Result<(), String> {
    let e = |e: std::io::Error| e.to_string();
    let sources = library::index(config.skill_index.as_deref()).map_err(e)?;
    match args {
        ["add", persona, name] => {
            let dir = config.dir.as_deref().ok_or("no config folder")?;
            let data = config
                .data_dir()
                .ok_or("no data folder: set `data` or HOME")?;
            let home = agents::home(dir, persona).map_err(e)?;
            let s = library::add(&home, &sources, name, &data.join("skills-fetch")).map_err(e)?;
            cooee_core::say!("added {} to {persona}: {}", s.name, s.description);
        }
        ["list"] => {
            let starter = library::starter();
            for src in &sources {
                for n in &src.skills {
                    let about = starter.iter().find(|s| &s.name == n);
                    let about = about.map_or(src.url.as_str(), |s| s.description.as_str());
                    cooee_core::say!("{n}  {about}");
                }
            }
        }
        _ => return Err(usage.to_owned()),
    }
    Ok(())
}
