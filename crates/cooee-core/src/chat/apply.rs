use shed_proto::EventKind;

use super::{Body, Card, Change, Chat, Item};

impl Chat {
    /// Applies shed event `seq` of this thread's terminal at time `at`.
    pub fn apply(&mut self, at: u64, seq: u64, event: EventKind) -> Vec<Change> {
        match event {
            EventKind::Started
            | EventKind::Resources(_)
            | EventKind::Mode { .. }
            | EventKind::Turn { .. }
            | EventKind::Session(_) => Vec::new(),
            EventKind::Usage(u) => {
                self.turn.context = Some(u);
                vec![Change::State(self.turn.clone())]
            }
            EventKind::State(r) => vec![self.state(&r.state, r.note)],
            EventKind::Text { text } => {
                self.turn.text.push_str(&text);
                vec![Change::Text(text)]
            }
            EventKind::ToolCall {
                id, title, status, ..
            } => {
                let done = matches!(status.as_str(), "completed" | "failed");
                let ended = done.then_some(at);
                let call = |b: &Body| matches!(b, Body::Tool { call, .. } if *call == id);
                let item = match self.find(call) {
                    Some(item) => self.update(item, seq, |b| {
                        *b = Body::Tool {
                            call: id,
                            title,
                            status,
                            ended,
                        };
                    }),
                    None => Some(self.push(
                        at,
                        seq,
                        Body::Tool {
                            call: id,
                            title,
                            status,
                            ended,
                        },
                    )),
                };
                item.map(Change::Item).into_iter().collect()
            }
            EventKind::Plan { entries } => {
                let plan = |b: &Body| matches!(b, Body::Plan { .. });
                let item = match self.find(plan) {
                    Some(item) => self.update(item, seq, |b| *b = Body::Plan { entries }),
                    None => Some(self.push(at, seq, Body::Plan { entries })),
                };
                item.map(Change::Item).into_iter().collect()
            }
            EventKind::ApprovalRequest { tool, options } => {
                let body = Body::Approval {
                    tool,
                    options,
                    tapped: None,
                    chosen: None,
                    plugin: None,
                    card: Card::default(),
                };
                vec![Change::Item(self.push(at, seq, body))]
            }
            EventKind::Approved { option } => {
                let open = self.items.iter().find(|i| {
                    matches!(
                        i.body,
                        Body::Approval {
                            chosen: None,
                            plugin: None,
                            ..
                        }
                    )
                });
                let item = open.map(|i| i.id).and_then(|id| {
                    self.update(id, seq, |b| {
                        if let Body::Approval { chosen, .. } = b {
                            *chosen = Some(option);
                        }
                    })
                });
                item.map(Change::Item).into_iter().collect()
            }
            EventKind::Reply { text, stop_reason } => {
                let status = (stop_reason == "cancelled").then_some("cancelled");
                let mut changes = self.end(at, seq, status);
                changes.push(Change::Item(self.push(
                    at,
                    seq,
                    Body::Agent { text, stop_reason },
                )));
                changes
            }
            EventKind::Exited { code } => {
                let text = code.map_or("The agent stopped.".to_owned(), |c| {
                    format!("The agent exited ({c}).")
                });
                self.ended(at, seq, text)
            }
            EventKind::Killed => self.ended(at, seq, "The thread was closed.".to_owned()),
        }
    }

    /// The session is over: the turn ends and an `ended` row says why.
    pub fn ended(&mut self, at: u64, seq: u64, text: String) -> Vec<Change> {
        let mut changes = self.end(at, seq, Some("cancelled"));
        changes.push(Change::Item(self.push(at, seq, Body::Ended { text })));
        changes.push(self.state("ended", None));
        changes
    }

    /// Records `by`'s tap of `option` on approval `id` at `now`. `None` when it isn't an approval or was already
    /// tapped or answered, so the tap must not be sent again; a conductor's answer is open to a change until its
    /// `until`.
    pub fn tap(&mut self, id: u64, option: &str, by: &str, now: u64) -> Option<Item> {
        let open = |b: &Body| match b {
            Body::Approval {
                tapped: None,
                chosen,
                card,
                ..
            } => chosen.is_none() || card.until.is_some_and(|u| now < u),
            _ => false,
        };
        if !self.items.iter().any(|i| i.id == id && open(&i.body)) {
            return None;
        }
        self.update(id, 0, |b| {
            if let Body::Approval {
                tapped,
                chosen,
                card,
                ..
            } = b
            {
                *tapped = Some(option.to_owned());
                *chosen = None;
                card.by = Some(by.to_owned());
                card.answered = Some(now);
                card.until = None;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approval(plugin: Option<&str>) -> Body {
        Body::Approval {
            tool: "Ship it?".to_owned(),
            options: Vec::new(),
            tapped: None,
            chosen: None,
            plugin: plugin.map(str::to_owned),
            card: Card::default(),
        }
    }

    #[test]
    fn shed_answers_its_own_approval_never_a_plugins_card() {
        let mut chat = Chat::default();
        let card = chat.push(1, 0, approval(Some("drover"))).id;
        let ask = chat.push(2, 1, approval(None)).id;
        let option = "allow".to_owned();
        chat.apply(3, 2, EventKind::Approved { option });
        let chosen = |id| match &chat.items.iter().find(|i| i.id == id).unwrap().body {
            Body::Approval { chosen, .. } => chosen.clone(),
            _ => None,
        };
        assert_eq!(
            (chosen(card), chosen(ask)),
            (None, Some("allow".to_owned()))
        );
    }

    #[test]
    fn a_conductors_answer_may_be_changed_for_its_window_only() {
        let mut chat = Chat::default();
        let mut body = approval(Some("drover"));
        if let Body::Approval { chosen, card, .. } = &mut body {
            *chosen = Some("yes".to_owned());
            card.until = Some(100);
        }
        let late = chat.push(1, 0, body.clone()).id;
        let id = chat.push(1, 0, body).id;
        assert!(chat.tap(late, "no", "iPhone", 100).is_none(), "too late");
        let changed = chat.tap(id, "no", "iPhone", 99).unwrap();
        let Body::Approval {
            tapped,
            chosen,
            card,
            ..
        } = changed.body
        else {
            unreachable!()
        };
        assert_eq!(
            (
                tapped.as_deref(),
                chosen,
                card.by.as_deref(),
                card.answered,
                card.until
            ),
            (Some("no"), None, Some("iPhone"), Some(99), None)
        );
        assert!(chat.tap(id, "yes", "iPhone", 99).is_none(), "once");
    }
}
