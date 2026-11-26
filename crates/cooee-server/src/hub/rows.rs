use std::{fs, sync::Mutex, time::Duration};

use blocks::{Action, Answer, Toast};
use cooee_core::{
    Body, Change, Item,
    plugins::{
        Manifest,
        rows::{Row, Said, To},
    },
    routines::Mark,
};
use serde_json::Value;
use tokio::sync::watch;

use super::{Hub, HubError, lock, now};
use crate::{Note, plugin};

pub(super) struct State {
    desk: Mutex<Vec<(String, String, Value)>>,
    changed: watch::Sender<u64>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            desk: Mutex::default(),
            changed: watch::channel(0).0,
        }
    }
}

fn holds(block: &Value, id: &str) -> bool {
    match block {
        Value::Object(o) => {
            o.get("id").and_then(Value::as_str) == Some(id) || o.values().any(|v| holds(v, id))
        }
        Value::Array(a) => a.iter().any(|v| holds(v, id)),
        _ => false,
    }
}

impl Hub {
    /// Installed plugin `name`'s manifest, when it serves rows.
    #[must_use]
    pub fn with_rows(&self, name: &str) -> Option<Manifest> {
        let installed = cooee_core::plugins::installed(&self.data).ok()?;
        let (_, m) = installed
            .iter()
            .find(|(l, _)| l.name == name && l.enabled)?;
        m.threads.is_some().then(|| m.clone())
    }

    /// The installed plugins that serve rows.
    #[must_use]
    pub fn row_plugins(&self) -> Vec<String> {
        let installed = cooee_core::plugins::installed(&self.data).unwrap_or_default();
        let names = installed.iter().map(|(l, _)| l.name.clone());
        names.filter(|n| self.with_rows(n).is_some()).collect()
    }

    /// The last row of plugin `name` this cooee applied, none before its first.
    #[must_use]
    pub fn row_cursor(&self, name: &str) -> Option<u64> {
        let file = self.data.join("plugins").join(name).join("rows.seq");
        fs::read_to_string(file).ok()?.trim().parse().ok()
    }

    /// Keeps `seq` as plugin `name`'s last applied row.
    ///
    /// # Errors
    /// When the file can't be written.
    pub fn keep_row_cursor(&self, name: &str, seq: u64) -> std::io::Result<()> {
        let dir = self.data.join("plugins").join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("rows.seq"), seq.to_string())
    }

    /// Applies plugin `name`'s row: drawn in its chat by key or kept in the cards place while open; a `loud` one
    /// (past the cursor) also types its prompt into the chat's agent and sends its push.
    pub fn apply_row(&self, name: &str, row: Row, loud: bool) {
        let Some(m) = self.with_rows(name) else {
            return;
        };
        let url = match &row.to {
            To::Thread { id } => {
                if let Err(e) = self.row_in(id, name, &row) {
                    cooee_core::note!("cooee: plugin {name}: row {}: {e}", row.key);
                }
                format!("/t/{id}")
            }
            To::Desk {} => {
                self.desk_row(name, &row);
                format!("/p/{name}")
            }
            To::Owner => format!("/p/{name}"),
        };
        if let (true, To::Thread { id }, Some(prompt)) = (loud, &row.to, &row.prompt) {
            let mark = Mark {
                id: format!("plugin-{name}"),
                name: name.to_owned(),
            };
            let rid = format!("row-{name}-{}", row.seq);
            if let Err(e) = self.queue_as(id, (prompt, &rid), &[], (Some(mark), None)) {
                cooee_core::note!("cooee: plugin {name}: row {}'s prompt: {e}", row.key);
            }
        }
        if let (true, true, Some(p)) = (loud, m.permissions.push, row.push) {
            self.push().send(&Note {
                title: p.title,
                body: p.body.chars().take(120).collect(),
                tag: p.tag,
                url,
                item: 0,
                say: false,
            });
        }
    }

    fn row_in(&self, id: &str, name: &str, row: &Row) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut l = lock(&live);
        let body = Body::Row {
            plugin: name.to_owned(),
            key: row.key.clone(),
            text: row.text.clone(),
            block: row.block.clone(),
        };
        let had = l.chat.items.iter().rev().find_map(|i| match &i.body {
            Body::Row { plugin, key, .. } if plugin == name && *key == row.key => Some(i.id),
            _ => None,
        });
        let item = match had.and_then(|at| l.chat.update(at, 0, |b| *b = body.clone())) {
            Some(item) => item,
            None => l.chat.push(now(), 0, body),
        };
        l.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    fn desk_row(&self, name: &str, row: &Row) {
        let mut desk = lock(&self.areas.rows.desk);
        desk.retain(|(p, k, _)| !(p == name && *k == row.key));
        if row.open {
            desk.push((name.to_owned(), row.key.clone(), row.block.clone()));
        }
        drop(desk);
        self.areas.rows.changed.send_modify(|n| *n += 1);
    }

    /// Plugin `name`'s open cards-place rows' blocks, oldest first.
    #[must_use]
    pub fn desk_rows(&self, name: &str) -> Vec<Value> {
        let desk = lock(&self.areas.rows.desk);
        let mine = desk.iter().filter(|(p, _, _)| p == name);
        mine.map(|(_, _, b)| b.clone()).collect()
    }

    /// Ticks when a cards-place row comes, changes or goes.
    #[must_use]
    pub fn desk_changed(&self) -> watch::Receiver<u64> {
        self.areas.rows.changed.subscribe()
    }

    /// `by`'s tap `a` on a plugin row's block in thread `id`, posted to its plugin; none when no row holds the block.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an action the plugin may not take, or when its backend fails or refuses.
    pub fn row_act(&self, id: &str, a: &Action, by: &str) -> Result<Option<Answer>, HubError> {
        let Some(block) = a.block.clone() else {
            return Ok(None);
        };
        let live = self.live(id)?;
        let name = lock(&live)
            .chat
            .items
            .iter()
            .rev()
            .find_map(|i| match &i.body {
                Body::Row {
                    plugin, block: b, ..
                } if holds(b, &block) => Some(plugin.clone()),
                _ => None,
            });
        let Some(name) = name else {
            return Ok(None);
        };
        let refused = |why: String| HubError::Refused(format!("plugin {name}: {why}"));
        let m = self
            .with_rows(&name)
            .ok_or_else(|| refused("not installed".to_owned()))?;
        if !m.permissions.actions.contains(&a.action) {
            return Err(refused(format!("{} isn't among its actions", a.action)));
        }
        let p = self
            .backend(&name)
            .ok_or_else(|| refused("down".to_owned()))?;
        let body = serde_json::json!({
            "screen": a.screen, "block": block, "action": a.action, "values": a.values, "by": by,
        });
        let json = [("Content-Type", "application/json")];
        let reply = plugin::call(
            &p,
            "POST",
            "/action",
            &json,
            body.to_string().as_bytes(),
            Duration::from_secs(10),
        )
        .map_err(|e| refused(e.to_string()))?;
        let status = reply.status;
        let v: Value =
            serde_json::from_slice(&reply.bytes().unwrap_or_default()).unwrap_or_default();
        let toast = v
            .get("toast")
            .and_then(|t| t.get("title"))
            .or_else(|| v.get("line"));
        let line = toast.and_then(Value::as_str).unwrap_or_default().to_owned();
        if !(200..300).contains(&status) {
            return Err(refused(if line.is_empty() {
                format!("answered {status}")
            } else {
                line
            }));
        }
        let toast = (!line.is_empty()).then(|| Toast::new(line));
        Ok(Some(Answer {
            toast,
            ..Answer::default()
        }))
    }

    pub(super) fn copy_said(&self, profile: &str, thread: &str, item: &Item) {
        let Body::You {
            text,
            routine: None,
            ..
        } = &item.body
        else {
            return;
        };
        let Some(name) = self.backing(profile).map(|(name, _)| name) else {
            return;
        };
        let path = self
            .with_rows(&name)
            .and_then(|m| m.threads)
            .map(|t| t.said);
        let (Some(path), Some(p)) = (path.filter(|s| !s.is_empty()), self.backend(&name)) else {
            return;
        };
        let said = Said {
            id: format!("{thread}-{}", item.id),
            ts: item.at / 1000,
            from: "owner".to_owned(),
            text: text.clone(),
        };
        let path = path.replace("{id}", thread);
        std::thread::spawn(move || {
            let json = [("Content-Type", "application/json")];
            let body = serde_json::to_vec(&said).unwrap_or_default();
            let sent = plugin::call(&p, "POST", &path, &json, &body, Duration::from_secs(10));
            match sent.map(|r| r.status) {
                Ok(200..=299) => {}
                Ok(code) => cooee_core::note!("cooee: plugin {}: said answered {code}", p.name),
                Err(e) => cooee_core::note!("cooee: plugin {}: said: {e}", p.name),
            }
        });
    }
}
