use std::{collections::BTreeMap, fs};

use cooee_core::{Body, Change, Kind, Launch, shed_proto};

use super::{Hub, HubError, LABEL, Recipe, lock, now, on_machine};

const FRESH: &str = "The agent started again fresh: it couldn't pick up where it left off.";

impl Hub {
    /// Starts chat `id`'s agent again once it is gone or stopped (#613), same persona and folder, as `recipe` says:
    /// it resumes the agent's last session when the agent can, else starts fresh with one line saying so. An agent
    /// that runs: nothing.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a closed chat, a room or another tool's session; [`HubError::Shed`] when the
    /// agent can't start.
    pub fn restart(&self, id: &str, recipe: &Recipe) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (old, machine, agent, folder, session) = {
            let l = lock(&live);
            let m = &l.meta;
            if m.closed || m.kind == Kind::Room || m.workflow.is_some() || !m.room.is_empty() {
                return Err(HubError::Refused(
                    "only an open chat of your own can restart".to_owned(),
                ));
            }
            let folder = m.folder.clone().unwrap_or_else(|| self.scratch.join(id));
            let session = Some(m.session.clone()).filter(|s| !s.is_empty());
            (
                m.terminal.clone(),
                m.machine.clone(),
                m.agent.clone(),
                folder,
                session,
            )
        };
        let runs = |t: shed_proto::TerminalInfo| t.state == shed_proto::State::Running;
        if self.shed.get(&old).is_ok_and(runs) {
            return Ok(());
        }
        self.shed.hello()?;
        self.reachable(&machine)?;
        let mut launch = self.mcp_launch(id, recipe)?;
        for (_, v) in &mut launch.env {
            *v = v.replace("{thread}", id);
        }
        let labels = BTreeMap::from([
            (LABEL.to_owned(), "1".to_owned()),
            (format!("{LABEL}.thread"), id.to_owned()),
        ]);
        let cmd = self.adapter(&agent);
        let stamp = now();
        let start = |resume: Option<String>, tail: &str| {
            let terminal = on_machine(&machine, &format!("cooee-{id}-{stamp:x}{tail}"));
            let launch = Launch {
                resume,
                ..launch.clone()
            };
            let run = (agent.as_str(), cmd.as_slice());
            let started = self.shed.run_acp(&terminal, run, &folder, &labels, &launch);
            let ready = started.and_then(|_| self.shed.ready(&terminal));
            if ready.is_err() {
                self.shed.close(&terminal).ok();
            }
            ready.map(|s| (terminal, s))
        };
        let resumed = session.and_then(|s| start(Some(s), "").ok());
        let started = resumed.clone().map_or_else(|| start(None, "f"), Ok);
        if let Some((file, _)) = &recipe.launch.secrets {
            fs::remove_file(file).ok();
        }
        let (terminal, ready) = started?;
        let mut l = lock(&live);
        l.meta.terminal.clone_from(&terminal);
        self.retarget(&old, &terminal);
        let mut changes = Vec::new();
        if resumed.is_none() {
            l.meta.session.clear();
            recipe.instructions.trim().clone_into(&mut l.meta.handover);
            let note = Body::Note {
                text: FRESH.to_owned(),
            };
            changes.push(Change::Item(l.chat.push(now(), 0, note)));
        }
        self.store.save(&l.meta)?;
        l.chat.turn.report("done", None);
        changes.push(Change::State(l.chat.turn.clone()));
        l.publish(self, changes)?;
        drop(l);
        if let Some(s) = ready
            && let Err(e) = self.settle(id, &terminal, &s)
        {
            cooee_core::note!("cooee: {id}: model: {e}");
        }
        self.shed.close(&old).ok();
        Ok(())
    }
}
