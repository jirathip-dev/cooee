use std::io;

use cooee_core::Config;

use super::{Setup, step};

impl Setup {
    pub(super) fn themes(&self, config: &Config) -> Result<(), String> {
        let e = |e: io::Error| e.to_string();
        let t = match cooee_server::Themes::new(config) {
            Err(_) if self.dry_run => {
                cooee_core::say!("themes:\n  would install scene and terminal");
                return Ok(());
            }
            t => t.map_err(e)?,
        };
        let have: Vec<String> = t.list().map_err(e)?.into_iter().map(|p| p.name).collect();
        let styles: Vec<&str> = cooee_server::STYLES
            .iter()
            .map(|(n, _)| *n)
            .filter(|n| !have.iter().any(|h| h == n))
            .collect();
        if step("themes", styles.is_empty()) {
            return Ok(());
        }
        for name in styles {
            if self.dry_run {
                cooee_core::say!("  would install theme {name}");
                continue;
            }
            match t.plan(name, None).and_then(|p| t.add(&p)) {
                Ok(()) => cooee_core::say!("  installed theme {name}"),
                Err(why) => {
                    cooee_core::say!("  theme {name}: {why}\n  later: cooee theme add {name}");
                }
            }
        }
        Ok(())
    }
}
