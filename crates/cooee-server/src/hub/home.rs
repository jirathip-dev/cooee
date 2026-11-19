use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use cooee_core::{
    StoreError, Summary,
    agents::{self, Agent},
};
use tokio::sync::broadcast;

use super::{Hub, Live, glance, lock, rooms};

type Stamp = Vec<(PathBuf, Option<(SystemTime, u64)>)>;

pub(super) struct Home {
    personas: Mutex<(Stamp, Arc<Vec<Agent>>)>,
    rows: Mutex<BTreeMap<String, Row>>,
    tx: broadcast::Sender<()>,
}

#[derive(Clone, PartialEq)]
struct Row {
    summary: Summary,
    speaker: String,
    ask: Option<(String, String, String)>,
    member: bool,
}

fn row(live: &Live) -> Row {
    let g = live.unloaded.clone().unwrap_or_else(|| glance(live));
    let meta = &live.meta;
    Row {
        summary: Summary {
            id: meta.id.clone(),
            title: meta.title.clone(),
            agent: meta.agent.clone(),
            account: meta.account.clone(),
            profile: meta.profile.clone(),
            persona: String::new(),
            persona_name: String::new(),
            persona_role: agents::Role::default(),
            folder: meta.folder.clone().unwrap_or_default(),
            machine: meta.machine.clone(),
            state: live.chat.turn.state.clone(),
            note: live.chat.turn.note.clone(),
            last: g.last,
            at: g.at,
            unread: g.unseen > 0,
            unseen: g.unseen,
            archived: meta.archived,
            setup: meta.setup,
            workflow: meta.workflow.clone(),
            routine: g.routine,
            members: rooms::seated(meta),
            asked_by: meta.asked_by.clone(),
        },
        speaker: g.speaker,
        ask: g.ask,
        member: !meta.room.is_empty(),
    }
}

impl Home {
    pub(super) fn new(threads: &BTreeMap<String, Arc<Mutex<Live>>>) -> Self {
        let rows = threads.iter().map(|(id, l)| (id.clone(), row(&lock(l))));
        Self {
            personas: Mutex::default(),
            rows: Mutex::new(rows.collect()),
            tx: broadcast::channel(16).0,
        }
    }

    pub(super) fn forget(&self, id: &str) {
        if lock(&self.rows).remove(id).is_some() {
            self.tx.send(()).ok();
        }
    }
}

impl Hub {
    /// The owner's personas, listed again only when one of the files they are read from changed.
    pub fn personas(&self) -> Arc<Vec<Agent>> {
        let (config, data) = (self.config_dir(), &self.data);
        let stamp = agents::stamp(config, data);
        let mut cached = lock(&self.home.personas);
        if cached.0 != stamp {
            *cached = (stamp, Arc::new(agents::list(config, data)));
            self.home.tx.send(()).ok();
        }
        Arc::clone(&cached.1)
    }

    /// One message each time a row of [`Hub::list`] may have changed.
    #[must_use]
    pub fn home(&self) -> broadcast::Receiver<()> {
        self.home.tx.subscribe()
    }

    pub(super) fn relist(&self, live: &Live) {
        let row = row(live);
        let mut rows = lock(&self.home.rows);
        if rows.get(&live.meta.id) != Some(&row) {
            rows.insert(live.meta.id.clone(), row);
            self.home.tx.send(()).ok();
        }
    }

    pub(super) fn save(&self, live: &Live) -> Result<(), StoreError> {
        self.store.save(&live.meta)?;
        self.relist(live);
        Ok(())
    }

    /// The thread list, newest first, from each thread's kept row: no thread's lock is taken.
    #[must_use]
    pub fn list(&self) -> Vec<Summary> {
        let personas = self.personas();
        let kept: Vec<Row> = lock(&self.home.rows).values().cloned().collect();
        let mut asks = Vec::new();
        let mut rows: Vec<Summary> = kept
            .into_iter()
            .filter_map(|r| {
                if r.member {
                    asks.extend(r.ask);
                    return None;
                }
                let mut s = r.summary;
                if let Some(p) = personas.iter().find(|p| p.id == s.profile) {
                    (s.persona, s.persona_name, s.persona_role) =
                        (p.id.clone(), p.name.clone(), p.role);
                }
                s.last = rooms::said(personas.as_slice(), &r.speaker, s.last);
                Some(s)
            })
            .collect();
        for (room, persona, tool) in asks {
            let Some(row) = rows.iter_mut().find(|r| r.id == room) else {
                continue;
            };
            let name = personas.iter().find(|p| p.id == persona);
            let name = name.map_or(persona.as_str(), |p| p.name.as_str());
            row.last = format!("{name}: {tool} waiting");
            row.unseen += 1;
            row.unread = true;
        }
        rows.sort_by_key(|r| std::cmp::Reverse(r.at));
        rows
    }
}
