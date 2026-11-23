use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

use cooee_core::{
    Provider, Status, links, redact,
    shed_proto::{RunParams, WaitOutcome},
};
use serde::Serialize;

use super::{
    Logins,
    checks::Check,
    logins::{LABEL, Login},
};
use crate::hub::{HubError, lock, now};

/// How long a login terminal may run, in seconds.
pub const LOGIN_TIMEOUT: f64 = 600.0;

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

fn login_run(
    name: &str,
    provider: &Arc<dyn Provider>,
    home: Option<&Path>,
    machine: Option<&str>,
) -> RunParams {
    RunParams {
        name: Some(format!("{LABEL}-{name}-{:x}", now())),
        labels: BTreeMap::from([(LABEL.to_owned(), "1".to_owned())]),
        cwd: cooee_core::env_var("HOME")
            .filter(|_| machine.is_none())
            .map_or_else(|| PathBuf::from("/"), PathBuf::from),
        keep: false,
        idle_after: None,
        cmd: provider.with_home(&provider.spec().login, home),
        acp: None,
        policy: None,
        resume: None,
        mcp_servers: Vec::new(),
        meta: None,
        no_hibernate: false,
        machine: machine.map(str::to_owned),
        parent: None,
        rows: None,
        cols: None,
    }
}

impl Logins {
    fn running(&self, name: &str) -> Option<String> {
        match lock(&self.live).get(name) {
            Some(Login::Running(t)) => Some(t.clone()),
            _ => None,
        }
    }

    /// Starts provider `name`'s login command in a new shed terminal (a running one is kept): for account
    /// `account` (none: the system default), or with `new` in a new account's folder; on `machine` (#150; none:
    /// this host), where its tokens stay.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider or account, or `new` for a provider without a config
    /// variable or on another machine; [`HubError::Shed`] when the terminal can't start.
    pub fn start(
        self: &Arc<Self>,
        name: &str,
        (account, new): (Option<&str>, bool),
        machine: Option<String>,
    ) -> Result<(), HubError> {
        let provider = self.provider(name)?;
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(_))) {
            return Ok(());
        }
        if new && machine.is_some() {
            return Err(HubError::Refused(
                "a new account signs in on this host".to_owned(),
            ));
        }
        let (account, home) = if new {
            if provider.spec().home.is_empty() {
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
        let run = login_run(name, provider, home.as_deref(), machine.as_deref());
        let terminal = match self.shed.run(&run) {
            Ok(t) => t.name,
            Err(e) => {
                if new {
                    self.accounts.drop_pending(name, &account);
                }
                return Err(e.into());
            }
        };
        self.shed.resize(&terminal, 40, 500).ok();
        live.insert(name.to_owned(), Login::Running(terminal.clone()));
        let logins = Arc::clone(self);
        let name = name.to_owned();
        thread::spawn(move || match machine {
            Some(m) => logins.reap_on(&m, &name, &terminal, &account),
            None => logins.reap(&name, &terminal, &account, new, home.as_deref()),
        });
        Ok(())
    }

    fn ended(&self, terminal: &str) -> (bool, String) {
        let ended = match self.shed.exited(terminal, LOGIN_TIMEOUT) {
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
            cooee_core::note!("cooee: {terminal}: {e}");
        }
        ended
    }

    fn reap_on(&self, machine: &str, name: &str, terminal: &str, account: &str) {
        let (_, why) = self.ended(terminal);
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(t)) if t == terminal) {
            let why = format!("{why} on {machine}");
            live.insert(name.to_owned(), Login::Ended(why, account.to_owned()));
        }
    }

    fn reap(&self, name: &str, terminal: &str, account: &str, new: bool, home: Option<&Path>) {
        let (ok, mut why) = self.ended(terminal);
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
        let moved =
            |p: &Arc<dyn Provider>| Some(p.check(self.accounts.get(name, account)?.dir.as_deref()));
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
        let key = (provider.spec().name.clone(), account);
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
}
