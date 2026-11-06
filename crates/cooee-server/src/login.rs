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
};

use cooee_core::{
    Account, Accounts, Provider, Shed, Sheet, Status, links, redact,
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
    /// The status of the account it was for (else the default), once no login runs.
    pub status: Option<Status>,
}

/// The providers, their accounts and logins.
pub struct Logins {
    shed: Shed,
    providers: Vec<Provider>,
    accounts: Accounts,
    live: Mutex<BTreeMap<String, Login>>,
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

    /// Every provider with its accounts' status, checked in parallel.
    #[must_use]
    pub fn statuses(&self) -> Vec<Sheet> {
        thread::scope(|s| {
            let checks: Vec<_> = self
                .providers
                .iter()
                .map(|p| {
                    let accounts = self.accounts.list(&p.name).into_iter().map(|a| {
                        s.spawn(move || {
                            let status = p.check(a.dir.as_deref());
                            (a, status)
                        })
                    });
                    (p, accounts.collect::<Vec<_>>())
                })
                .collect();
            checks
                .into_iter()
                .map(|(p, accounts)| {
                    let mut accounts: Vec<_> =
                        accounts.into_iter().filter_map(|c| c.join().ok()).collect();
                    if p.missing() {
                        accounts.truncate(1);
                    }
                    let why = match lock(&self.live).get(&p.name) {
                        Some(Login::Ended(why, _)) => Some(why.clone()),
                        _ => None,
                    };
                    Sheet {
                        provider: p.clone(),
                        accounts,
                        why,
                    }
                })
                .collect()
        })
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
        let signed_in = ok && self.provider(name).is_ok_and(|p| p.check(home).signed_in);
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
            });
        }
        let (why, account) = match lock(&self.live).get(name) {
            Some(Login::Ended(why, account)) => (Some(why.clone()), account.clone()),
            _ => (None, String::new()),
        };
        let account = self
            .accounts
            .get(name, &account)
            .unwrap_or_else(|| self.accounts.default(name));
        Ok(View {
            state: if why.is_some() { "ended" } else { "none" },
            screen: String::new(),
            links: Vec::new(),
            why,
            status: Some(provider.check(account.dir.as_deref())),
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
        if out.is_ok() || a.dir.is_some() {
            self.accounts
                .remove(name, account)
                .map_err(|e| HubError::Refused(e.to_string()))?;
        }
        out.map_err(|e| HubError::Refused(format!("{} sign-out: {e}", a.label())))
    }

    /// Makes `account` provider `name`'s default for new threads.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown account, or when it can't be saved.
    pub fn set_default(&self, name: &str, account: &str) -> Result<(), HubError> {
        self.provider(name)?;
        self.account(name, account)?;
        self.accounts
            .set_default(name, account)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    /// What a thread of ACP agent `agent` runs with: its account's label (empty for an agent no provider signs
    /// in), and the variable and config folder (none for the system default). `account` none: the default.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an account the agent's provider doesn't have.
    pub fn home(
        &self,
        agent: &str,
        account: Option<&str>,
    ) -> Result<(String, Option<(String, PathBuf)>), HubError> {
        let Some(p) = self.providers.iter().find(|p| p.agent == agent) else {
            return Ok((String::new(), None));
        };
        let a = match account {
            Some(name) => self.account(&p.name, name)?,
            None => self.accounts.default(&p.name),
        };
        let env = p
            .env(a.dir.as_deref())
            .map(|(var, dir)| (var.to_owned(), dir.to_owned()));
        Ok((a.label().to_owned(), env))
    }
}
