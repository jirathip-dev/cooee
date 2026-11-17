use cooee_core::{Body, Chat, Thread, Turn, agents, history};

use super::Hub;
use crate::push::Note;

impl Hub {
    pub(super) fn beat_end(&self, meta: &Thread, chat: &Chat, turn: &Turn) {
        let reply = chat.items.iter().rev().find_map(|i| match &i.body {
            Body::Agent { text, .. } => Some(text.as_str()),
            _ => None,
        });
        let lines: Vec<&str> = reply
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        let home = self
            .config_dir()
            .and_then(|c| agents::home(c, &meta.profile).ok());
        let rule = home.as_ref().map(agents::notify).unwrap_or_default();
        let why = match (turn.state.as_str(), lines.last().copied()) {
            ("error", _) => format!("it failed: {}", turn.note.as_deref().unwrap_or("no reason")),
            _ if rule.is_empty() => "no notify rule".to_owned(),
            (_, Some("NOTIFY")) => String::new(),
            (_, last) => last
                .and_then(|l| l.strip_prefix("SILENT:"))
                .map_or("its reply named no NOTIFY", str::trim)
                .to_owned(),
        };
        let pushed = why.is_empty();
        let key = format!("{}-{}", agents::HEARTBEAT, meta.profile);
        let ended = history::watermark(&self.data, &key, |w| {
            let open = w
                .last
                .as_mut()
                .filter(|b| b.run == meta.random_id && !b.pushed && b.why.is_empty());
            open.map(|b| {
                b.why = why;
                b.pushed = pushed;
            })
            .is_some()
        });
        if ended.as_ref().is_ok_and(|ended| *ended && pushed) {
            let name = agents::find(self.config_dir(), &self.data, &meta.profile)
                .map_or_else(|_| meta.profile.clone(), |p| p.name);
            let found = lines.iter().find(|l| **l != "NOTIFY");
            self.push.send(&Note {
                title: format!("{name} · heartbeat"),
                body: found
                    .map_or("it found something", |l| l)
                    .chars()
                    .take(120)
                    .collect(),
                tag: meta.id.clone(),
                url: format!("/t/{}", meta.id),
                item: chat.items.last().map_or(0, |i| i.id),
            });
        }
        if let Err(e) = ended {
            cooee_core::note!("cooee: {key}: {e}");
        }
    }
}
