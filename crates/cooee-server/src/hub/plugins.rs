use std::{collections::HashMap, path::Path, sync::Mutex};

use cooee_core::{Plugin, Profile};

use crate::plugin;

use super::{Hub, HubError, Recipe, Seed, lock, now};

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

    pub(super) fn backing(&self, profile: &str) -> Option<(String, String)> {
        let own = || cooee_core::agents::find(self.config_dir(), &self.data, profile).ok();
        let backing = if profile.contains('/') {
            Some(profile.to_owned())
        } else {
            own().and_then(|a| a.profile)
        };
        let (name, p) = backing.as_deref()?.split_once('/')?;
        Some((name.to_owned(), p.to_owned()))
    }

    /// Where a chat of `profile` attaches to its plugin; none when it doesn't.
    #[must_use]
    pub fn attaches(&self, profile: &str) -> Option<String> {
        let (name, _) = self.backing(profile)?;
        let threads = self.with_rows(&name)?.threads?;
        (!threads.attach.is_empty()).then_some(threads.attach)
    }

    pub(super) fn attached(
        &self,
        id: &str,
        agent: &str,
        recipe: &Recipe,
    ) -> Result<Recipe, HubError> {
        let mut recipe = recipe.clone();
        let meta = lock(&*self.live(id)?).meta.clone();
        let Some(at) = &meta.attach else {
            return Ok(recipe);
        };
        let refused = |why: String| HubError::Refused(format!("attach: {why}"));
        let path = self
            .attaches(&meta.profile)
            .ok_or_else(|| refused("its agent's plugin attaches no chat".to_owned()))?;
        let name = self
            .backing(&meta.profile)
            .map(|(n, _)| n)
            .unwrap_or_default();
        let p = self
            .backend(&name)
            .ok_or_else(|| refused(format!("plugin {name} is down")))?;
        let a = plugin::attach(&p, &path, &at.repo, meta.plugin_thread())
            .map_err(|e| refused(e.to_string()))?;
        if !a.ok {
            return Err(refused(format!("{}: {}", a.code, a.message)));
        }
        recipe.launch.env.retain(|(k, _)| !a.env.contains_key(k));
        recipe.launch.env.extend(a.env);
        if !a.settings.is_empty() {
            let key = self
                .providers
                .for_agent(agent)
                .map(|p| p.spec().settings.as_str());
            let key = key
                .filter(|k| !k.is_empty())
                .ok_or_else(|| refused(format!("{agent} takes no settings")))?;
            let meta = recipe.launch.meta.take();
            let with = crate::login::with_settings(meta, key, Path::new(&a.settings));
            recipe.launch.meta = Some(with.map_err(refused)?);
        }
        let brief = [recipe.instructions.trim(), a.brief.trim()];
        brief
            .join("\n\n")
            .trim()
            .clone_into(&mut recipe.instructions);
        Ok(recipe)
    }

    pub(super) fn report_start(&self, thread: &str, profile: &str) {
        let Some((name, p)) = self.backing(profile) else {
            return;
        };
        if let Some(b) = self.backend(&name)
            && let Err(e) = plugin::started(&b, thread, &p)
        {
            cooee_core::note!("cooee: plugin {name}: thread-started: {e}");
        }
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
