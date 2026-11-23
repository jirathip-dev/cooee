use std::{collections::HashMap, sync::Mutex};

use cooee_core::{Plugin, Profile};

use crate::plugin;

use super::{Hub, Seed, lock, now};

pub(super) struct State {
    plugins: Mutex<Vec<Plugin>>,
    served: Mutex<HashMap<String, Option<Vec<Profile>>>>,
}

impl State {
    pub(super) fn open(seed: &Seed<'_>) -> Self {
        Self {
            plugins: Mutex::new(seed.plugins.clone()),
            served: Mutex::default(),
        }
    }
}

impl Hub {
    /// Installed plugin `name`'s backend: its first tab's entry (its URL and token).
    #[must_use]
    pub fn backend(&self, name: &str) -> Option<Plugin> {
        let lock = cooee_core::plugins::read_lock(&self.data).ok()?;
        let tabs = lock.into_iter().find(|l| l.name == name)?.tabs;
        self.plugins().into_iter().find(|p| tabs.contains(&p.name))
    }

    /// Installed plugin `name`'s profiles as its backend last served them: `None` while no backend runs or when it
    /// serves none (its manifest's are offered), none before its first answer.
    #[must_use]
    pub fn served(&self, name: &str) -> Option<Vec<Profile>> {
        self.backend(name)?;
        lock(&self.areas.plugins.served)
            .get(name)
            .cloned()
            .unwrap_or(Some(Vec::new()))
    }

    pub(super) fn refresh_served(&self) {
        let mut fresh = HashMap::new();
        let installed = cooee_core::plugins::read_lock(&self.data).unwrap_or_default();
        for name in installed.into_iter().filter(|l| l.enabled).map(|l| l.name) {
            let Some(p) = self.backend(&name) else {
                continue;
            };
            let got = plugin::profiles(&p).or_else(|e| {
                cooee_core::note!("cooee: plugin {}: its profiles: {e}", p.name);
                lock(&self.areas.plugins.served)
                    .get(&name)
                    .cloned()
                    .ok_or(e)
            });
            if let Ok(list) = got {
                fresh.insert(name, list);
            }
        }
        *lock(&self.areas.plugins.served) = fresh;
        self.surface(now());
    }

    /// The plugins: the config's own and the installed ones' tabs.
    #[must_use]
    pub fn plugins(&self) -> Vec<Plugin> {
        lock(&self.areas.plugins.plugins).clone()
    }

    /// The installed plugin `[[plugins]]` entry `tab` is a tab of: its lock entry's name, else its own (a config's).
    #[must_use]
    pub fn plugin_of(&self, tab: &str) -> String {
        let lock = cooee_core::plugins::read_lock(&self.data).unwrap_or_default();
        let found = lock.into_iter().find(|l| l.tabs.iter().any(|t| t == tab));
        found.map_or_else(|| tab.to_owned(), |l| l.name)
    }

    /// The plugins a persona may be attached to: the installed ones (on or off) and the config's.
    #[must_use]
    pub fn installed(&self) -> Vec<String> {
        let lock = cooee_core::plugins::read_lock(&self.data).unwrap_or_default();
        let mut names: Vec<String> = lock.into_iter().map(|l| l.name).collect();
        names.extend(self.plugins().iter().map(|p| self.plugin_of(&p.name)));
        names
    }

    /// Replaces the plugins (the supervisor, as plugins are installed or removed).
    pub fn set_plugins(&self, plugins: Vec<Plugin>) {
        *lock(&self.areas.plugins.plugins) = plugins;
    }
}
