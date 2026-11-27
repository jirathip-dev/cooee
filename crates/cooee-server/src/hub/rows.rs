use std::{collections::HashMap, fs, io, sync::Mutex, time::Duration};

use blocks::{Action, Answer, Toast};
use cooee_core::{
    Body, Change, Item, Plugin,
    plugins::{
        Manifest, Permissions,
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
    offered: Mutex<HashMap<String, Permissions>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            desk: Mutex::default(),
            changed: watch::channel(0).0,
            offered: Mutex::default(),
        }
    }
}

/// The id of the button on cooee's own card that accepts a plugin's grown permissions.
pub const ACCEPT: &str = "cooee-permissions";

fn missing<T: PartialEq + Clone>(all: &[T], had: &[T]) -> Vec<T> {
    all.iter().filter(|a| !had.contains(a)).cloned().collect()
}

fn widen(to: &mut Permissions, by: &Permissions) {
    to.actions.extend(missing(&by.actions, &to.actions));
    to.destructive
        .extend(missing(&by.destructive, &to.destructive));
    to.places.extend(missing(&by.places, &to.places));
}

fn beyond(served: &Permissions, had: &Permissions) -> Permissions {
    let actions = missing(&served.actions, &had.actions);
    Permissions {
        places: missing(&served.places, &had.places),
        destructive: missing(&served.destructive, &had.destructive)
            .into_iter()
            .filter(|d| actions.contains(d))
            .collect(),
        actions,
        ..Permissions::default()
    }
}

fn asking(name: &str, new: &Permissions) -> Value {
    let action = |a: &String| {
        if new.destructive.contains(a) {
            format!("- {a} (always confirmed first)")
        } else {
            format!("- {a}")
        }
    };
    let mut lines: Vec<String> = new.actions.iter().map(action).collect();
    lines.extend(new.places.iter().map(|p| {
        format!(
            "- the {} place",
            serde_json::json!(p).as_str().unwrap_or_default()
        )
    }));
    serde_json::json!({
        "type": "card", "id": format!("{ACCEPT}-card"), "kind": "permissions",
        "title": format!("{name} asks for more"),
        "subtitle": "its manifest grew since you attached it",
        "blocks": [{ "type": "text", "id": format!("{ACCEPT}-new"), "markdown": lines.join("\n") }],
        "buttons": [{ "type": "button", "id": ACCEPT, "label": "accept", "action": "accept", "values": {} }],
    })
}

fn row_of(item: &Item, id: &str) -> Option<String> {
    match &item.body {
        Body::Row { plugin, block, .. } if holds(block, id) => Some(plugin.clone()),
        _ => None,
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

    /// Keeps `seq` as plugin `name`'s last applied row; fails when the file can't be written.
    #[expect(clippy::missing_errors_doc, reason = "the line names the error")]
    pub fn keep_row_cursor(&self, name: &str, seq: u64) -> std::io::Result<()> {
        let dir = self.data.join("plugins").join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("rows.seq"), seq.to_string())
    }

    /// Applies plugin `name`'s row in its chat or the cards place; a `loud` one also types its prompt and sends its push.
    pub fn apply_row(&self, name: &str, row: Row, loud: bool) {
        let Some(m) = self.with_rows(name) else {
            return;
        };
        let chat = match &row.to {
            To::Thread { id } => self.chat_of(id),
            _ => None,
        };
        let url = match (&row.to, &chat) {
            (To::Thread { .. }, Some(id)) => {
                if let Err(e) = self.row_in(id, name, &row) {
                    cooee_core::note!("cooee: plugin {name}: row {}: {e}", row.key);
                }
                format!("/t/{id}")
            }
            (To::Thread { id }, None) => {
                cooee_core::note!("cooee: plugin {name}: row {}: no chat {id}", row.key);
                format!("/p/{name}")
            }
            (To::Desk {}, _) => {
                self.desk_row(name, &row);
                format!("/p/{name}")
            }
            (To::Owner, _) => format!("/p/{name}"),
        };
        if let (true, Some(id), Some(prompt)) = (loud, &chat, &row.prompt) {
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

    fn chat_of(&self, thread: &str) -> Option<String> {
        self.all().into_iter().find_map(|l| {
            let l = lock(&l);
            (l.meta.plugin_thread() == thread).then(|| l.meta.id.clone())
        })
    }

    fn row_in(&self, id: &str, name: &str, row: &Row) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut l = lock(&live);
        let body = Body::Row {
            plugin: name.to_owned(),
            key: row.key.clone(),
            text: row.text.clone(),
            block: row.block.clone(),
            waits: row.open,
            group: row.group.clone(),
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
        let mut block = row.block.clone();
        if let (Value::Object(o), false) = (&mut block, row.group.is_empty()) {
            o.insert("group".to_owned(), Value::from(row.group.clone()));
        }
        self.desk_put(name, &row.key, row.open.then_some(block));
    }

    fn desk_put(&self, name: &str, key: &str, block: Option<Value>) {
        let mut desk = lock(&self.areas.rows.desk);
        desk.retain(|(p, k, _)| !(p == name && k == key));
        if let Some(block) = block {
            desk.push((name.to_owned(), key.to_owned(), block));
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

    fn granted_file(&self, name: &str) -> std::path::PathBuf {
        self.data.join("plugins").join(name).join("granted.json")
    }

    fn grants(&self, name: &str, m: &Manifest) -> Permissions {
        let mut all = m.permissions.clone();
        let file = fs::read(self.granted_file(name)).unwrap_or_default();
        if let Ok(granted) = serde_json::from_slice::<Permissions>(&file) {
            widen(&mut all, &granted);
        }
        all
    }

    fn serving(&self, name: &str) -> Option<(Plugin, String)> {
        let installed = cooee_core::plugins::installed(&self.data).ok()?;
        let (l, m) = installed
            .iter()
            .find(|(l, _)| l.name == name && l.enabled)?;
        let threads = m.threads.as_ref().filter(|_| l.listed)?;
        Some((self.backend(name)?, threads.manifest()))
    }

    /// Whether plugin `name` takes `action`: pinned in `m` or accepted; else re-reads what it serves.
    #[must_use]
    pub fn takes(&self, name: &str, m: &Manifest, action: &str) -> bool {
        if self.grants(name, m).actions.iter().any(|a| a == action) {
            return true;
        }
        self.offer(name);
        false
    }

    /// Shows once on cooee's card in its cards place what index plugin `name` serves beyond its grants.
    pub fn offer(&self, name: &str) {
        let (Some((p, path)), Some(m)) = (self.serving(name), self.with_rows(name)) else {
            return;
        };
        let served = match crate::plugin::served(&p, &path) {
            Ok(served) => served,
            Err(e) => return cooee_core::note!("cooee: plugin {name}: its manifest: {e}"),
        };
        let new = beyond(&served, &self.grants(name, &m));
        let none = new.actions.is_empty() && new.places.is_empty();
        let mut offered = lock(&self.areas.rows.offered);
        if none {
            offered.remove(name);
        } else if offered.get(name) != Some(&new) {
            offered.insert(name.to_owned(), new.clone());
        } else {
            return;
        }
        drop(offered);
        self.desk_put(name, ACCEPT, (!none).then(|| asking(name, &new)));
    }

    /// The owner accepts what plugin `name` asked for on cooee's card; false when it asked nothing.
    #[expect(clippy::missing_errors_doc, reason = "the line names it")]
    pub fn accept(&self, name: &str) -> io::Result<bool> {
        let Some(new) = lock(&self.areas.rows.offered).remove(name) else {
            return Ok(false);
        };
        let file = self.granted_file(name);
        let mut granted: Permissions =
            serde_json::from_slice(&fs::read(&file).unwrap_or_default()).unwrap_or_default();
        widen(&mut granted, &new);
        fs::create_dir_all(self.data.join("plugins").join(name))?;
        fs::write(&file, serde_json::to_vec(&granted)?)?;
        self.desk_put(name, ACCEPT, None);
        Ok(true)
    }

    /// `by`'s tap `a` on a plugin row's block in thread `id`, posted to its plugin; [`HubError::Refused`] when refused.
    #[expect(clippy::missing_errors_doc, reason = "the line names it")]
    pub fn row_act(&self, id: &str, a: &Action, by: &str) -> Result<Option<Answer>, HubError> {
        let Some(block) = a.block.clone() else {
            return Ok(None);
        };
        let live = self.live(id)?;
        let Some(name) = lock(&live)
            .chat
            .items
            .iter()
            .rev()
            .find_map(|i| row_of(i, &block))
        else {
            return Ok(None);
        };
        let refused = |why: String| HubError::Refused(format!("plugin {name}: {why}"));
        let m = self
            .with_rows(&name)
            .ok_or_else(|| refused("not installed".to_owned()))?;
        if !self.takes(&name, &m, &a.action) {
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
        let title = toast.and_then(Value::as_str).unwrap_or_default().to_owned();
        if !(200..300).contains(&status) {
            let why = if title.is_empty() {
                format!("answered {status}")
            } else {
                title
            };
            return Err(refused(why));
        }
        let toast = (!title.is_empty()).then(|| Toast::new(title));
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
