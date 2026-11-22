use std::{sync::Arc, thread};

use cooee_core::{
    links,
    shed_proto::{Account, AccountAdd, AccountState, WaitOutcome},
};
use serde::Serialize;

use super::{Logins, logins::Login};
use crate::hub::{HubError, lock};

/// How long a login terminal may run, in seconds (shed ends it then).
pub const LOGIN_TIMEOUT: f64 = 600.0;

const DEFAULT: &str = "default";

/// A provider's login as the page sees it.
#[derive(Debug, Serialize)]
pub struct View {
    /// `running`, `ended` (`why` says how) or `none`.
    pub state: &'static str,
    /// The terminal's screen, as shed redacts it, while it runs.
    pub screen: String,
    /// The links on the screen.
    pub links: Vec<String>,
    /// How the last login ended.
    pub why: Option<String>,
    /// The account it was for, as `account.status` read it once the login ended.
    pub account: Option<Account>,
}

impl Logins {
    fn running(&self, name: &str) -> Option<String> {
        match lock(&self.live).get(name) {
            Some(Login::Running(t)) => Some(t.clone()),
            _ => None,
        }
    }

    /// Starts provider `name`'s login in shed's redacted terminal (a running one is kept): for account `account`
    /// (none: `default`), or with `new` for a new account shed makes.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider; [`HubError::Shed`] when shed refuses the account or the login.
    pub fn start(
        self: &Arc<Self>,
        name: &str,
        account: Option<&str>,
        new: bool,
    ) -> Result<(), HubError> {
        self.provider(name)?;
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(..))) {
            return Ok(());
        }
        let account = match account.filter(|a| !a.is_empty()) {
            _ if new => self.add(name)?,
            Some(a) => a.to_owned(),
            None => DEFAULT.to_owned(),
        };
        let terminal = match self.shed.account_login(name, &account) {
            Ok(t) => t,
            Err(e) => {
                if new {
                    self.shed.account_rm(name, &account).ok();
                }
                return Err(e.into());
            }
        };
        self.shed.resize(&terminal, 40, 500).ok();
        live.insert(name.to_owned(), Login::Running(terminal.clone()));
        let logins = Arc::clone(self);
        let name = name.to_owned();
        thread::spawn(move || logins.reap(&name, &terminal, &account, new));
        Ok(())
    }

    fn add(&self, name: &str) -> Result<String, HubError> {
        let taken: Vec<String> = (self.shed.accounts(Some(name))?.into_iter())
            .map(|a| a.name)
            .collect();
        let account = (2..=u16::MAX)
            .map(|i| format!("account-{i}"))
            .find(|n| !taken.contains(n))
            .ok_or_else(|| HubError::Refused("no free account name".to_owned()))?;
        self.shed.account_add(&AccountAdd {
            provider: name.to_owned(),
            name: account.clone(),
            ..AccountAdd::default()
        })?;
        Ok(account)
    }

    fn reap(&self, name: &str, terminal: &str, account: &str, new: bool) {
        let ended = match self.shed.exited(terminal, LOGIN_TIMEOUT + 30.0) {
            Ok(r) if r.outcome == WaitOutcome::Timeout => "timed out after 10 minutes".to_owned(),
            Ok(r) => match r.terminal.exit {
                Some(0) => "the login command finished".to_owned(),
                Some(code) => format!("the login command exited {code}"),
                None => "the login command ended".to_owned(),
            },
            Err(e) => e.to_string(),
        };
        self.shed.close(terminal).ok();
        let status = self.shed.account_status(name, account);
        let ready = matches!(&status, Ok(a) if a.state != AccountState::SignedOut);
        let why = match &status {
            Err(e) => format!("{account}: {e}"),
            Ok(a) if !ready && ended == "the login command finished" => {
                format!("{ended}, but the account isn't signed in: {}", a.line)
            }
            Ok(_) => ended,
        };
        if new && !ready {
            self.shed.account_rm(name, account).ok();
        }
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(t)) if t == terminal) {
            let account = status.ok().filter(|_| ready || !new).map(Box::new);
            live.insert(name.to_owned(), Login::Ended(why, account));
        }
    }

    /// Provider `name`'s login: its screen while it runs, else how it ended and its account.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider.
    pub fn view(&self, name: &str) -> Result<View, HubError> {
        self.provider(name)?;
        if let Some(screen) = self.running(name).and_then(|t| self.shed.screen(&t).ok()) {
            return Ok(View {
                state: "running",
                links: links(&screen),
                screen,
                why: None,
                account: None,
            });
        }
        let (why, account) = match lock(&self.live).get(name) {
            Some(Login::Ended(why, account)) => (Some(why.clone()), account.as_deref().cloned()),
            _ => (None, None),
        };
        Ok(View {
            state: if why.is_some() { "ended" } else { "none" },
            screen: String::new(),
            links: Vec::new(),
            why,
            account,
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
        let why = Login::Ended("cancelled".to_owned(), None);
        lock(&self.live).insert(name.to_owned(), why);
        Ok(())
    }

    /// Signs provider `name`'s account `account` out and removes it (`account.rm`; shed keeps `default`).
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider; [`HubError::Shed`] when shed refuses.
    pub fn sign_out(&self, name: &str, account: &str) -> Result<(), HubError> {
        self.provider(name)?;
        Ok(self.shed.account_rm(name, account)?)
    }

    /// Reads every account's status again (the sign-ins card opened), in the background.
    pub fn ask(self: &Arc<Self>) {
        let logins = Arc::clone(self);
        thread::spawn(move || {
            for s in logins.sheets() {
                for a in s.accounts {
                    if let Err(e) = logins.shed.account_status(&a.provider, &a.name) {
                        cooee_core::note!("cooee: account.status {}/{}: {e}", a.provider, a.name);
                    }
                }
            }
        });
    }
}
