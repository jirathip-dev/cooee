use cooee_core::{Config, Shed, provider};

use super::{Setup, step};

impl Setup {
    pub(super) fn agents(&self, config: &Config) -> Result<(), String> {
        let have: Vec<String> = Shed::new(config.shed.clone(), config.shed_dir.clone())
            .agents()
            .map(|a| a.into_iter().map(|a| a.id).collect())
            .unwrap_or_default();
        let mut specs = provider::defaults().into_iter();
        let first = specs.next().filter(|s| !have.contains(&s.agent));
        let mut want: Vec<String> = first.map(|s| s.agent).into_iter().collect();
        for s in specs.filter(|s| !have.contains(&s.agent)) {
            if self
                .ask(&format!("also {}? [y/N]", s.name), "n")
                .starts_with(['y', 'Y'])
            {
                want.push(s.agent);
            }
        }
        step("agents", want.is_empty());
        let shed = config.shed.display().to_string();
        let dir = config.shed_dir.as_ref().map(|d| d.display().to_string());
        for id in want {
            let mut argv = vec![shed.as_str()];
            if let Some(dir) = &dir {
                argv.extend(["--dir", dir]);
            }
            argv.extend(["agents", "add", &id]);
            self.exec(&argv)?;
        }
        Ok(())
    }
}
