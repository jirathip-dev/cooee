use std::{
    fs, io,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::Path,
    process::Stdio,
    time::Duration,
};

use cooee_core::{Config, SHED_CARGO, SHED_LABEL, Shed, ShedError, provider};

use crate::{
    cli::load,
    devices::pair,
    doctor::doctor,
    service::{job, plist, refuse_app, tailnet_name, write_plist},
};

/// `cooee setup`'s flags.
pub struct Setup {
    pub instance: String,
    pub port: Option<u16>,
    pub explicit: bool,
    pub dry_run: bool,
    pub yes: bool,
}

pub(crate) fn ok(argv: &[&str]) -> bool {
    let Some((prog, rest)) = argv.split_first() else {
        return false;
    };
    cooee_core::runner::command(prog)
        .args(rest)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

pub(crate) fn output(argv: &[&str]) -> Option<String> {
    let (prog, rest) = argv.split_first()?;
    let out = cooee_core::runner::command(prog).args(rest).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

pub(crate) fn wait(f: impl Fn() -> bool) -> bool {
    (0..40).any(|_| {
        let done = f();
        if !done {
            cooee_core::clock::sleep(Duration::from_millis(250));
        }
        done
    })
}

impl Setup {
    fn exec(&self, argv: &[&str]) -> Result<(), String> {
        let line = argv.join(" ");
        if self.dry_run {
            cooee_core::say!("  would run: {line}");
            return Ok(());
        }
        cooee_core::say!("  $ {line}");
        let (prog, rest) = argv.split_first().ok_or("nothing to run")?;
        let status = cooee_core::runner::command(prog).args(rest).status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("`{line}` failed ({s})")),
            Err(e) => Err(format!("`{line}`: {e}")),
        }
    }

    fn write(
        &self,
        file: &Path,
        text: &str,
        f: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.dry_run {
            cooee_core::say!("  would write {}:\n{text}", file.display());
            return Ok(());
        }
        f()?;
        cooee_core::say!("  wrote {}", file.display());
        Ok(())
    }

    fn ask(&self, question: &str, default: &str) -> String {
        if self.yes {
            return default.to_owned();
        }
        cooee_core::say!("  {question}");
        let mut line = String::new();
        io::stdin().read_line(&mut line).ok();
        let line = line.trim();
        (if line.is_empty() { default } else { line }).to_owned()
    }

    /// Every step, in order, then the pairing code.
    pub fn run(&self, path: &Path) -> Result<(), String> {
        refuse_app()?;
        let uid = output(&["id", "-u"]).ok_or("`id -u` failed")?;
        let gui = format!("gui/{}", uid.trim());
        let updated = self.shed()?;
        self.shed_service(&gui, updated)?;
        let (config, changed) = self.config(path)?;
        self.themes(&config)?;
        self.agents(&config)?;
        self.service(&config, path, &gui, changed, "serve")?;
        self.service(&config, path, &gui, false, "update")?;
        self.tailnet(config.port)?;
        if self.dry_run {
            cooee_core::say!("would run: cooee doctor, then cooee pair");
            return Ok(());
        }
        cooee_core::say!("doctor:");
        doctor(&config)
            .map_err(|e| format!("doctor: {e}\nfix it, then run `cooee setup` again"))?;
        cooee_core::say!("\npair your phone:");
        pair(&config)
    }

    fn shed(&self) -> Result<bool, String> {
        let rev = SHED_CARGO[6];
        let done = output(&["cargo", "install", "--list"])
            .is_some_and(|l| l.contains(&format!("/shed?rev={rev}#")));
        if !step(&format!("shed {}", &rev[..7]), done) {
            self.exec(&SHED_CARGO)?;
        }
        Ok(!done)
    }

    fn shed_service(&self, gui: &str, updated: bool) -> Result<(), String> {
        let loaded = ok(&["launchctl", "list", SHED_LABEL]);
        let answers = || ok(&["shed", "ls"]);
        let serve = Shed::new("shed".into(), None);
        let mismatch = || {
            serve
                .hello()
                .err()
                .filter(|e| matches!(e, ShedError::Version { .. }))
        };
        let held = |e: ShedError| format!("{e}\ncooee's service was left on its running build");
        let other = mismatch();
        if step(
            &format!("shed service {SHED_LABEL}"),
            !updated && loaded && other.is_none() && answers(),
        ) {
            return Ok(());
        }
        match other {
            Some(e) if !loaded => return Err(held(e)),
            _ if loaded => {
                cooee_core::say!("  warn: restarting shed serve ends every running shed session");
                if self
                    .ask("restart it now? [Y/n]", "y")
                    .starts_with(['n', 'N'])
                {
                    return Err("shed serve was not restarted, so cooee wasn't either".to_owned());
                }
            }
            _ => {}
        }
        self.exec(&["shed", "install"])?;
        if loaded {
            self.exec(&[
                "launchctl",
                "kickstart",
                "-k",
                &format!("{gui}/{SHED_LABEL}"),
            ])?;
        } else {
            let home = cooee_core::env_var("HOME").ok_or("no HOME")?;
            let file = Path::new(&home).join(format!("Library/LaunchAgents/{SHED_LABEL}.plist"));
            self.exec(&["launchctl", "bootstrap", gui, &file.display().to_string()])?;
        }
        if self.dry_run {
            return Ok(());
        }
        if !wait(answers) {
            return Err("shed doesn't answer `shed ls`; see ~/Library/Logs/shed.log".to_owned());
        }
        if !wait(|| mismatch().is_none()) {
            return mismatch().map_or(Ok(()), |e| Err(held(e)));
        }
        Ok(())
    }

    fn config(&self, path: &Path) -> Result<(Config, bool), String> {
        let text = fs::read_to_string(path).unwrap_or_default();
        let keys: toml::Table = text
            .parse()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let port = match (
            keys.get("port").and_then(toml::Value::as_integer),
            self.port,
        ) {
            (Some(had), Some(want)) if had != i64::from(want) => {
                return Err(format!(
                    "{} says port = {had}: change it to {want} first",
                    path.display()
                ));
            }
            (Some(had), _) => u16::try_from(had).map_err(|e| e.to_string())?,
            (None, want) => want.unwrap_or(Config::default().port),
        };
        let mut add = Vec::new();
        if !keys.contains_key("port") {
            add.push(format!("port = {port}"));
        }
        if !keys.contains_key("allowed_hosts") {
            match tailnet_name() {
                Some(name) => add.push(format!("allowed_hosts = [{name:?}]")),
                None => {
                    cooee_core::say!(
                        "  note: no tailnet name (is Tailscale up?): allowed_hosts left out"
                    );
                }
            }
        }
        if !keys.contains_key("roots") {
            let root = self.ask("folders for threads [~/Projects]:", "~/Projects");
            add.push(format!("roots = [{root:?}]"));
        }
        let plugins = keys.get("plugins").and_then(toml::Value::as_array);
        let has = plugins.is_some_and(|a| {
            a.iter()
                .any(|p| p.get("name").and_then(toml::Value::as_str) == Some(cooee_terminal::NAME))
        });
        let plugin = if has {
            String::new()
        } else {
            let at = port
                .checked_add(1000)
                .ok_or("no port + 1000 for the terminal plugin")?;
            let token = cooee_core::sha256(&cooee_core::random(32).map_err(|e| e.to_string())?);
            format!(
                "\n[[plugins]]\nname = \"terminal\"\nlabel = \"Terminal\"\nurl = \"http://127.0.0.1:{at}\"\n\
                 module = \"/tab.js\"\ntoken = \"{token}\"\n"
            )
        };
        let changed = !add.is_empty() || !plugin.is_empty();
        if !step(&format!("config {}", path.display()), !changed) {
            if !text.is_empty() {
                let what = add.iter().map(String::as_str);
                let what: Vec<&str> = what
                    .chain((!has).then_some("the terminal plugin"))
                    .collect();
                cooee_core::say!("  note: kept its keys; adding {}", what.join(", "));
            }
            let head: String = add.iter().flat_map(|k| [k.as_str(), "\n"]).collect();
            let new = format!("{head}{text}{plugin}");
            self.write(path, &new, || {
                let dir = path.parent().unwrap_or(path);
                if !dir.exists() {
                    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                    cooee_core::gitignore::write(dir).map_err(|e| e.to_string())?;
                }
                fs::write(path, &new).map_err(|e| e.to_string())
            })?;
        }
        let config = if path.exists() {
            load(Some(path), &self.instance)?
        } else {
            Config {
                port,
                instance: self.instance.clone(),
                ..Config::default()
            }
        };
        Ok((config, changed))
    }

    fn themes(&self, config: &Config) -> Result<(), String> {
        let e = |e: io::Error| e.to_string();
        let t = match cooee_server::Themes::new(config) {
            Err(_) if self.dry_run => {
                cooee_core::say!(
                    "themes:\n  would install farm and ask which other themes to install"
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
        let (farm, rest): (Vec<&str>, Vec<&str>) = first.into_iter().partition(|n| *n == "farm");
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

    fn agents(&self, config: &Config) -> Result<(), String> {
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

    fn service(
        &self,
        config: &Config,
        path: &Path,
        gui: &str,
        changed: bool,
        verb: &str,
    ) -> Result<(), String> {
        let label = job(&self.instance, verb);
        let (file, text) = plist(config, path, self.explicit, verb)?;
        let same = fs::read_to_string(&file).is_ok_and(|t| t == text);
        let loaded = ok(&["launchctl", "list", &label]);
        if step(
            &format!("cooee service {label}"),
            same && loaded && !changed,
        ) {
            return Ok(());
        }
        if !same {
            self.write(&file, &text, || write_plist(config, &file, &text))?;
        }
        let target = format!("{gui}/{label}");
        if loaded && same {
            self.exec(&["launchctl", "kickstart", "-k", &target])?;
        } else {
            if loaded {
                self.exec(&["launchctl", "bootout", &target])?;
            }
            self.exec(&["launchctl", "bootstrap", gui, &file.display().to_string()])?;
        }
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, config.port));
        if !self.dry_run {
            wait(|| TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok());
        }
        Ok(())
    }

    fn tailnet(&self, port: u16) -> Result<(), String> {
        let served = output(&["tailscale", "serve", "status", "--json"])
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .is_some_and(|s| {
                s.get("TCP")
                    .and_then(|t| t.get(port.to_string()))
                    .is_some_and(|p| !p.is_null())
            });
        if !step(&format!("tailscale serve {port}"), served) {
            let (https, to) = (
                format!("--https={port}"),
                format!("http://127.0.0.1:{port}"),
            );
            self.exec(&["tailscale", "serve", "--bg", &https, &to])?;
        }
        Ok(())
    }
}

fn step(name: &str, done: bool) -> bool {
    cooee_core::say!("{name}:{}", if done { " done, skipped" } else { "" });
    done
}
