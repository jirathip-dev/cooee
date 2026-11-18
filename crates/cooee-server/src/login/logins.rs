use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use cooee_core::{Account, Accounts, Provider, Providers, Shed};

use super::checks::Check;
use crate::hub::HubError;

pub(super) const LABEL: &str = "cooee-login";

pub(super) enum Login {
    Running(String),
    Ended(String, String),
}

/// The providers, their accounts and logins.
pub struct Logins {
    pub(super) shed: Shed,
    pub(super) providers: Providers,
    pub(super) accounts: Accounts,
    pub(super) live: Mutex<BTreeMap<String, Login>>,
    pub(super) checks: Mutex<BTreeMap<(String, String), Check>>,
}

impl Logins {
    /// The providers' logins on `shed`, their accounts in `data`; removes login terminals a previous run left behind.
    #[must_use]
    pub fn new(shed: Shed, providers: Providers, data: &Path) -> Arc<Self> {
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
                cooee_core::clock::sleep(Duration::from_secs(1));
            }
        });
        let shed = logins.shed.clone();
        thread::spawn(move || {
            let labels = BTreeMap::from([(LABEL.to_owned(), "1".to_owned())]);
            for t in shed.ls(labels).unwrap_or_default() {
                shed.close(&t.name).ok();
            }
        });
        logins
    }

    pub(super) fn provider(&self, name: &str) -> Result<&Arc<dyn Provider>, HubError> {
        self.providers
            .named(name)
            .ok_or_else(|| HubError::Refused(format!("no provider {name}")))
    }

    pub(super) fn account(&self, provider: &str, name: &str) -> Result<Account, HubError> {
        self.accounts
            .get(provider, name)
            .ok_or_else(|| HubError::Refused(format!("{provider} has no account {name}")))
    }
}
