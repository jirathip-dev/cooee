//! "Use a folder…" (#86) and a move to another machine (#151): a thread's agent starts again at its new place.

use std::{collections::BTreeMap, fs, path::Path};

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
        let machine = lock(&*self.live(id)?).meta.machine.clone();
        self.relaunch(id, (&folder, &machine), recipe)
    }

    /// Moves thread `id` to `machine` (#151; empty: this host), in the same folder, as `relaunch` says.
    ///
    /// # Errors
    /// [`HubError::Refused`] while a turn is under way (stop it first); as `relaunch`.
    pub fn move_to(&self, id: &str, machine: &str, recipe: &Recipe) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let folder = {
            let l = lock(&live);
            if matches!(
                l.chat.turn.state.as_str(),
                "working" | "connecting" | "blocked"
            ) {
                return Err(HubError::Refused(
                    "a turn is under way: stop it first".to_owned(),
                ));
            }
            l.meta
                .folder
                .clone()
                .unwrap_or_else(|| self.scratch.join(id))
        };
        self.relaunch(id, (&folder, machine), recipe)
    }

    /// Thread `id`'s later turns run in `folder` on `machine`. shed fixes a session's folder and machine when it
    /// starts, so its agent starts again there as `recipe` says, and the next message carries a hand-over: the
    /// profile's instructions, where it was and its last turns. The chat says so; the old terminal is closed.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a closed or Workflow thread; [`HubError::Shed`] when the agent can't start there
    /// (the thread stays where it was).
    fn relaunch(
        &self,
        id: &str,
        (folder, machine): (&Path, &str),
        recipe: &Recipe,
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let (agent, old) = {
            let l = lock(&live);
            if l.meta.closed || l.meta.workflow.is_some() {
                return Err(HubError::Refused(
                    "only an open chat of your own can move".to_owned(),
                ));
            }
            (l.meta.agent.clone(), l.meta.terminal.clone())
        };
        let terminal = on_machine(machine, &format!("cooee-{id}-{:x}", now()));
        let labels = BTreeMap::from([
            (LABEL.to_owned(), "1".to_owned()),
            (format!("{LABEL}.thread"), id.to_owned()),
        ]);
        let started = self
            .mcp_launch(id, recipe)
            .and_then(|launch| {
                let cmd = self.adapter(&agent);
                let run = (agent.as_str(), cmd.as_slice());
                Ok(self
                    .shed
                    .run_acp(&terminal, run, folder, &labels, &launch)?)
            })
            .and_then(|_| Ok(self.shed.ready(&terminal)?));
        if let Some((file, _)) = &recipe.launch.secrets {
            let _ = fs::remove_file(file);
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
        let turns: Vec<String> = live
            .chat
            .items
            .iter()
            .filter_map(|i| match &i.body {
                Body::You { text, .. } => Some(format!("Owner: {text}")),
                Body::Agent { text, .. } => Some(format!("You: {text}")),
                _ => None,
            })
            .collect();
        let last = turns[turns.len().saturating_sub(6)..].join("\n\n");
        let handover = format!(
            "{}\n\nThis chat moved from {} to {}, where you now run; what it made before is still there. Its last turns:\n\n{last}",
            recipe.instructions.trim_end(),
            place(&was, &live.meta.machine),
            place(folder, machine)
        );
        handover.trim_start().clone_into(&mut live.meta.handover);
        live.meta.terminal.clone_from(&terminal);
        {
            let mut terminals = lock(&self.terminals);
            let me = terminals.remove(&old);
            terminals.extend(me.map(|l| (terminal.clone(), l)));
        }
        let text = if live.meta.machine == machine {
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
            eprintln!("cooee: {id}: model: {e}");
        }
        if let Err(e) = self.shed.close(&old) {
            eprintln!("cooee: {e}");
        }
        Ok(item)
    }
}
