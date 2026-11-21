use std::io;

use cooee_core::Config;

use super::{Setup, step};

impl Setup {
    pub(super) fn themes(&self, config: &Config) -> Result<(), String> {
        let e = |e: io::Error| e.to_string();
        let t = match cooee_server::Themes::new(config) {
            Err(_) if self.dry_run => {
                cooee_core::say!(
                    "themes:\n  would install farm, scene and terminal and ask which other themes to install"
                );
                return Ok(());
            }
            t => t.map_err(e)?,
        };
        let have: Vec<String> = t.list().map_err(e)?.into_iter().map(|p| p.name).collect();
        let first: Vec<&str> = t
            .first_party()
            .into_iter()
            .filter(|n| !have.iter().any(|h| h == n))
            .collect();
        if step("themes", first.is_empty()) {
            return Ok(());
        }
        let (farm, rest): (Vec<&str>, Vec<&str>) = first
            .into_iter()
            .partition(|n| *n == "farm" || cooee_server::STYLES.iter().any(|(s, _)| s == n));
        let all = rest.join(" ");
        let want = if rest.is_empty() {
            String::new()
        } else {
            self.ask(&format!("themes to install, or none [{all}]:"), &all)
        };
        let want: Vec<&str> = want
            .split([' ', ','])
            .filter(|n| !n.is_empty() && *n != "none")
            .collect();
        if let Some(name) = want.iter().find(|n| !rest.contains(n)) {
            return Err(format!("theme {name}: not one of {all}"));
        }
        for name in farm.into_iter().chain(want) {
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
