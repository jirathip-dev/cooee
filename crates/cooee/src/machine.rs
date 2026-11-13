//! `cooee machine check <m> [--yes]` and `cooee doctor`'s row per machine (#150): whether machine `m` is ready for
//! the personas that run on it. shed reaches it with the same version; then for each persona its agent's CLI is
//! installed there, its folder is there, its account is signed in there and its profile's settings (its rules and
//! hooks) are there as here. Every check runs on `m` as a plain shed terminal; cooee never reaches it but through
//! shed. With `--yes` a missing folder is cloned there from its git origin here, and the settings are copied there.

use std::{collections::BTreeSet, fs, path::Path, process::Command};

use cooee_core::{
    Accounts, Config, Provider, Providers, Shed, personas, personas::Persona, profiles,
    shed_proto::LinkState,
};

/// How long one check may run there, in seconds.
const PROBE: f64 = 30.0;
/// How long a clone may run there, in seconds.
const CLONE: f64 = 600.0;

/// One check's line: whether it passed, and what it says (with its fix when it failed).
pub type Line = (bool, String);

/// What the checks read: the config, its shed, providers, accounts and personas.
struct Ctx {
    shed: Shed,
    providers: Providers,
    accounts: Option<Accounts>,
    personas: Vec<Persona>,
    config: Option<std::path::PathBuf>,
    data: std::path::PathBuf,
}

impl Ctx {
    fn new(config: &Config) -> Self {
        let data = config.data_dir().unwrap_or_default();
        Self {
            shed: Shed::new(config.shed.clone(), config.shed_dir.clone()),
            providers: Providers::new(&config.providers),
            accounts: config.data_dir().map(|d| Accounts::new(&d)),
            personas: personas::list(config.dir.as_deref(), &data),
            config: config.dir.clone(),
            data,
        }
    }

    /// `cmd` on `machine`: whether it exited 0.
    fn ran(&self, machine: &str, cmd: &[&str], timeout: f64) -> Result<bool, String> {
        let cmd = cmd.iter().map(|w| (*w).to_owned()).collect();
        let exit = self.shed.run_on(machine, cmd, timeout);
        exit.map(|e| e == Some(0)).map_err(|e| e.to_string())
    }

    /// Every check of `machine`, applying the fixes with `yes`.
    fn check(&self, machine: &str, yes: bool) -> Vec<Line> {
        let link = match self.shed.link(machine) {
            Ok(Some(l)) if l.state == LinkState::Up => Ok(()),
            Ok(Some(l)) => Err(format!(
                "{} since {}{}",
                format!("{:?}", l.state).to_lowercase(),
                cooee_machines::ago(cooee_machines::unix_now(), l.since),
                l.error.map(|e| format!(": {e}")).unwrap_or_default()
            )),
            Ok(None) => Err("not added: shed machine add".to_owned()),
            Err(e) => Err(e.to_string()),
        };
        let Err(why) = link else {
            let mut out = vec![(true, format!("shed on {machine}: reachable, same version"))];
            let on = self.personas.iter().filter(|p| p.machine == machine);
            out.extend(on.flat_map(|p| self.persona(machine, p, yes)));
            return out;
        };
        vec![(false, format!("shed on {machine}: {why}"))]
    }

    /// Persona `p`'s checks on `machine`.
    fn persona(&self, machine: &str, p: &Persona, yes: bool) -> Vec<Line> {
        let profile = p
            .profile
            .as_deref()
            .and_then(|id| profiles::find(self.config.as_deref(), &self.data, id).ok());
        let mut out = Vec::new();
        match self.providers.for_agent(&p.agent) {
            Some(provider) => {
                let account = profile.as_ref().and_then(|f| f.account.as_deref());
                out.push(self.cli(machine, provider.as_ref()));
                out.push(self.signed_in(machine, provider.as_ref(), account.unwrap_or("")));
            }
            None => out.push((false, format!("no provider signs in agent {:?}", p.agent))),
        }
        if !matches!(p.folder.as_str(), "" | "scratch") {
            out.push(self.folder(machine, &p.folder, yes));
        }
        if let Some(settings) = profile.and_then(|f| f.settings) {
            out.push(self.rules(machine, &settings, yes));
        }
        out.into_iter()
            .map(|(ok, what)| (ok, format!("{}: {what}", p.id)))
            .collect()
    }

    fn cli(&self, machine: &str, provider: &dyn Provider) -> Line {
        let bin = provider.spec().login.first().map_or("", String::as_str);
        match self.ran(
            machine,
            &["sh", "-c", r#"command -v "$1""#, "sh", bin],
            PROBE,
        ) {
            Ok(true) => (true, format!("{bin} installed")),
            Ok(false) => (
                false,
                format!("{bin} not installed\n     fix: {}", provider.spec().install),
            ),
            Err(e) => (false, format!("{bin}: {e}")),
        }
    }

    fn signed_in(&self, machine: &str, provider: &dyn Provider, account: &str) -> Line {
        let spec = provider.spec();
        let home = self
            .accounts
            .as_ref()
            .and_then(|a| a.get(&spec.name, account)?.dir);
        let cmd = provider.with_home(&spec.status, home.as_deref());
        let cmd: Vec<&str> = cmd.iter().map(String::as_str).collect();
        let shown = if account.is_empty() {
            "default"
        } else {
            account
        };
        let what = format!("{} account {shown} signed in", spec.name);
        match self.ran(machine, &cmd, PROBE) {
            Ok(true) => (true, what),
            Ok(false) => (
                false,
                format!(
                    "{what}: no\n     fix: sign in to {} on {machine}",
                    spec.name
                ),
            ),
            Err(e) => (false, format!("{what}: {e}")),
        }
    }

    fn folder(&self, machine: &str, folder: &str, yes: bool) -> Line {
        let what = format!("folder {folder}");
        match self.ran(machine, &["test", "-d", folder], PROBE) {
            Ok(true) => return (true, what),
            Err(e) => return (false, format!("{what}: {e}")),
            Ok(false) => {}
        }
        let origin = Command::new("git")
            .args(["-C", folder, "remote", "get-url", "origin"])
            .output();
        let origin = origin.ok().filter(|o| o.status.success());
        let Some(url) = origin.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned()) else {
            return (
                false,
                format!("{what} missing, and no git origin here to clone"),
            );
        };
        if !yes {
            let fix = format!("cooee machine check {machine} --yes (clones {url} there)");
            return (false, format!("{what} missing\n     fix: {fix}"));
        }
        match self.ran(machine, &["git", "clone", "--quiet", &url, folder], CLONE) {
            Ok(true) => (true, format!("{what}: cloned {url}")),
            Ok(false) => (false, format!("{what}: git clone {url} failed")),
            Err(e) => (false, format!("{what}: {e}")),
        }
    }

    fn rules(&self, machine: &str, settings: &Path, yes: bool) -> Line {
        let path = settings.to_string_lossy();
        let what = format!("rules and hooks {path}");
        let Ok(text) = fs::read_to_string(settings) else {
            return (false, format!("{what}: can't be read here"));
        };
        let same = r#"printf %s "$2" | cmp -s - "$1""#;
        match self.ran(machine, &["sh", "-c", same, "sh", &path, &text], PROBE) {
            Ok(true) => return (true, format!("{what} synced")),
            Err(e) => return (false, format!("{what}: {e}")),
            Ok(false) if !yes => {
                let fix = format!("cooee machine check {machine} --yes (copies it there)");
                return (false, format!("{what} not synced\n     fix: {fix}"));
            }
            Ok(false) => {}
        }
        let copy = r#"mkdir -p "$(dirname "$1")" && printf %s "$2" > "$1""#;
        match self.ran(machine, &["sh", "-c", copy, "sh", &path, &text], PROBE) {
            Ok(true) => (true, format!("{what} synced: copied")),
            Ok(false) => (false, format!("{what}: copying it failed")),
            Err(e) => (false, format!("{what}: {e}")),
        }
    }
}

/// `cooee machine check <machine> [--yes]`: each check's line; fails if any fails.
///
/// # Errors
/// How many checks failed.
pub fn run(config: &Config, machine: &str, yes: bool) -> Result<(), String> {
    let lines = Ctx::new(config).check(machine, yes);
    for (ok, what) in &lines {
        println!("{} {what}", if *ok { "ok  " } else { "FAIL" });
    }
    match lines.iter().filter(|(ok, _)| !ok).count() {
        0 => Ok(()),
        n => Err(format!("{machine}: {n} check(s) failed")),
    }
}

/// `cooee doctor`'s row per machine shed holds a link to or a persona runs on.
#[must_use]
pub fn rows(config: &Config) -> Vec<Line> {
    let ctx = Ctx::new(config);
    let linked = ctx.shed.links().unwrap_or_default().into_keys();
    let named = ctx.personas.iter().map(|p| p.machine.clone());
    let machines: BTreeSet<String> = linked.chain(named).filter(|m| !m.is_empty()).collect();
    machines
        .into_iter()
        .map(|m| {
            let failed = ctx
                .check(&m, false)
                .into_iter()
                .filter(|(ok, _)| !ok)
                .count();
            match failed {
                0 => (true, format!("machine {m} ready")),
                n => (
                    false,
                    format!("machine {m}: {n} check(s) failed (cooee machine check {m})"),
                ),
            }
        })
        .collect()
}
