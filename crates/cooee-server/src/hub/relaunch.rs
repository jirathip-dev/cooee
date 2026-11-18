use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use cooee_core::history;

use super::{
    Body, Change, Hub, HubError, Item, LABEL, Recipe, lock, now, on_machine, outside, short,
};

impl Hub {
    /// "Use a folder…" (#86): thread `id`'s later turns run in `folder` (inside a root), as `relaunch` says.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a folder outside the roots; as `relaunch`.
    pub fn attach(&self, id: &str, folder: &Path, recipe: &Recipe) -> Result<Item, HubError> {
        let folder = self.allowed(folder).ok_or_else(|| outside(folder))?;
        let l = self.live(id)?;
        let (machine, agent) = {
            let l = lock(&l);
            (l.meta.machine.clone(), l.meta.agent.clone())
        };
        self.relaunch(id, (&folder, &machine), (&agent, recipe), None)
    }

    /// Moves thread `id` to `machine` (#151; empty: this host), in the same folder, as `relaunch` says.
    ///
    /// # Errors
    /// [`HubError::Refused`] while a turn is under way (stop it first); as `relaunch`.
    pub fn move_to(&self, id: &str, machine: &str, recipe: &Recipe) -> Result<Item, HubError> {
        let (folder, _, agent) = self.idle(id)?;
        self.relaunch(id, (&folder, machine), (&agent, recipe), None)
    }

    /// Switches thread `id` to ACP agent `agent` (#503), set up by `recipe`: a new session gets its brief and a handoff
    /// of the thread so far, as `relaunch` says; the thread keeps its id.
    ///
    /// # Errors
    /// [`HubError::Refused`] while a turn is under way, or when it already runs on `agent`; as `relaunch`.
    pub fn switch(&self, id: &str, agent: &str, recipe: &Recipe) -> Result<Item, HubError> {
        let (folder, machine, was) = self.idle(id)?;
        if was == agent {
            return Err(HubError::Refused(format!("it already runs on {agent}")));
        }
        let handoff = history::handoff(&self.store, id, 4_000)?;
        self.relaunch(id, (&folder, &machine), (agent, recipe), Some(&handoff))
    }

    fn idle(&self, id: &str) -> Result<(PathBuf, String, String), HubError> {
        let live = self.live(id)?;
        let l = lock(&live);
        if matches!(
            l.chat.turn.state.as_str(),
            "working" | "connecting" | "blocked"
        ) {
            return Err(HubError::Refused(
                "a turn is under way: stop it first".to_owned(),
            ));
        }
        let folder = l.meta.folder.clone();
        Ok((
            folder.unwrap_or_else(|| self.scratch.join(id)),
            l.meta.machine.clone(),
            l.meta.agent.clone(),
        ))
    }

    fn relaunch(
        &self,
        id: &str,
        (folder, machine): (&Path, &str),
        (agent, recipe): (&str, &Recipe),
        handoff: Option<&str>,
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let old = {
            let l = lock(&live);
            if l.meta.closed || l.meta.workflow.is_some() {
                return Err(HubError::Refused(
                    "only an open chat of your own can move".to_owned(),
                ));
            }
            l.meta.terminal.clone()
        };
        let terminal = on_machine(machine, &format!("cooee-{id}-{:x}", now()));
        let labels = BTreeMap::from([
            (LABEL.to_owned(), "1".to_owned()),
            (format!("{LABEL}.thread"), id.to_owned()),
        ]);
        let started = self
            .mcp_launch(id, recipe)
            .and_then(|launch| {
                let cmd = self.adapter(agent);
                let run = (agent, cmd.as_slice());
                Ok(self
                    .shed
                    .run_acp(&terminal, run, folder, &labels, &launch)?)
            })
            .and_then(|_| Ok(self.shed.ready(&terminal)?));
        if let Some((file, _)) = &recipe.launch.secrets {
            fs::remove_file(file).ok();
        }
        let session = started?;
        let mut live = lock(&live);
        let was = live
            .meta
            .folder
            .clone()
            .unwrap_or_else(|| self.scratch.join(id));
        let place = |f: &Path, m: &str| match m {
            "" => f.display().to_string(),
            m => format!("{} on {m}", f.display()),
        };
        let story = handoff.map_or_else(
            || {
                format!(
                    "This chat moved from {} to {}, where you now run; what it made before is still there. Its last turns:\n\n{}",
                    place(&was, &live.meta.machine),
                    place(folder, machine),
                    last_turns(&live.chat.items)
                )
            },
            str::to_owned,
        );
        let handover = format!("{}\n\n{story}", recipe.instructions.trim_end());
        handover.trim_start().clone_into(&mut live.meta.handover);
        live.meta.terminal.clone_from(&terminal);
        {
            let mut terminals = lock(&self.terminals);
            let me = terminals.remove(&old);
            terminals.extend(me.map(|l| (terminal.clone(), l)));
        }
        let text = if handoff.is_some() {
            agent.clone_into(&mut live.meta.agent);
            live.meta.account.clone_from(&recipe.account);
            (live.meta.model, live.meta.effort) = (recipe.model.clone(), recipe.effort.clone());
            format!("now on {agent} · later turns run there")
        } else if live.meta.machine == machine {
            format!("now in {} · later turns run here", short(folder))
        } else {
            format!(
                "moved to {}",
                Some(machine)
                    .filter(|m| !m.is_empty())
                    .unwrap_or("this host")
            )
        };
        if was != folder {
            live.meta.folder = Some(folder.to_owned());
        }
        machine.clone_into(&mut live.meta.machine);
        self.store.save(&live.meta)?;
        let item = live.chat.push(now(), 0, Body::Note { text });
        live.publish(self, vec![Change::Item(item.clone())])?;
        drop(live);
        if let Some(s) = session
            && let Err(e) = self.settle(id, &terminal, &s)
        {
            cooee_core::note!("cooee: {id}: model: {e}");
        }
        if let Err(e) = self.shed.close(&old) {
            cooee_core::note!("cooee: {e}");
        }
        Ok(item)
    }
}

fn last_turns(items: &[Item]) -> String {
    let turns: Vec<String> = items
        .iter()
        .filter_map(|i| match &i.body {
            Body::You { text, .. } => Some(format!("Owner: {text}")),
            Body::Agent { text, .. } => Some(format!("You: {text}")),
            _ => None,
        })
        .collect();
    let from = turns.len().saturating_sub(6);
    turns.get(from..).unwrap_or_default().join("\n\n")
}
