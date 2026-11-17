use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Mutex},
    thread,
};

use cooee_core::{Agent, Body, Change, Chat, Thread, shed_proto};

use super::{AGENTS_FOR, Hub, HubError, LABEL, Live, Recipe, lock, now, on_machine, refused};

impl Hub {
    /// The installed ACP agents, as last checked.
    ///
    /// # Errors
    /// Why `shed agents` failed, when it did.
    pub fn agents(&self) -> Result<Vec<Agent>, String> {
        lock(&self.agents).0.clone()
    }

    pub(super) fn refresh(&self, at: u64, now: bool) {
        if !now && at.saturating_sub(lock(&self.agents).1) < AGENTS_FOR {
            return;
        }
        let fresh = self.shed.agents().map_err(|e| e.to_string());
        let mut agents = lock(&self.agents);
        agents.1 = at;
        match fresh {
            Err(e) if agents.0.is_ok() => cooee_core::note!("cooee: kept the agent list: {e}"),
            fresh => agents.0 = fresh,
        }
    }

    pub(super) fn adapter(&self, id: &str) -> Vec<String> {
        let find = || {
            let agents = self.agents().ok()?;
            agents.into_iter().find(|a| a.id == id).map(|a| a.cmd)
        };
        find()
            .or_else(|| {
                self.refresh(now(), true);
                find()
            })
            .unwrap_or_default()
    }

    /// Starts a thread: ACP agent `agent` in `folder` (inside a root; empty: its own scratch folder) as a new shed terminal, as `recipe` says, then
    /// sends `text`. Returns the thread's id. A retry with the same `random_id` (after a timeout, or a crash) adopts
    /// the thread it started and never starts another.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a folder outside the roots or an empty message; [`HubError::Shed`] when the
    /// terminal can't start.
    pub fn create(
        &self,
        agent: &str,
        folder: &Path,
        text: &str,
        random_id: &str,
        recipe: &Recipe,
    ) -> Result<String, HubError> {
        let folder = self.place(folder)?;
        self.start(agent, &folder, text, random_id, recipe)
    }

    /// [`Hub::create`] in a folder of cooee's own (a plugin's checkout), not checked against the roots.
    ///
    /// # Errors
    /// As [`Hub::create`].
    pub fn start(
        &self,
        agent: &str,
        folder: &Path,
        text: &str,
        random_id: &str,
        recipe: &Recipe,
    ) -> Result<String, HubError> {
        let started = self.start_in(agent, folder, (text, random_id), recipe);
        if let Some((file, _)) = &recipe.launch.secrets {
            fs::remove_file(file).ok();
        }
        started
    }

    fn start_in(
        &self,
        agent: &str,
        folder: &Path,
        (text, random_id): (&str, &str),
        recipe: &Recipe,
    ) -> Result<String, HubError> {
        self.shed.hello()?;
        self.reachable(&recipe.machine)?;
        let (id, fresh) = self.admit(agent, folder, text, random_id, recipe)?;
        self.connect(&id, (fresh, false), text, random_id, recipe)?;
        Ok(id)
    }

    /// [`Hub::create`] that answers at once (#48): the thread is listed `connecting`, its first message pending,
    /// and the agent starts behind it. A start that fails leaves it `unconnected` with why (`name`, the provider's
    /// name, said for the agent's id), kept; [`Hub::retry`], or the same start again, starts it again.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a folder outside the roots or an empty message.
    pub fn create_later(
        self: &Arc<Self>,
        (agent, name): (&str, &str),
        folder: &Path,
        (text, random_id): (&str, &str),
        recipe: Recipe,
    ) -> Result<String, HubError> {
        let folder = self.place(folder)?;
        let (id, fresh) = self.admit(agent, &folder, text, random_id, &recipe)?;
        if fresh {
            let live = self.live(&id)?;
            lock(&live).pending = Some(text.to_owned());
        }
        self.retry(&id, recipe, name)?;
        Ok(id)
    }

    /// Starts thread `id`'s agent again after its start failed (none failed, or one runs: nothing), and sends its
    /// first message; as [`Hub::create_later`].
    ///
    /// # Errors
    /// [`HubError::NoThread`]; [`HubError::Store`] when the state can't be published.
    pub fn retry(self: &Arc<Self>, id: &str, recipe: Recipe, name: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (text, random_id, agent, fresh) = {
            let mut l = lock(&live);
            let Some(text) = l
                .pending
                .clone()
                .filter(|_| l.chat.turn.state != "connecting")
            else {
                return Ok(());
            };
            let fresh = l.chat.turn.state.is_empty();
            "connecting".clone_into(&mut l.chat.turn.state);
            l.chat.turn.note = None;
            let turn = l.chat.turn.clone();
            l.publish(self, vec![Change::State(turn)])?;
            (text, l.meta.random_id.clone(), l.meta.agent.clone(), fresh)
        };
        let (hub, id, name) = (Arc::clone(self), id.to_owned(), name.to_owned());
        thread::spawn(move || {
            let done = hub.shed.hello().map_err(HubError::from);
            let done = done.and_then(|()| hub.reachable(&recipe.machine));
            let done =
                done.and_then(|()| hub.connect(&id, (fresh, true), &text, &random_id, &recipe));
            if let Some((file, _)) = &recipe.launch.secrets {
                fs::remove_file(file).ok();
            }
            let mut l = lock(&live);
            match done {
                Ok(()) if l.chat.turn.state == "connecting" => {
                    "working".clone_into(&mut l.chat.turn.state);
                }
                Ok(()) => return,
                Err(e) => {
                    "unconnected".clone_into(&mut l.chat.turn.state);
                    l.chat.turn.note = Some(format!(
                        "{name} stopped: {}",
                        e.to_string().replace(&agent, &name)
                    ));
                }
            }
            let turn = l.chat.turn.clone();
            if let Err(e) = l.publish(&hub, vec![Change::State(turn)]) {
                cooee_core::note!("cooee: {e}");
            }
        });
        Ok(())
    }

    fn admit(
        &self,
        agent: &str,
        folder: &Path,
        text: &str,
        random_id: &str,
        recipe: &Recipe,
    ) -> Result<(String, bool), HubError> {
        if text.trim().is_empty() {
            return Err(HubError::Refused("the first message is empty".to_owned()));
        }
        self.offers(agent, &recipe.account, &recipe.model)?;
        let title: String = text
            .trim()
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(60)
            .collect();
        let (id, fresh) = self.mint(random_id, |id| Thread {
            terminal: on_machine(&recipe.machine, &format!("cooee-{id}")),
            id,
            title,
            agent: agent.to_owned(),
            folder: Some(folder.to_owned()).filter(|f| !f.as_os_str().is_empty()),
            account: recipe.account.clone(),
            profile: recipe.profile.clone(),
            machine: recipe.machine.clone(),
            model: recipe.model.clone(),
            effort: recipe.effort.clone(),
            random_id: random_id.to_owned(),
            room: recipe.room.clone(),
            asked_by: recipe.asked_by.clone(),
            setup: recipe.setup,
            routine: recipe.routine.clone(),
            ..Thread::default()
        })?;
        if fresh && !recipe.machine.is_empty() {
            let live = self.live(&id)?;
            let mut live = lock(&live);
            let text = format!("runs on {}", recipe.machine);
            let item = live.chat.push(now(), 0, Body::Note { text });
            live.publish(self, vec![Change::Item(item)])?;
        }
        Ok((id, fresh))
    }

    pub(super) fn reachable(&self, machine: &str) -> Result<(), HubError> {
        if machine.is_empty() {
            return Ok(());
        }
        match self.shed.link(machine) {
            Ok(Some(l)) if l.state != shed_proto::LinkState::Up => {
                let why = l.error.map(|e| format!(": {e}")).unwrap_or_default();
                let seen = cooee_machines::ago(cooee_machines::unix_now(), l.since);
                Err(HubError::Refused(format!(
                    "{machine} is offline, last seen {seen}{why}"
                )))
            }
            Ok(None) => Err(HubError::Refused(format!(
                "no machine {machine}: add it with shed machine add"
            ))),
            _ => Ok(()),
        }
    }

    pub(super) fn mint(
        &self,
        random_id: &str,
        make: impl FnOnce(String) -> Thread,
    ) -> Result<(String, bool), HubError> {
        let mut last = lock(&self.last);
        let started = self.all().into_iter().find_map(|l| {
            let l = lock(&l);
            let same = !random_id.is_empty() && l.meta.random_id == random_id;
            same.then(|| l.meta.id.clone())
        });
        if let Some(id) = started {
            return Ok((id, false));
        }
        *last = (*last + 1).max(now());
        while lock(&self.threads).contains_key(&format!("t{:x}", *last)) {
            *last += 1;
        }
        let id = format!("t{:x}", *last);
        let meta = make(id.clone());
        self.store.save(&meta)?;
        let terminal = meta.terminal.clone();
        let live = Arc::new(Mutex::new(Live::new(meta, Chat::default())));
        self.insert(id.clone(), terminal, &live);
        Ok((id, true))
    }

    fn connect(
        &self,
        id: &str,
        (fresh, keep): (bool, bool),
        text: &str,
        random_id: &str,
        recipe: &Recipe,
    ) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (terminal, agent, folder) = {
            let l = lock(&live);
            let folder = l.meta.folder.clone();
            (l.meta.terminal.clone(), l.meta.agent.clone(), folder)
        };
        let folder = if let Some(f) = folder {
            f
        } else {
            let dir = self.scratch.join(id);
            fs::create_dir_all(&dir).map_err(|e| refused(&dir, &e))?;
            dir
        };
        if fresh || (keep && self.shed.get(&terminal).is_err()) {
            let room = Some(&recipe.room).filter(|r| !r.is_empty());
            let mut labels = BTreeMap::from([
                (LABEL.to_owned(), "1".to_owned()),
                (format!("{LABEL}.thread"), room.map_or(id, |r| r).to_owned()),
            ]);
            if room.is_some() {
                labels.insert(format!("{LABEL}.member"), recipe.profile.clone());
            }
            let mut launch = self.mcp_launch(id, recipe)?;
            for (_, v) in &mut launch.env {
                *v = v.replace("{thread}", id);
            }
            if let Err(e) = self.shed.run_acp(
                &terminal,
                (&agent, &self.adapter(&agent)),
                &folder,
                &labels,
                &launch,
            ) && self.shed.get(&terminal).is_err()
            {
                if !keep {
                    self.remove(id);
                    self.store.remove(id)?;
                }
                return Err(e.into());
            }
        }
        if let Some(s) = self.shed.ready(&terminal)?
            && let Err(e) = self.settle(id, &terminal, &s)
        {
            cooee_core::note!("cooee: {id}: model: {e}");
        }
        let note = self.unconnected(recipe)?;
        let before = [recipe.instructions.trim_end(), &note].map(str::trim);
        let before = before
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        self.send_after(id, text, random_id, &[], &before.join("\n\n"))?;
        Ok(())
    }
}
