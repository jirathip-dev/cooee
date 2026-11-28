use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use cooee_core::{
    Body, Calls as _, Change, Chat, Item, Kind, ShedError, Thread, Workflow, history,
    shed_proto::{self, AcpSession, TerminalInfo},
};

use super::{Hub, HubError, LABEL, Recipe, live::Live, lock, now, on_machine, outside};

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

    /// Switches thread `id` to ACP agent `agent` (#503) and `recipe`'s account (#76): a new session gets its brief and a handoff
    /// of the thread so far, as `relaunch` says; the thread keeps its id.
    ///
    /// # Errors
    /// [`HubError::Refused`] while a turn is under way, or when it already runs on `agent` and that account; as
    /// `relaunch`.
    pub fn switch(&self, id: &str, agent: &str, recipe: &Recipe) -> Result<Item, HubError> {
        let (folder, machine, was) = self.idle(id)?;
        if was == agent && lock(&*self.live(id)?).meta.account == recipe.account {
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

    fn start_terminal(
        &self,
        id: &str,
        (terminal, folder): (&str, &Path),
        agent: &str,
        recipe: &Recipe,
    ) -> Result<AcpSession, HubError> {
        let labels = BTreeMap::from([
            (LABEL.to_owned(), "1".to_owned()),
            (format!("{LABEL}.thread"), id.to_owned()),
        ]);
        self.mcp_launch(id, recipe)
            .and_then(|launch| {
                let cmd = self.adapter(agent);
                let run = (agent, cmd.as_slice());
                Ok(self.shed.run_acp(terminal, run, folder, &labels, &launch)?)
            })
            .and_then(|_| Ok(self.shed.ready(terminal, super::start::READY)?))
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
        let recipe = &self.attached(id, agent, recipe)?;
        let terminal = on_machine(machine, &format!("cooee-{id}-{:x}", now()));
        let session = self.start_terminal(id, (&terminal, folder), agent, recipe)?;
        let mut live = lock(&live);
        let was = live
            .meta
            .folder
            .clone()
            .unwrap_or_else(|| self.scratch.join(id));
        let story = story(&live, &was, (folder, machine), handoff);
        let handover = format!("{}\n\n{story}", recipe.instructions.trim_end());
        handover.trim_start().clone_into(&mut live.meta.handover);
        live.meta.terminal.clone_from(&terminal);
        self.retarget(&old, &terminal);
        let text = moved_line(
            &mut live,
            (folder, machine),
            (agent, recipe),
            handoff.is_some(),
        );
        if was != folder {
            live.meta.folder = Some(folder.to_owned());
        }
        machine.clone_into(&mut live.meta.machine);
        self.save(&live)?;
        let item = live.chat.push(now(), 0, Body::Note { text });
        live.publish(self, vec![Change::Item(item.clone())])?;
        drop(live);
        if let Err(e) = self.settle(id, &terminal, &session) {
            cooee_core::note!("cooee: {id}: model: {e}");
        }
        if let Err(e) = self.shed.close(&old) {
            cooee_core::note!("cooee: {e}");
        }
        Ok(item)
    }

    pub(super) fn reattach(&self) -> Result<Option<u64>, ShedError> {
        self.shed.hello()?;
        let (at, terminals) = (now(), self.shed.ls(&shed_proto::LsParams::default())?);
        self.prune(&terminals, at);
        for t in &terminals {
            if !lock(&self.terminals).contains_key(&t.name) {
                self.adopt(t);
            }
        }
        let mut since: Option<u64> = None;
        for live in self.all() {
            let terminal = {
                let l = lock(&live);
                let done =
                    l.meta.closed || l.meta.kind == Kind::Room || l.chat.turn.state == "ended";
                if done {
                    continue;
                }
                l.meta.terminal.clone()
            };
            let got = self.shed.get(&terminal);
            if let Err(e) = self.warm(&live) {
                cooee_core::note!("cooee: {e}");
                continue;
            }
            let mut live = lock(&live);
            if let Err(e) = self.load(&mut live) {
                cooee_core::note!("cooee: {e}");
                continue;
            }
            let changes = match got {
                Ok(t) if t.state == shed_proto::State::Running => {
                    since = Some(since.map_or(live.applied, |s| s.min(live.applied)));
                    let r = t.reported.unwrap_or_default();
                    live.chat.turn.report(&r.state, r.note);
                    live.chat.turn.context = t.session.and_then(|s| s.usage);
                    Vec::new()
                }
                Ok(_) => live.chat.ended(now(), 0, "The agent stopped.".to_owned()),
                Err(e) if e.code() == Some(shed_proto::ErrorCode::NoSuchTerminal) => live
                    .chat
                    .ended(now(), 0, "The agent is gone (shed restarted).".to_owned()),
                Err(e) if e.code() == Some(shed_proto::ErrorCode::NoLink) => Vec::new(),
                Err(e) => return Err(e),
            };
            if let Err(e) = live.publish(self, changes) {
                cooee_core::note!("cooee: {e}");
            }
        }
        Ok(since)
    }

    pub(super) fn adopt(&self, t: &TerminalInfo) -> Option<Arc<Mutex<Live>>> {
        let workflow = Workflow::from_labels(&t.labels).filter(|_| t.acp.is_some())?;
        let safe = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        let id: String = format!("w-{}", t.name)
            .chars()
            .map(|c| if safe(c) { c } else { '_' })
            .take(64)
            .collect();
        let title = format!("{} {}", workflow.group, workflow.item);
        let meta = Thread {
            id: id.clone(),
            title: Some(title.trim())
                .filter(|t| !t.is_empty())
                .unwrap_or(&t.name)
                .to_owned(),
            agent: t.acp.clone().unwrap_or_default(),
            folder: Some(t.cwd.clone()),
            terminal: t.name.clone(),
            workflow: Some(workflow),
            ..Thread::default()
        };
        if let Err(e) = self.store.save(&meta) {
            cooee_core::note!("cooee: {e}");
            return None;
        }
        let live = Arc::new(Mutex::new(Live::new(meta, Chat::default())));
        self.insert(id, t.name.clone(), &live);
        Some(live)
    }
}

impl Hub {
    pub(super) fn retarget(&self, old: &str, terminal: &str) {
        let mut terminals = lock(&self.terminals);
        let me = terminals.remove(old);
        terminals.extend(me.map(|l| (terminal.to_owned(), l)));
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

fn short(folder: &Path) -> String {
    if folder.as_os_str().is_empty() {
        return "scratch".to_owned();
    }
    let home = cooee_core::env_var("HOME").map(PathBuf::from);
    let homes = home
        .iter()
        .flat_map(|h| [h.clone(), h.canonicalize().unwrap_or_default()]);
    let homes = homes.filter(|h| h.has_root());
    let rest = homes
        .into_iter()
        .find_map(|h| folder.strip_prefix(h).ok().map(Path::to_path_buf));
    match rest {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => folder.display().to_string(),
    }
}

fn story(
    live: &Live,
    was: &Path,
    (folder, machine): (&Path, &str),
    handoff: Option<&str>,
) -> String {
    let place = |f: &Path, m: &str| match m {
        "" => f.display().to_string(),
        m => format!("{} on {m}", f.display()),
    };
    handoff.map_or_else(
        || {
            format!(
                "This chat moved from {} to {}, where you now run; what it made before is still there. Its last turns:\n\n{}",
                place(was, &live.meta.machine),
                place(folder, machine),
                last_turns(&live.chat.items)
            )
        },
        str::to_owned,
    )
}

fn moved_line(
    live: &mut Live,
    (folder, machine): (&Path, &str),
    (agent, recipe): (&str, &Recipe),
    switched: bool,
) -> String {
    if switched {
        agent.clone_into(&mut live.meta.agent);
        live.meta.account.clone_from(&recipe.account);
        (live.meta.model, live.meta.effort) = (recipe.model.clone(), recipe.effort.clone());
        let on = Some(recipe.account.as_str()).filter(|a| !a.is_empty());
        let on = on.map_or_else(|| agent.to_owned(), |a| format!("{agent} · {a}"));
        format!("now on {on} · later turns run there")
    } else if live.meta.machine == machine {
        format!("now in {} · later turns run here", short(folder))
    } else {
        format!(
            "moved to {}",
            Some(machine)
                .filter(|m| !m.is_empty())
                .unwrap_or("this host")
        )
    }
}
