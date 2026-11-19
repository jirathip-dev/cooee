use std::{io, path::Path};

use cooee_core::{Config, Shed, chief};

use super::{Setup, step};

const FLAGS: &str = "add --chief-agent <id> --chief-machine <name|here>";

impl Setup {
    pub(super) fn chief_flags(&self, path: &Path) -> Result<(), String> {
        let dir = path.parent().unwrap_or(path);
        let picked = self.chief_agent.is_some() && self.chief_machine.is_some();
        if !self.yes || picked || chief::find(Some(dir), dir).is_some() {
            return Ok(());
        }
        let no = "there is no Chief yet and --yes can't pick for you";
        Err(format!("chief: {no}: {FLAGS}\nnothing was changed"))
    }

    pub(super) fn chief(&self, config: &Config, path: &Path) -> Result<(), String> {
        let (dir, data) = (
            path.parent().unwrap_or(path),
            config.data_dir().ok_or("no data folder")?,
        );
        if step("chief", chief::find(Some(dir), &data).is_some()) {
            return Ok(());
        }
        cooee_core::say!(
            "  Chief is cooee's own persona: it sets up personas and routines for you."
        );
        let shed = Shed::new(config.shed.clone(), config.shed_dir.clone());
        let agents = shed.agents().unwrap_or_default().into_iter().map(|a| a.id);
        let linked = shed.links().unwrap_or_default().into_keys();
        let machines: Vec<String> = std::iter::once("here".to_owned()).chain(linked).collect();
        let agent = self.pick(
            "its agent",
            &agents.collect::<Vec<_>>(),
            self.chief_agent.as_deref(),
        )?;
        let machine = self.pick("its machine", &machines, self.chief_machine.as_deref())?;
        self.write(&dir.join("agents").join(chief::ID), "", || {
            let made = chief::create(dir, &data, &agent, &machine);
            made.map(drop).map_err(|e| e.to_string())
        })
    }

    fn pick(&self, what: &str, from: &[String], given: Option<&str>) -> Result<String, String> {
        if let Some(g) = given {
            let known = self.dry_run || from.iter().any(|f| f == g);
            let list = from.join(", ");
            return known
                .then(|| g.to_owned())
                .ok_or(format!("chief: {what} {g} isn't one of: {list}"));
        }
        let menu: Vec<String> = from
            .iter()
            .zip(1..)
            .map(|(f, n)| format!("{n} {f}"))
            .collect();
        loop {
            cooee_core::say!("  {what}:  {}", menu.join("   "));
            let mut line = String::new();
            if io::stdin()
                .read_line(&mut line)
                .map_err(|e| e.to_string())?
                == 0
            {
                return Err(format!("chief: no answer for {what}: {FLAGS}"));
            }
            let (line, at) = (line.trim(), line.trim().parse::<usize>().ok());
            let at = at.and_then(|n| from.get(n.checked_sub(1)?));
            if let Some(f) = at.or_else(|| from.iter().find(|f| *f == line)) {
                return Ok(f.clone());
            }
        }
    }
}
