use std::collections::HashMap;

use cooee_core::Summary;

use super::{Hub, lock, now};
use crate::push::{Overview, Shown, due, kind, overview, rank};

#[derive(Default)]
pub(super) struct Surface {
    since: HashMap<String, (String, u64)>,
    sent: Option<(Shown, u64)>,
}

impl Surface {
    fn seen(&mut self, rows: &[Summary], at: u64) {
        self.since
            .retain(|id, (state, _)| rows.iter().any(|r| &r.id == id && &r.state == state));
        for r in rows {
            self.since
                .entry(r.id.clone())
                .or_insert_with(|| (r.state.clone(), at));
        }
    }

    fn since(&self, at: u64) -> impl Fn(&str) -> u64 {
        let since: HashMap<String, u64> = self
            .since
            .iter()
            .map(|(id, (_, t))| (id.clone(), *t))
            .collect();
        move |id| since.get(id).copied().unwrap_or(at)
    }
}

impl Hub {
    /// What needs you and what runs, now (`GET /glance`).
    #[must_use]
    pub fn overview(&self) -> Overview {
        let (rows, at) = (self.list(), now());
        let mut s = lock(&self.surface);
        s.seen(&rows, at);
        overview(&rows, s.since(at), at)
    }

    pub(super) fn surface(&self, at: u64) {
        let rows = self.list();
        let mut s = lock(&self.surface);
        s.seen(&rows, at);
        let next = rank(&rows, s.since(at));
        let Some(event) = due(s.sent.as_ref(), next.as_ref(), at) else {
            return;
        };
        let ended = || {
            let (was, _) = s.sent.clone().unwrap_or_default();
            let replied = rows
                .iter()
                .find(|r| r.id == was.thread && kind(r).is_none());
            Shown {
                kind: "ended".to_owned(),
                line: replied.map_or(was.line.clone(), |r| r.last.clone()),
                more: String::new(),
                since: at,
                ..was
            }
        };
        let shown = next.clone().unwrap_or_else(ended);
        s.sent = next.map(|n| (n, at));
        drop(s);
        match self.push.send_activity(event, &shown) {
            Ok(log) => log.iter().for_each(|l| cooee_core::note!("cooee: push{l}")),
            Err(e) => cooee_core::note!("cooee: push: live activity: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, state: &str) -> Summary {
        Summary {
            id: id.to_owned(),
            state: state.to_owned(),
            ..Summary::default()
        }
    }

    #[test]
    fn a_thread_keeps_the_time_it_entered_its_state_until_the_state_changes() {
        let mut s = Surface::default();
        s.seen(&[row("a", "working")], 10);
        s.seen(&[row("a", "working"), row("b", "working")], 20);
        assert_eq!(
            (s.since(30)("a"), s.since(30)("b"), s.since(30)("c")),
            (10, 20, 30)
        );
        s.seen(&[row("a", "blocked")], 40);
        assert_eq!((s.since(50)("a"), s.since(50)("b")), (40, 50));
    }
}
