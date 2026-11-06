//! Provider logins: a provider's own login command in a shed terminal (terminal mode), its screen read live for
//! the page (redacted, never stored or logged), typed codes, and the terminal killed and removed when the command
//! exits, is cancelled or after 10 minutes.

use std::{
    collections::BTreeMap,
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use cooee_core::{
    Provider, Shed, Status, links, redact,
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
    /// It ended, and why.
    Ended(String),
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
    /// The provider's status, once no login runs.
    pub status: Option<Status>,
}

/// The providers and their logins.
pub struct Logins {
    shed: Shed,
    providers: Vec<Provider>,
    live: Mutex<BTreeMap<String, Login>>,
}

impl Logins {
    /// The providers' logins on `shed`; removes login terminals a previous run left behind.
    #[must_use]
    pub fn new(shed: Shed, providers: Vec<Provider>) -> Arc<Self> {
        let logins = Arc::new(Self {
            shed,
            providers,
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

    fn running(&self, name: &str) -> Option<String> {
        match lock(&self.live).get(name) {
            Some(Login::Running(t)) => Some(t.clone()),
            _ => None,
        }
    }

    /// Every provider with its status, checked in parallel.
    #[must_use]
    pub fn statuses(&self) -> Vec<(Provider, Status)> {
        thread::scope(|s| {
            let checks: Vec<_> = self
                .providers
                .iter()
                .map(|p| s.spawn(|| (p.clone(), p.check())))
                .collect();
            checks.into_iter().filter_map(|c| c.join().ok()).collect()
        })
    }

    /// Starts provider `name`'s login command in a new shed terminal (a running one is kept).
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown provider; [`HubError::Shed`] when the terminal can't start.
    pub fn start(self: &Arc<Self>, name: &str) -> Result<(), HubError> {
        let provider = self.provider(name)?;
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(_))) {
            return Ok(());
        }
        let terminal = format!("{LABEL}-{name}-{:x}", now());
        let run = RunParams {
            name: Some(terminal.clone()),
            labels: BTreeMap::from([(LABEL.to_owned(), "1".to_owned())]),
            cwd: env::var_os("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from),
            keep: false,
            idle_after: None,
            cmd: provider.login.clone(),
            acp: None,
        };
        self.shed.run(&run)?;
        // Wide, so a long sign-in link stays on one line.
        let _ = self.shed.resize(&terminal, 40, 500);
        live.insert(name.to_owned(), Login::Running(terminal.clone()));
        let (logins, name) = (Arc::clone(self), name.to_owned());
        thread::spawn(move || logins.reap(&name, &terminal));
        Ok(())
    }

    /// Waits for login terminal `terminal` to exit (or for the timeout), then kills and removes it.
    fn reap(&self, name: &str, terminal: &str) {
        let why = match self.shed.exited(terminal, LOGIN_TIMEOUT) {
            Ok(r) if r.outcome == WaitOutcome::Timeout => "timed out after 10 minutes".to_owned(),
            Ok(r) => match r.terminal.exit {
                Some(0) => "the login command finished".to_owned(),
                Some(code) => format!("the login command exited {code}"),
                None => "the login command ended".to_owned(),
            },
            Err(e) => e.to_string(),
        };
        if let Err(e) = self.shed.close(terminal) {
            eprintln!("cooee: {terminal}: {e}");
        }
        let mut live = lock(&self.live);
        if matches!(live.get(name), Some(Login::Running(t)) if t == terminal) {
            live.insert(name.to_owned(), Login::Ended(why));
        }
    }

    /// Provider `name`'s login: its screen while it runs, else how it ended and the provider's status.
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
        let why = match lock(&self.live).get(name) {
            Some(Login::Ended(why)) => Some(why.clone()),
            _ => None,
        };
        Ok(View {
            state: if why.is_some() { "ended" } else { "none" },
            screen: String::new(),
            links: Vec::new(),
            why,
            status: Some(provider.check()),
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
        lock(&self.live).insert(name.to_owned(), Login::Ended("cancelled".to_owned()));
        Ok(())
    }
}
