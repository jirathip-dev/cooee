use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use cooee_core::{
    Calls as _, Provider, Providers, Shed, Spec,
    shed_proto::{Account, AccountAdd},
};

use crate::hub::HubError;

pub(super) enum Login {
    Running(String),
    Ended(String, Option<Box<Account>>),
}

/// A provider as the sign-ins sheet shows it: its accounts as shed last read them, `default` first.
#[derive(Debug, Clone)]
pub struct Sheet {
    /// The provider.
    pub provider: Spec,
    /// Its accounts (`account.ls`).
    pub accounts: Vec<Account>,
}

/// The providers and their logins; shed keeps the accounts.
pub struct Logins {
    pub(super) shed: Shed,
    pub(super) providers: Providers,
    pub(super) live: Mutex<BTreeMap<String, Login>>,
}

impl Logins {
    /// The providers' logins on `shed`, which adopts an older cooee's `data/accounts` in place.
    #[must_use]
    pub fn new(shed: Shed, providers: Providers, data: &Path) -> Arc<Self> {
        let logins = Arc::new(Self {
            shed,
            providers,
            live: Mutex::new(BTreeMap::new()),
        });
        let (adopting, data) = (Arc::clone(&logins), data.join("accounts"));
        thread::spawn(move || adopting.adopt(&data));
        logins
    }

    fn adopt(&self, data: &Path) {
        for p in self.providers.iter() {
            let provider = &p.spec().name;
            let mut found: BTreeMap<String, AccountAdd> = BTreeMap::new();
            for e in fs::read_dir(data.join(provider))
                .into_iter()
                .flatten()
                .flatten()
            {
                let Ok(file) = e.file_name().into_string() else {
                    continue;
                };
                let (name, token) = file
                    .strip_suffix(".token")
                    .map_or((file.as_str(), false), |n| (n, true));
                let path = e.path();
                if name.starts_with('.') || !(token || path.is_dir()) {
                    continue;
                }
                let add = found.entry(name.to_owned()).or_insert_with(|| AccountAdd {
                    provider: provider.clone(),
                    name: name.to_owned(),
                    ..AccountAdd::default()
                });
                if token {
                    add.token_file = Some(path);
                } else {
                    add.folder = Some(fs::read_link(&path).unwrap_or(path));
                }
            }
            for add in found.values() {
                if let Err(e) = self.shed.account_add(add) {
                    cooee_core::note!("cooee: handing {provider}/{} to shed: {e}", add.name);
                }
            }
        }
    }

    /// Every provider with its accounts as shed last read them.
    #[must_use]
    pub fn sheets(&self) -> Vec<Sheet> {
        let all = self.shed.accounts(None).unwrap_or_else(|e| {
            cooee_core::note!("cooee: account.ls: {e}");
            Vec::new()
        });
        let sheet = |p: &Arc<dyn Provider>| Sheet {
            provider: p.spec().clone(),
            accounts: (all.iter().filter(|a| a.provider == p.spec().name).cloned()).collect(),
        };
        self.providers.iter().map(sheet).collect()
    }

    /// The name of ACP agent `agent`'s provider, else the agent's own without `-acp`.
    #[must_use]
    pub fn provider_name(&self, agent: &str) -> String {
        self.providers.for_agent(agent).map_or_else(
            || agent.strip_suffix("-acp").unwrap_or(agent).to_owned(),
            |p| p.spec().name.clone(),
        )
    }

    pub(super) fn provider(&self, name: &str) -> Result<&Arc<dyn Provider>, HubError> {
        self.providers
            .named(name)
            .ok_or_else(|| HubError::Refused(format!("no provider {name}")))
    }
}
