//! Provider logins and accounts: a provider's own login command for one account's config folder in a shed terminal
//! (terminal mode), its screen read live for the page (redacted, never stored or logged), typed codes, and the
//! terminal killed and removed when the command exits, is cancelled or after 10 minutes. A new account's folder is
//! listed only once its login succeeds; a sign-out runs the CLI's logout, then removes the folder.

use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use cooee_core::{
    Account, Accounts, Launch, Profile, Provider, Shed, Sheet, Status, links, redact,
    shed_proto::{RunParams, WaitOutcome},
};
use serde::Serialize;

use crate::hub::{HubError, lock, now};

/// How long a login terminal may run, in seconds.
pub const LOGIN_TIMEOUT: f64 = 600.0;

/// The label every login terminal carries.
const LABEL: &str = "cooee-login";

/// A provider's login.
enum Login {
    /// Its terminal runs.
    Running(String),
    /// It ended, and why; the account it was for.
    Ended(String, String),
}

/// A provider's login as the page sees it.
#[derive(Debug, Serialize)]
pub struct View {
    /// `running`, `ended` (`why` says how) or `none`.
    pub state: &'static str,
    /// The terminal's screen, redacted, while it runs.
    pub screen: String,
    /// The links on the screen.
    pub links: Vec<String>,
    /// How the last login ended.
    pub why: Option<String>,
    /// The status of the account it was for (else the system default), as last checked, once no login runs.
    pub status: Option<Status>,
    /// How long ago that status was checked, in seconds; none before its first check.
    pub age: Option<u64>,
}

/// How often an account's status is checked when nothing asks, in ms (#106).
const EVERY: u64 = 10 * 60_000;
/// The longest that interval grows to, doubling after each timeout.
const SLOWEST: u64 = 80 * 60_000;
/// How soon after a check another may be asked for (the sign-ins card opened), in ms.
const GAP: u64 = 30_000;

/// One account's status checks: the last result, and when the next is due.
#[derive(Default)]
struct Check {
    status: Option<Status>,
    /// When the last check finished (ms).
    at: u64,
    /// The interval after it: [`EVERY`], doubled after each timeout up to [`SLOWEST`].
    every: u64,
    /// When the next check is due (ms).
    next: u64,
    /// A check runs: never two at once.
    busy: bool,
}

impl Check {
    /// Whether a check is due at `at`; it is then marked running.
    fn start(&mut self, at: u64) -> bool {
        let due = !self.busy && at >= self.next;
        self.busy |= due;
        due
    }

    /// Notes check result `s` at `at`: a timeout keeps the signed-in state last known and doubles the interval, any
    /// other result resets it.
    fn settle(&mut self, s: Status, at: u64) {
        self.every = if s.unknown {
            (self.every.max(EVERY) * 2).min(SLOWEST)
        } else {
            EVERY
        };
        (self.at, self.next) = (at, at + self.every);
        self.status = Some(match self.status.take() {
            Some(old) if s.unknown => Status {
                signed_in: old.signed_in,
                who: old.who,
                usage: old.usage,
                ..s
            },
            _ => s,
        });
    }

    /// Asked for at `at`: due now, unless it backs off after a timeout or ran in the last [`GAP`].
    fn ask(&mut self, at: u64) {
        if self.every <= EVERY && at >= self.at + GAP {
            self.next = self.next.min(at);
        }
    }

    /// Its status as last checked, and its age in seconds at `at`.
    fn known(&self, at: u64) -> (Status, Option<u64>) {
        match &self.status {
            Some(s) => (s.clone(), Some(at.saturating_sub(self.at) / 1000)),
            None => (Status::unknown("not checked yet"), None),
        }
    }
}

/// The providers, their accounts and logins.
pub struct Logins {
    shed: Shed,
    providers: Vec<Provider>,
    accounts: Accounts,
    live: Mutex<BTreeMap<String, Login>>,
    /// Each account's status checks, by provider and account: a request never runs a status command itself
    /// (herdsman#337, #106). Checked at start, every 10 minutes (longer after a timeout), when the sign-ins card
    /// opens, and after a login or sign-out.
    checks: Mutex<BTreeMap<(String, String), Check>>,
}

impl Logins {
    /// The providers' logins on `shed`, their accounts in `data`; removes login terminals a previous run left behind.
    #[must_use]
    pub fn new(shed: Shed, providers: Vec<Provider>, data: &Path) -> Arc<Self> {
        let logins = Arc::new(Self {
            shed,
            providers,
            accounts: Accounts::new(data),
            live: Mutex::new(BTreeMap::new()),
            checks: Mutex::new(BTreeMap::new()),
        });
        let checker = Arc::clone(&logins);
        thread::spawn(move || {
            loop {
                checker.tick();
                thread::sleep(Duration::from_secs(1));
            }
        });
        let shed = logins.shed.clone();
        thread::spawn(move || {
            let labels = BTreeMap::from([(LABEL.to_owned(), "1".to_owned())]);
            for t in shed.ls(labels).unwrap_or_default() {
                let _ = shed.close(&t.name);
            }
        });
        logins
    }

    fn provider(&self, name: &str) -> Result<&Provider, HubError> {
        self.providers
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| HubError::Refused(format!("no provider {name}")))
    }

    fn account(&self, provider: &str, name: &str) -> Result<Account, HubError> {
        self.accounts
            .get(provider, name)
            .ok_or_else(|| HubError::Refused(format!("{provider} has no account {name}")))
    }

    fn running(&self, name: &str) -> Option<String> {
        match lock(&self.live).get(name) {
            Some(Login::Running(t)) => Some(t.clone()),
            _ => None,
        }
    }

    /// Every provider with its accounts' status, as last checked, and how its last login ended.
    #[must_use]
    pub fn statuses(&self) -> Vec<Sheet> {
        let at = now();
        let checks = lock(&self.checks);
        let live = lock(&self.live);
        let sheet = |p: &Provider| {
            let mut accounts: Vec<_> = (self.accounts.list(&p.name).into_iter())
                .map(|a| {
                    let check = checks.get(&(p.name.clone(), a.name.clone()));
                    let status = check.map_or_else(|| Check::default().known(at), |c| c.known(at));
                    (a, status.0)
                })
                .collect();
            if p.missing() {
                accounts.truncate(1);
            }
            let why = match live.get(&p.name) {
                Some(Login::Ended(why, _)) => Some(why.clone()),
                _ => None,
            };
            Sheet {
                provider: p.clone(),
                accounts,
                why,
            }
        };
        self.providers.iter().map(sheet).collect()
    }

    /// Starts each account's status check that is due, each on its own thread.
    fn tick(self: &Arc<Self>) {
        for p in &self.providers {
            for a in self.accounts.list(&p.name) {
                let key = (p.name.clone(), a.name.clone());
                if !lock(&self.checks)
                    .entry(key.clone())
                    .or_default()
                    .start(now())
                {
                    continue;
                }
                let (logins, p) = (Arc::clone(self), p.clone());
                thread::spawn(move || {
                    let status = p.check(a.dir.as_deref());
                    let mut checks = lock(&logins.checks);
                    if let Some(c) = checks.get_mut(&key) {
                        c.busy = false;
                        c.settle(status, now());
                    }
                });
            }
        }
    }

    /// Asks for every account's status check again (the sign-ins card opened): those not checked in the last 30 s
    /// nor backing off after a timeout run within a second.
    pub fn ask(&self) {
        let at = now();
        for c in lock(&self.checks).values_mut() {
            c.ask(at);
        }
    }

    /// Starts provider `name`'s login command in a new shed terminal (a running one is kept): for account
    /// `account` (none: the system default), or with `new` in a new account's folder.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider or account, or `new` for a provider without a config
    /// variable; [`HubError::Shed`] when the terminal can't start.
    pub fn start(
        self: &Arc<Self>,
        name: &str,
        account: Option<&str>,
        new: bool,
    ) -> Result<(), HubError> {
        let provider = self.provider(name)?;
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(_))) {
            return Ok(());
        }
        let (account, home) = if new {
            if provider.home.is_empty() {
                return Err(HubError::Refused(format!(
                    "{name} has only its system default account"
                )));
            }
            let (account, dir) = self
                .accounts
                .pending(name)
                .map_err(|e| HubError::Refused(e.to_string()))?;
            (account, Some(dir))
        } else {
            let a = self.account(name, account.unwrap_or_default())?;
            (a.name, a.dir)
        };
        let terminal = format!("{LABEL}-{name}-{:x}", now());
        let run = RunParams {
            name: Some(terminal.clone()),
            labels: BTreeMap::from([(LABEL.to_owned(), "1".to_owned())]),
            cwd: env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from),
            keep: false,
            idle_after: None,
            cmd: provider.with_home(&provider.login, home.as_deref()),
            acp: None,
            resume: None,
            mcp_servers: Vec::new(),
        };
        if let Err(e) = self.shed.run(&run) {
            if new {
                self.accounts.drop_pending(name, &account);
            }
            return Err(e.into());
        }
        // Wide, so a long sign-in link stays on one line.
        let _ = self.shed.resize(&terminal, 40, 500);
        live.insert(name.to_owned(), Login::Running(terminal.clone()));
        let logins = Arc::clone(self);
        let name = name.to_owned();
        thread::spawn(move || logins.reap(&name, &terminal, &account, new, home.as_deref()));
        Ok(())
    }

    /// Waits for login terminal `terminal` to exit (or for the timeout), then kills and removes it. A login that
    /// exited 0 and left the account signed in is noted; a new account's folder is then listed, else removed.
    fn reap(&self, name: &str, terminal: &str, account: &str, new: bool, home: Option<&Path>) {
        let (ok, mut why) = match self.shed.exited(terminal, LOGIN_TIMEOUT) {
            Ok(r) if r.outcome == WaitOutcome::Timeout => {
                (false, "timed out after 10 minutes".to_owned())
            }
            Ok(r) => match r.terminal.exit {
                Some(0) => (true, "the login command finished".to_owned()),
                Some(code) => (false, format!("the login command exited {code}")),
                None => (false, "the login command ended".to_owned()),
            },
            Err(e) => (false, e.to_string()),
        };
        if let Err(e) = self.shed.close(terminal) {
            eprintln!("cooee: {terminal}: {e}");
        }
        let status = self.provider(name).ok().map(|p| p.check(home));
        let signed_in = ok && status.as_ref().is_some_and(|s| s.signed_in);
        let noted = match (signed_in, new) {
            (true, true) => self.accounts.adopt(name, account, now()),
            (true, false) => self.accounts.signed_in(name, account, now()),
            (false, true) => {
                self.accounts.drop_pending(name, account);
                Ok(())
            }
            (false, false) => Ok(()),
        };
        if let Err(e) = noted {
            why = format!("{account}: {e}");
        } else if ok && !signed_in {
            "the login command finished, but the account isn't signed in".clone_into(&mut why);
        }
        // A new account's folder moved when it was listed: its status is checked there.
        let moved = |p: &Provider| Some(p.check(self.accounts.get(name, account)?.dir.as_deref()));
        let status = if new && signed_in {
            self.provider(name).ok().and_then(moved)
        } else {
            status.filter(|_| !new)
        };
        if let Some(status) = status {
            let key = (name.to_owned(), account.to_owned());
            lock(&self.checks)
                .entry(key)
                .or_default()
                .settle(status, now());
        }
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(t)) if t == terminal) {
            let account = if new && !signed_in { "" } else { account };
            live.insert(name.to_owned(), Login::Ended(why, account.to_owned()));
        }
    }

    /// Provider `name`'s login: its screen while it runs, else how it ended and its account's status.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider.
    pub fn view(&self, name: &str) -> Result<View, HubError> {
        let provider = self.provider(name)?;
        if let Some(screen) = self.running(name).and_then(|t| self.shed.screen(&t).ok()) {
            let screen = redact(&screen);
            return Ok(View {
                state: "running",
                links: links(&screen),
                screen,
                why: None,
                status: None,
                age: None,
            });
        }
        let (why, account) = match lock(&self.live).get(name) {
            Some(Login::Ended(why, account)) => (Some(why.clone()), account.clone()),
            _ => (None, String::new()),
        };
        let key = (provider.name.clone(), account);
        let (status, age) = lock(&self.checks)
            .get(&key)
            .map_or_else(|| Check::default().known(now()), |c| c.known(now()));
        Ok(View {
            state: if why.is_some() { "ended" } else { "none" },
            screen: String::new(),
            links: Vec::new(),
            why,
            status: Some(status),
            age,
        })
    }

    /// Types `text` (a code from the sign-in page) into provider `name`'s login terminal, then enter.
    ///
    /// # Errors
    /// [`HubError::Refused`] when no login runs; [`HubError::Shed`] when shed fails.
    pub fn send(&self, name: &str, text: &str) -> Result<(), HubError> {
        let terminal = self
            .running(name)
            .ok_or_else(|| HubError::Refused(format!("no {name} login is running")))?;
        Ok(self.shed.type_line(&terminal, text.trim())?)
    }

    /// Cancels provider `name`'s login: kills and removes its terminal.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails.
    pub fn cancel(&self, name: &str) -> Result<(), HubError> {
        let Some(terminal) = self.running(name) else {
            return Ok(());
        };
        self.shed.close(&terminal)?;
        let why = Login::Ended("cancelled".to_owned(), String::new());
        lock(&self.live).insert(name.to_owned(), why);
        Ok(())
    }

    /// Signs provider `name`'s account `account` out with the CLI's logout, then removes its folder (an added
    /// account's, even when the logout fails; the system default's is never removed).
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown account, or with the logout's output when it fails.
    pub fn sign_out(&self, name: &str, account: &str) -> Result<(), HubError> {
        let provider = self.provider(name)?;
        let a = self.account(name, account)?;
        let out = provider.sign_out(a.dir.as_deref());
        let changed = out.is_ok() || a.dir.is_some();
        if changed {
            self.accounts
                .remove(name, account)
                .map_err(|e| HubError::Refused(e.to_string()))?;
        }
        // Checked again within a second, whatever its backoff; unknown until then, unless nothing was signed out.
        let key = (name.to_owned(), account.to_owned());
        if let Some(c) = lock(&self.checks).get_mut(&key) {
            c.next = 0;
            if changed {
                c.status = None;
            }
        }
        let who = if account.is_empty() { name } else { account };
        out.map_err(|e| HubError::Refused(format!("{who} sign-out: {e}")))
    }

    /// What a thread of ACP agent `agent` runs with: its account's name (empty for the system default, or an agent
    /// no provider signs in), and its config folder in the provider's variable (none for the system default). `account` none:
    /// the system default.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an account the agent's provider doesn't have.
    pub fn home(&self, agent: &str, account: Option<&str>) -> Result<(String, Launch), HubError> {
        let Some(p) = self.providers.iter().find(|p| p.agent == agent) else {
            return Ok((String::new(), Launch::default()));
        };
        let a = self.account(&p.name, account.unwrap_or_default())?;
        let env = p.env(a.dir.as_deref()).into_iter();
        let env = env.map(|(var, dir)| (var.to_owned(), dir.display().to_string()));
        let launch = Launch {
            env: env.collect(),
            ..Launch::default()
        };
        Ok((a.name, launch))
    }

    /// What a thread of agent profile `p` runs with: its account's label and [`Launch`] (the account's folder, the
    /// settings file in the provider's `settings` variable (in place of the account's folder when it is the same
    /// variable), the env, its secrets read from `data` through a file
    /// there, the policy), and its instructions. A profile its agent can't run as written is refused: settings
    /// with no `settings` variable (its guard can't run) or MCP servers (shed starts sessions without them).
    ///
    /// # Errors
    /// [`HubError::Refused`] as above, for an unknown account, a missing secret or an unreadable file.
    pub fn profile(&self, p: &Profile, data: &Path) -> Result<(String, Launch, String), HubError> {
        let refused = |why: String| HubError::Refused(why);
        let (account, mut launch) = self.home(&p.agent, p.account.as_deref())?;
        if let Some(settings) = &p.settings {
            let var = self.providers.iter().find(|v| v.agent == p.agent);
            let var = var.map(|v| v.settings.as_str()).filter(|v| !v.is_empty());
            let Some(var) = var else {
                return Err(refused(format!(
                    "this profile's guard can't run with {}: its ACP adapter has no settings variable",
                    p.agent
                )));
            };
            launch.env.retain(|(k, _)| k != var);
            launch
                .env
                .push((var.to_owned(), settings.display().to_string()));
        }
        if p.mcp.is_some() {
            return Err(refused(format!(
                "this profile's MCP servers can't run with {}: shed starts ACP sessions without them",
                p.agent
            )));
        }
        let (plain, secret) = p.env(data).map_err(refused)?;
        launch.env.extend(plain);
        if !secret.is_empty() {
            let name = cooee_core::random(16).map_err(|e| refused(e.to_string()))?;
            let name = cooee_core::sha256(&name);
            launch.secrets = Some((data.join("secrets").join(format!(".run-{name}")), secret));
        }
        launch.policy.clone_from(&p.policy);
        let instructions = match &p.instructions {
            Some(f) => std::fs::read_to_string(f)
                .map_err(|e| refused(format!("profile {}: {}: {e}", p.name, f.display())))?,
            None => String::new(),
        };
        Ok((account, launch, instructions))
    }

    /// The models and efforts `agent`'s provider offers (#553 row 34).
    #[must_use]
    pub fn choices(&self, agent: &str) -> (Vec<String>, Vec<String>) {
        let p = self.providers.iter().find(|p| p.agent == agent);
        p.map(|p| (p.models.clone(), p.efforts.clone()))
            .unwrap_or_default()
    }

    /// The words that start `agent` on `model` and `effort` (both may be empty).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a pick its provider doesn't offer.
    pub fn picks(&self, agent: &str, model: &str, effort: &str) -> Result<Vec<String>, HubError> {
        let p = self.providers.iter().find(|p| p.agent == agent);
        let words = match p {
            Some(p) => p.picks(model, effort),
            None => (model.is_empty() && effort.is_empty()).then(Vec::new),
        };
        words.ok_or_else(|| {
            HubError::Refused(format!(
                "{agent} offers no model {model:?} or effort {effort:?}"
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(signed_in: bool, line: &str) -> Status {
        Status {
            signed_in,
            unknown: false,
            line: line.to_owned(),
            install: None,
            who: Some("o@example.com".to_owned()),
            usage: None,
        }
    }

    /// #106: a check that hangs runs once at a time; each timeout doubles the interval up to 80 minutes, a card
    /// opening doesn't cut that short, and a result resets it to 10 minutes.
    #[test]
    fn a_hanging_check_starts_once_per_growing_interval() {
        let min = 60_000;
        let timeout = Status::unknown("can't tell: the status command timed out");
        let mut c = Check::default();
        let mut t = 0;
        let mut started = Vec::new();
        for _ in 0..6 {
            while !c.start(t) {
                c.ask(t);
                t += 1000;
            }
            started.push(t / min);
            // It hangs for 10 s: nothing else starts, asked for or not.
            for _ in 0..10 {
                c.ask(t);
                t += 1000;
                assert!(!c.start(t));
            }
            c.busy = false;
            c.settle(timeout.clone(), t);
        }
        // Each start is 10 s after the last ended: 20, 40, then 80 minutes apart.
        assert_eq!(started, [0, 20, 60, 140, 220, 300]);
        assert_eq!(c.every, 80 * min);

        assert!(c.start(t + 80 * min));
        c.busy = false;
        c.settle(status(true, "signed in"), t + 80 * min);
        assert_eq!(c.every, 10 * min);
        // Healthy: a card opening checks again once 30 s have passed.
        c.ask(t + 80 * min + 29_000);
        assert!(!c.start(t + 80 * min + 29_000));
        c.ask(t + 80 * min + 30_000);
        assert!(c.start(t + 80 * min + 30_000));
    }

    /// #106: a timeout leaves a signed-in account signed in, as unknown; never signed out.
    #[test]
    fn a_timeout_keeps_a_signed_in_account_signed_in_as_unknown() {
        let mut c = Check::default();
        assert_eq!(c.known(0), (Status::unknown("not checked yet"), None));
        c.settle(status(true, "signed in"), 1000);
        let timeout = Status::unknown("can't tell: the status command timed out");
        c.settle(timeout.clone(), 5000);
        c.settle(timeout, 9000);
        let (s, age) = c.known(12_000);
        assert!(s.signed_in && s.unknown, "{s:?}");
        assert_eq!(s.line, "can't tell: the status command timed out");
        assert_eq!(s.who.as_deref(), Some("o@example.com"));
        assert_eq!(age, Some(3));
        c.settle(status(false, "Not signed in"), 20_000);
        assert!(!c.known(20_000).0.signed_in && !c.known(20_000).0.unknown);
    }
}
