//! Rooms (#154): a thread of the owner and 2 to 5 member personas, each in its own ACP session (a hidden thread,
//! started on its first turn). An owner message gives every member one turn in room order; each turn sends the
//! member the room's final replies it hasn't had, attributed (`[Name]: …`): never tool rows, approvals or files.

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use cooee_core::{Body, Change, Item, Kind, Member, Thread, personas};

use super::{Hub, HubError, Live, Recipe, Upload, lock, now};

/// How a room member's session starts, given its persona: its agent, folder and recipe.
pub type Starter =
    Box<dyn Fn(&Hub, &str) -> Result<(String, PathBuf, Recipe), HubError> + Send + Sync>;

/// The most members a room has (G6).
const MEMBERS: usize = 5;

/// Who speaks for the owner in a room.
const OWNER: &str = "owner";

fn refused(why: impl Into<String>) -> HubError {
    HubError::Refused(why.into())
}

impl Hub {
    /// Sets how a member's session starts (once).
    pub fn set_starter(&self, starter: Starter) {
        let _ = self.starter.set(starter);
    }

    /// A new room of `members` (persona ids, in room order), titled `title` (empty: their ids); once per
    /// `random_id`. Returns its id.
    ///
    /// # Errors
    /// [`HubError::Refused`] for fewer than 2 or more than 5 members, one twice, or one that can't join.
    pub fn create_room(
        &self,
        title: &str,
        members: &[String],
        random_id: &str,
    ) -> Result<String, HubError> {
        if !(2..=MEMBERS).contains(&members.len()) {
            return Err(refused(format!("a room has 2 to {MEMBERS} members")));
        }
        for (i, m) in members.iter().enumerate() {
            if members[..i].contains(m) {
                return Err(refused(format!("{m} is in the room twice")));
            }
            self.joinable(m)?;
        }
        let title = Some(title.trim())
            .filter(|t| !t.is_empty())
            .map_or_else(|| members.join(", "), str::to_owned);
        let members = members.iter().map(|p| Member {
            persona: p.clone(),
            ..Member::default()
        });
        let make = |id| Thread {
            id,
            title,
            kind: Kind::Room,
            members: members.collect(),
            random_id: random_id.to_owned(),
            ..Thread::default()
        };
        Ok(self.mint(random_id, make)?.0)
    }

    /// Adds persona `persona` to room `room` at its end, or (`remove`) takes it out and closes its session. Either
    /// twice does it once. Returns the members.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a thread that isn't a room, a sixth member or one that can't join.
    pub fn member(&self, room: &str, persona: &str, remove: bool) -> Result<Vec<String>, HubError> {
        if !remove {
            self.joinable(persona)?;
        }
        let live = self.live(room)?;
        let mut l = lock(&live);
        if l.meta.kind != Kind::Room {
            return Err(refused(format!("{room} isn't a room")));
        }
        let at = l.meta.members.iter().position(|m| m.persona == persona);
        let mut gone = String::new();
        match (at, remove) {
            (Some(at), true) => {
                gone = l.meta.members.remove(at).session;
                l.meta.round.retain(|p| p != persona);
            }
            (None, false) if l.meta.members.len() >= MEMBERS => {
                return Err(refused(format!("a room has at most {MEMBERS} members")));
            }
            (None, false) => l.meta.members.push(Member {
                persona: persona.to_owned(),
                ..Member::default()
            }),
            _ => return Ok(l.meta.members.iter().map(|m| m.persona.clone()).collect()),
        }
        self.store.save(&l.meta)?;
        let members = l.meta.members.iter().map(|m| m.persona.clone()).collect();
        drop(l);
        if !gone.is_empty() {
            self.close(&gone)?;
        }
        Ok(members)
    }

    /// Persona `persona` may join a room: the conductor's may, a lane thread may not (G5).
    fn joinable(&self, persona: &str) -> Result<(), HubError> {
        if self
            .live(persona)
            .is_ok_and(|l| lock(&l).meta.workflow.is_some())
        {
            return Err(refused(format!(
                "{persona} is a lane thread: it stays a Workflow chat, never a room member"
            )));
        }
        personas::find(self.config_dir(), &self.data, persona)
            .map(drop)
            .map_err(|e| refused(e.to_string()))
    }

    /// The owner's message to room `live`: stored as the owner's, then every member gets a turn after the one
    /// taking its turn now, in room order (G1).
    pub(super) fn room_send(
        &self,
        live: &Arc<Mutex<Live>>,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Item, HubError> {
        let mut l = lock(live);
        if let Some(item) = Self::sent(&l, random_id) {
            return Ok(item);
        }
        let q = self.take(&l, text, random_id, files)?;
        let body = Body::You {
            text: q.text,
            random_id: q.random_id,
            files: q.files,
        };
        let item = l.chat.said(now(), 0, OWNER.to_owned(), body);
        let idle = l.meta.round.is_empty();
        let after = l.meta.round.get(1..).unwrap_or_default().to_vec();
        let turns = l.meta.members.iter().map(|m| m.persona.clone());
        let turns: Vec<String> = turns.filter(|p| !after.contains(p)).collect();
        l.meta.round.extend(turns);
        self.store.save(&l.meta)?;
        l.publish(self, vec![Change::Item(item.clone())])?;
        let id = l.meta.id.clone();
        drop(l);
        if idle {
            self.turns.send(id).map_err(|e| refused(e.to_string()))?;
        }
        Ok(item)
    }

    /// Starts the turn of room `room`'s next member: its session (started now if it has none) gets the room's
    /// final replies since its last turn, attributed. One that can't take it is said so in the room, and skipped.
    pub(super) fn take_turn(&self, room: &str) -> Result<(), HubError> {
        let live = self.live(room)?;
        let (persona, session, prompt, random_id) = {
            let mut l = lock(&live);
            let Some(persona) = l.meta.round.first().cloned() else {
                return Ok(());
            };
            let names = personas::list(self.config_dir(), &self.data);
            let name = |s: &str| match names.iter().find(|p| p.id == s) {
                _ if s == OWNER => "Owner".to_owned(),
                Some(p) => p.name.clone(),
                None => s.to_owned(),
            };
            let last = l.chat.items.last().map_or(0, |i| i.id);
            let Live { meta, chat, .. } = &mut *l;
            let m = meta.members.iter_mut().find(|m| m.persona == persona);
            let m = m.ok_or_else(|| refused(format!("{persona} left")))?;
            let new = chat
                .items
                .iter()
                .filter(|i| i.id > m.heard && i.speaker != persona);
            let lines: Vec<String> = new
                .filter_map(|i| match &i.body {
                    Body::You { text, .. } | Body::Agent { text, .. } => {
                        Some(format!("[{}]: {text}", name(&i.speaker)))
                    }
                    _ => None,
                })
                .collect();
            m.heard = last;
            let out = (persona.clone(), m.session.clone(), lines.join("\n\n"));
            self.store.save(meta)?;
            (out.0, out.1, out.2, format!("{room}-{persona}-{last}"))
        };
        let started = if session.is_empty() {
            self.seat(&persona, (&prompt, &random_id), |r| {
                room.clone_into(&mut r.room);
            })
        } else {
            self.send(&session, &prompt, &random_id, &[])
                .map(|_| session)
        };
        let mut l = lock(&live);
        match started {
            Ok(session) => {
                if let Some(m) = l.meta.members.iter_mut().find(|m| m.persona == persona) {
                    m.session = session;
                }
                Ok(self.store.save(&l.meta)?)
            }
            Err(e) => {
                let text = format!("{persona} couldn't take its turn: {e}");
                let item = l.chat.push(now(), 0, Body::Note { text });
                l.publish(self, vec![Change::Item(item)])?;
                drop(l);
                self.replied(room, &persona, None)
            }
        }
    }

    /// Starts a session of persona `persona` with `prompt`, its recipe as `seat` makes it (a room's member, an
    /// asked persona); returns its thread id.
    pub(super) fn seat(
        &self,
        persona: &str,
        (prompt, random_id): (&str, &str),
        seat: impl FnOnce(&mut Recipe),
    ) -> Result<String, HubError> {
        let starter = self.starter.get().ok_or_else(|| refused("no starter"))?;
        let (agent, folder, mut recipe) = starter(self, persona)?;
        seat(&mut recipe);
        let seated = self
            .place(&folder)
            .and_then(|folder| self.start(&agent, &folder, prompt, random_id, &recipe));
        // As in [`Hub::start`], which a folder outside the roots never reaches.
        if let Some((file, _)) = &recipe.launch.secrets {
            let _ = fs::remove_file(file);
        }
        seated
    }

    /// Member `persona`'s turn in room `room` ended with `reply` (its final reply, `None`: none): stored as its, and
    /// the next member's turn starts. A reply out of turn (a member taken out) is dropped.
    pub(super) fn replied(
        &self,
        room: &str,
        persona: &str,
        reply: impl Into<Option<Body>>,
    ) -> Result<(), HubError> {
        let live = self.live(room)?;
        let mut l = lock(&live);
        if l.meta.round.first().is_none_or(|p| p != persona) {
            return Ok(());
        }
        l.meta.round.remove(0);
        self.store.save(&l.meta)?;
        if let Some(body) = reply.into() {
            let item = l.chat.said(now(), 0, persona.to_owned(), body);
            l.publish(self, vec![Change::Item(item)])?;
        }
        drop(l);
        self.turns
            .send(room.to_owned())
            .map_err(|e| refused(e.to_string()))
    }
}
