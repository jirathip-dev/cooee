use cooee_core::{
    Change, plugins,
    shed_proto::{LinkState, TerminalInfo},
};

use super::{Hub, Live, lock};

impl Hub {
    /// Drops the Workflow sessions whose terminal is not among `terminals` (listed at `at`): the tool that started
    /// them cleaned them up (#33). One adopted since the listing or closed by the owner stays; one whose machine's
    /// link is down stays as `offline` (#381) and takes its reported state back once listed again.
    pub fn prune(&self, terminals: &[TerminalInfo], at: u64) {
        let links = self.shed.links().unwrap_or_default();
        for live in self.all() {
            let mut live = lock(&live);
            if live.meta.workflow.is_none() || live.meta.closed || live.moved > at {
                continue;
            }
            let offline = live.chat.turn.state == "offline";
            let listed = terminals.iter().find(|t| t.name == live.meta.terminal);
            let machine = live.meta.terminal.split_once('/').map(|(m, _)| m);
            let down = machine
                .and_then(|m| links.get(m))
                .is_some_and(|l| l.state != LinkState::Up);
            match listed {
                Some(t) if offline => {
                    let r = t.reported.clone().unwrap_or_default();
                    self.restate(&mut live, &r.state, r.note);
                }
                None if down && !offline => self.restate(&mut live, "offline", None),
                None if !down => {
                    let id = live.meta.id.clone();
                    drop(live);
                    self.remove(&id);
                    if let Err(e) = self.store.remove(&id) {
                        cooee_core::note!("cooee: {e}");
                    }
                }
                _ => {}
            }
        }
    }

    fn restate(&self, live: &mut Live, state: &str, note: Option<String>) {
        live.chat.turn.report(state, note);
        let changes = vec![Change::State(live.chat.turn.clone())];
        if let Err(e) = live.publish(self, changes) {
            cooee_core::note!("cooee: {e}");
        }
    }

    pub(super) fn closable(&self, live: &Live) -> bool {
        let Some(source) = live.meta.workflow.as_ref().map(|w| w.source.as_str()) else {
            return true;
        };
        let installed = plugins::installed(&self.data).unwrap_or_default();
        let claimed = installed
            .iter()
            .any(|(_, m)| m.claims.iter().any(|c| c == source));
        live.chat.turn.state == "ended" || !claimed
    }
}
