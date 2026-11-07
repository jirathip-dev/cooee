//! `cooee setup`: shed at the pinned rev and its service, the config, agents, cooee's service, `tailscale serve`,
//! doctor and a pairing code; each step is skipped when already done, and `--dry-run` only prints the commands.

use std::{
    env, fs, io,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use cooee_core::{Config, Shed};

use super::{SHED_CARGO, doctor, label, load, pair, plist, tailnet_name, write_plist};

const SHED_LABEL: &str = "dev.shed.serve";

/// `cooee setup`'s flags.
pub struct Setup {
    pub instance: String,
    pub port: Option<u16>,
    pub explicit: bool,
    pub dry_run: bool,
    pub yes: bool,
}

/// Whether `argv` exits 0 (its output dropped).
fn ok(argv: &[&str]) -> bool {
    Command::new(argv[0])
        .args(&argv[1..])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// `argv`'s stdout, if it exits 0.
fn output(argv: &[&str]) -> Option<String> {
    let out = Command::new(argv[0]).args(&argv[1..]).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Polls `f` for up to 10 seconds.
fn wait(f: impl Fn() -> bool) -> bool {
    (0..40).any(|_| {
        let done = f();
        if !done {
            thread::sleep(Duration::from_millis(250));
        }
        done
    })
}

impl Setup {
    /// Runs `argv`, or with `--dry-run` only prints it.
    fn exec(&self, argv: &[&str]) -> Result<(), String> {
        let line = argv.join(" ");
        if self.dry_run {
            println!("  would run: {line}");
            return Ok(());
        }
        println!("  $ {line}");
        let status = Command::new(argv[0]).args(&argv[1..]).status();
        match status {
            Ok(s) if s.success() => Ok(()),
            Ok(s) => Err(format!("`{line}` failed ({s})")),
            Err(e) => Err(format!("`{line}`: {e}")),
        }
    }

    /// Writes `text` to `file`, or with `--dry-run` only prints it.
    fn write(
        &self,
        file: &Path,
        text: &str,
        f: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.dry_run {
            println!("  would write {}:\n{text}", file.display());
            return Ok(());
        }
        f()?;
        println!("  wrote {}", file.display());
        Ok(())
    }

    /// Asks `question`; `--yes`, an empty answer or no input take `default`.
    fn ask(&self, question: &str, default: &str) -> String {
        if self.yes {
            return default.to_owned();
        }
        println!("  {question}");
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        let line = line.trim();
        (if line.is_empty() { default } else { line }).to_owned()
    }

    /// Every step, in order, then the pairing code.
    pub fn run(&self, path: &Path) -> Result<(), String> {
        let uid = output(&["id", "-u"]).ok_or("`id -u` failed")?;
        let gui = format!("gui/{}", uid.trim());
        let updated = self.shed()?;
        self.shed_service(&gui, updated)?;
        let (config, changed) = self.config(path)?;
        self.themes(&config)?;
        self.agents(&config)?;
        self.service(&config, path, &gui, changed)?;
        self.tailnet(config.port)?;
        if self.dry_run {
            println!("would run: cooee doctor, then cooee pair");
            return Ok(());
        }
        println!("doctor:");
        doctor(&config)
            .map_err(|e| format!("doctor: {e}\nfix it, then run `cooee setup` again"))?;
        println!("\npair your phone:");
        pair(&config)
    }

    /// shed at the pinned rev; whether it was installed now.
    fn shed(&self) -> Result<bool, String> {
        let rev = SHED_CARGO[6];
        let done = output(&["cargo", "install", "--list"])
            .is_some_and(|l| l.contains(&format!("/shed?rev={rev}#")));
        if !step(&format!("shed {}", &rev[..7]), done) {
            self.exec(&SHED_CARGO)?;
        }
        Ok(!done)
    }

    /// shed's launchd job (`shed install`), started or restarted onto a new binary, answering `shed ls`.
    fn shed_service(&self, gui: &str, updated: bool) -> Result<(), String> {
        let loaded = ok(&["launchctl", "list", SHED_LABEL]);
        let answers = || ok(&["shed", "ls"]);
        if step(
            &format!("shed service {SHED_LABEL}"),
            !updated && loaded && answers(),
        ) {
            return Ok(());
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
            let home = env::var_os("HOME").ok_or("no HOME")?;
            let file = Path::new(&home).join(format!("Library/LaunchAgents/{SHED_LABEL}.plist"));
            self.exec(&["launchctl", "bootstrap", gui, &file.display().to_string()])?;
        }
        if !self.dry_run && !wait(answers) {
            return Err("shed doesn't answer `shed ls`; see ~/Library/Logs/shed.log".to_owned());
        }
        Ok(())
    }

    /// The config with `port`, `allowed_hosts` and `roots`: missing keys added, the others kept; whether it changed.
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
                    println!("  note: no tailnet name (is Tailscale up?): allowed_hosts left out");
                }
            }
        }
        if !keys.contains_key("roots") {
            let root = self.ask("folders for threads [~/Projects]:", "~/Projects");
            add.push(format!("roots = [{root:?}]"));
        }
        // The owner's terminal plugin (#28), on the instance's port + 1000 with a token of its own.
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
                println!("  note: kept its keys; adding {}", what.join(", "));
            }
            // Top-level keys go first: after a `[table]` they would belong to it. The plugin goes last.
            let head: String = add.iter().flat_map(|k| [k.as_str(), "\n"]).collect();
            let new = format!("{head}{text}{plugin}");
            self.write(path, &new, || {
                fs::create_dir_all(path.parent().unwrap_or(path)).map_err(|e| e.to_string())?;
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

    /// The first-party theme packs asked for (default: all of them) from the index, the installed ones skipped.
    fn themes(&self, config: &Config) -> Result<(), String> {
        let e = |e: io::Error| e.to_string();
        let t = match cooee_server::Themes::new(config) {
            Err(_) if self.dry_run => {
                println!("themes:\n  would ask which first-party themes to install");
                return Ok(());
            }
            t => t.map_err(e)?,
        };
        let have: Vec<String> = t.list().map_err(e)?.into_iter().map(|p| p.name).collect();
        let first: Vec<&str> = t
            .index()
            .iter()
            .filter(|l| l.first_party && !have.contains(&l.name))
            .map(|l| l.name.as_str())
            .collect();
        if step("themes", first.is_empty()) {
            return Ok(());
        }
        let all = first.join(" ");
        let want = self.ask(&format!("themes to install, or none [{all}]:"), &all);
        for name in want
            .split([' ', ','])
            .filter(|n| !n.is_empty() && *n != "none")
        {
            if !first.contains(&name) {
                return Err(format!("theme {name}: not one of {all}"));
            }
            if self.dry_run {
                println!("  would install theme {name}");
                continue;
            }
            // One that can't be fetched now (offline) is no reason to stop.
            match t.plan(name, None).and_then(|p| t.add(&p)) {
                Ok(()) => println!("  installed theme {name}"),
                Err(why) => println!("  theme {name}: {why}\n  later: cooee theme add {name}"),
            }
        }
        Ok(())
    }

    /// Claude's ACP agent in shed, and Codex's and Gemini's if asked for.
    fn agents(&self, config: &Config) -> Result<(), String> {
        let have: Vec<String> = Shed::new(config.shed.clone(), config.shed_dir.clone())
            .agents()
            .map(|a| a.into_iter().map(|a| a.id).collect())
            .unwrap_or_default();
        let mut want = Vec::new();
        if !have.iter().any(|a| a == "claude-acp") {
            want.push("claude-acp");
        }
        for (name, id) in [("Codex", "codex-acp"), ("Gemini", "gemini")] {
            let missing = !have.iter().any(|a| a == id);
            if missing
                && self
                    .ask(&format!("also {name}? [y/N]"), "n")
                    .starts_with(['y', 'Y'])
            {
                want.push(id);
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
            argv.extend(["agents", "add", id]);
            self.exec(&argv)?;
        }
        Ok(())
    }

    /// cooee's launchd job: its plist written and loaded, restarted onto a changed config.
    fn service(
        &self,
        config: &Config,
        path: &Path,
        gui: &str,
        changed: bool,
    ) -> Result<(), String> {
        let label = label(&self.instance);
        let (file, text) = plist(config, path, self.explicit)?;
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

    /// `tailscale serve` of the port over https, unless it is served already.
    fn tailnet(&self, port: u16) -> Result<(), String> {
        let served = output(&["tailscale", "serve", "status", "--json"])
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .is_some_and(|s| !s["TCP"][port.to_string()].is_null());
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

/// Prints step `name`, `done, skipped` if `done`; returns `done`.
fn step(name: &str, done: bool) -> bool {
    println!("{name}:{}", if done { " done, skipped" } else { "" });
    done
}
