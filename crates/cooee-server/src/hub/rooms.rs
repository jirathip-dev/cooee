//! Rooms (#154): a thread of the owner and 2 to 5 member personas, each in its own ACP session (a hidden thread,
//! started on its first turn). Each turn sends the member the room's final replies it hasn't had, attributed
//! (`[Name]: …`): never tool rows, approvals or files.
//!
//! The turn engine (#155): an owner message's @mentioned members take turns in mention order; with no mention every
//! member gets one turn in room order and may pass (a reply of exactly [`PASS`], never shown). A reply naming another
//! member gives it a turn. At most [`TURNS`] turns per owner message, the last told it is winding down, and none
//! after [`DEADLINE`]; stop (the room's cancel) cancels the member taking its turn and drops the rest.

use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use cooee_core::{
    Body, Change, Item, Kind, Member, Reason, RoomTurn, Thread,
    personas::{self, Persona},
};
use serde_json::json;

use super::{Hub, HubError, Live, Recipe, Upload, lock, now};
use crate::push::Note;

/// How a room member's session starts, given its persona: its agent, folder and recipe.
pub type Starter =
    Box<dyn Fn(&Hub, &str) -> Result<(String, PathBuf, Recipe), HubError> + Send + Sync>;

/// The most members a room has (G6).
const MEMBERS: usize = 5;

/// Who speaks for the owner in a room.
const OWNER: &str = "owner";

/// The most member turns one owner message starts (G2).
const TURNS: usize = 6;

/// How long after the owner's message its turns may run, in milliseconds (G2).
pub(super) const DEADLINE: u64 = 10 * 60 * 1000;

/// A member's whole reply when it passes: not shown in the room.
const PASS: &str = "[pass]";

/// Told to the member taking the last turn an owner message allows.
const WIND_DOWN: &str = "(This is the last turn on the owner's message: wind down; naming a member starts no more turns.)";

/// The words `text` names with `@`, keyed (spaces dropped, any case), in order.
fn mentions(text: &str) -> impl Iterator<Item = String> {
    text.match_indices('@').filter_map(|(at, _)| {
        let before = text[..at].chars().next_back();
        if before.is_some_and(char::is_alphanumeric) {
            return None;
        }
        let word: String = text[at + 1..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || "-_".contains(*c))
            .collect();
        Some(key(&word))
    })
}

fn key(s: &str) -> String {
    s.replace(' ', "").to_lowercase()
}

/// The members `text` names with `@` and their id or name (spaces dropped, any case), in the order named, once each.
fn named(text: &str, members: &[Member], names: &[Persona]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for word in mentions(text) {
        let hit = members.iter().find(|m| {
            key(&m.persona) == word
                || names
                    .iter()
                    .any(|p| p.id == m.persona && key(&p.name) == word)
        });
        if let Some(m) = hit.filter(|m| !out.contains(&m.persona)) {
            out.push(m.persona.clone());
        }
    }
    out
}

/// A reply addresses the owner when it names `@you` or `@owner` (#158).
fn addresses_owner(text: &str) -> bool {
    mentions(text).any(|w| w == "you" || w == OWNER)
}

/// How the room shows `id`: the owner, a persona's name, else the id.
fn display(names: &[Persona], id: &str) -> String {
    match names.iter().find(|p| p.id == id) {
        _ if id == OWNER => "Owner".to_owned(),
        Some(p) => p.name.clone(),
        None => id.to_owned(),
    }
}

fn refused(why: impl Into<String>) -> HubError {
    HubError::Refused(why.into())
}

impl Hub {
    /// `note` about thread `meta`, said by its room when `meta` is a member's session (#158).
    pub(super) fn routed(&self, meta: &Thread, note: Note) -> Note {
        let room = Some(&meta.room).filter(|r| !r.is_empty());
        let Some(room) = room.and_then(|r| self.live(r).ok()) else {
            return note;
        };
        let room = lock(&room);
        let names = personas::list(self.config_dir(), &self.data);
        let last = room.chat.items.last().map_or(0, |i| i.id);
        note.in_room(&room.meta, &display(&names, &meta.profile), last)
    }

    /// Pushes room `room`'s `key` (`needs`, `fail`) about member `persona`, `line`, at item `item` (#158).
    fn room_push(&self, room: &Thread, persona: &str, (key, line): (&str, &str), item: u64) {
        let names = personas::list(self.config_dir(), &self.data);
        let name = display(&names, persona);
        let note = Note::new(room, (key, &name, line), item, &self.theme);
        self.push.send(&note.in_room(room, &name, item));
    }

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
                l.meta.round.retain(|t| t.member != persona);
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

    /// The owner's message to room `live`: stored as the owner's; then, after the member taking its turn now, its
    /// mentioned members take turns, else every member in room order (G1). Earlier messages' queued turns are dropped.
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
            routine: None,
        };
        let item = l.chat.said(now(), 0, OWNER.to_owned(), body);
        let idle = l.meta.round.is_empty();
        let names = personas::list(self.config_dir(), &self.data);
        let mut turns = named(text, &l.meta.members, &names);
        let reason = if turns.is_empty() {
            turns = l.meta.members.iter().map(|m| m.persona.clone()).collect();
            Reason::Round
        } else {
            Reason::Mention
        };
        l.meta.round.truncate(1);
        let turns = turns.into_iter().map(|member| RoomTurn { member, reason });
        l.meta.round.extend(turns);
        l.meta.taken = 0;
        l.meta.asked_at = now();
        self.store.save(&l.meta)?;
        l.publish(self, vec![Change::Item(item.clone())])?;
        let id = l.meta.id.clone();
        drop(l);
        if idle {
            self.turns.send(id).map_err(|e| refused(e.to_string()))?;
        }
        Ok(item)
    }

    /// Starts the turn of room `room`'s next member: its session (started now if it has none, told who is in the
    /// room) gets the room's final replies since its last turn, attributed; the last turn is told it is winding down.
    /// One that can't take it is said so in the room, and skipped.
    pub(super) fn take_turn(&self, room: &str) -> Result<(), HubError> {
        let live = self.live(room)?;
        let (persona, session, prompt, random_id) = {
            let mut l = lock(&live);
            let Some(turn) = l.meta.round.first().cloned() else {
                return Ok(());
            };
            let (persona, reason) = (turn.member, turn.reason);
            l.meta.taken = l.meta.taken.saturating_add(1);
            let last_turn = usize::from(l.meta.taken) >= TURNS;
            let event = json!({ "member": persona, "reason": reason });
            l.emit("room-turn", event.to_string());
            let names = personas::list(self.config_dir(), &self.data);
            let name = |s: &str| display(&names, s);
            let last = l.chat.items.last().map_or(0, |i| i.id);
            let Live { meta, chat, .. } = &mut *l;
            let others = meta.members.iter().filter(|m| m.persona != persona);
            let others: Vec<String> = others.map(|m| name(&m.persona)).collect();
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
            let mut prompt = lines.join("\n\n");
            if m.session.is_empty() {
                let me = name(&persona);
                let others = others.join(", ");
                prompt = format!(
                    "You are {me}, a member of a room with the owner and {others}. Write @ and a member's name to \
                     give them the next turn. Reply exactly {PASS} when you have nothing to add.\n\n{prompt}"
                );
            }
            if last_turn {
                prompt = format!("{prompt}\n\n{WIND_DOWN}");
            }
            let out = (persona.clone(), m.session.clone(), prompt);
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
        self.started(room, &persona, started)
    }

    /// Member `persona`'s turn in room `room` started (`Ok`: its session) or couldn't: its session is kept, and
    /// cancelled if [stop] came while it started; one that couldn't is said so in the room, and skipped.
    pub(super) fn started(
        &self,
        room: &str,
        persona: &str,
        started: Result<String, HubError>,
    ) -> Result<(), HubError> {
        let live = self.live(room)?;
        let mut l = lock(&live);
        // [stop] had no session to cancel. (Its turn may also be gone because it already replied: no stop.)
        let stopped = std::mem::take(&mut l.stopped_starting);
        match started {
            Ok(session) => {
                if let Some(m) = l.meta.members.iter_mut().find(|m| m.persona == persona) {
                    m.session.clone_from(&session);
                }
                self.store.save(&l.meta)?;
                drop(l);
                if stopped {
                    self.cancel(&session)
                } else {
                    Ok(())
                }
            }
            Err(e) => {
                let text = format!("{persona} couldn't take its turn: {e}");
                let item = l.chat.push(now(), 0, Body::Note { text });
                self.room_push(&l.meta, persona, ("fail", &e.to_string()), item.id);
                l.publish(self, vec![Change::Item(item)])?;
                drop(l);
                self.replied(room, persona, None)
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

    /// Member `persona`'s turn in room `room` ended with `reply` (its final reply, `None`: none): stored as its unless
    /// it passed, the members it names queued (up to the turn cap: past it they are dropped and the room told), and
    /// the next member's turn starts. A reply out of turn (a member taken out, a stopped round) is dropped.
    pub(super) fn replied(
        &self,
        room: &str,
        persona: &str,
        reply: impl Into<Option<Body>>,
    ) -> Result<(), HubError> {
        let live = self.live(room)?;
        let mut l = lock(&live);
        if l.meta.round.first().is_none_or(|t| t.member != persona) {
            return Ok(());
        }
        l.meta.round.remove(0);
        let mut changes = Vec::new();
        let mut capped = false;
        let reply = reply.into().filter(|b| match b {
            Body::Agent { text, .. } => text.trim() != PASS,
            _ => true,
        });
        if let Some(Body::Agent { text, .. }) = &reply {
            let names = personas::list(self.config_dir(), &self.data);
            for member in named(text, &l.meta.members, &names) {
                if member == persona || l.meta.round.iter().any(|t| t.member == member) {
                    continue;
                }
                if usize::from(l.meta.taken) + l.meta.round.len() >= TURNS {
                    capped = true;
                    continue;
                }
                let reason = Reason::Chained;
                l.meta.round.push(RoomTurn { member, reason });
            }
        }
        if let Some(body) = reply {
            let item = l.chat.said(now(), 0, persona.to_owned(), body);
            if let Body::Agent { text, .. } = &item.body
                && addresses_owner(text)
            {
                let first = text.lines().find(|l| !l.trim().is_empty());
                let said = ("needs", first.unwrap_or_default());
                self.room_push(&l.meta, persona, said, item.id);
            }
            changes.push(Change::Item(item));
        }
        if capped {
            let text = format!("{TURNS} turns on one message: no more start until your next");
            changes.push(Change::Item(l.chat.push(now(), 0, Body::Note { text })));
            l.emit("room-stop", json!({ "reason": "cap" }).to_string());
        }
        self.store.save(&l.meta)?;
        l.publish(self, changes)?;
        drop(l);
        self.turns
            .send(room.to_owned())
            .map_err(|e| refused(e.to_string()))
    }

    /// Ends room `room`'s round, `why` (`stop`, `deadline`), with `note` (if any) said in the room: its queued turns
    /// are dropped and the member taking its turn is cancelled. Twice, or with no round, does nothing.
    pub(super) fn stop_round(&self, room: &str, why: &str, note: &str) -> Result<(), HubError> {
        let live = self.live(room)?;
        let mut l = lock(&live);
        let Some(turn) = l.meta.round.first().cloned() else {
            return Ok(());
        };
        l.meta.round.clear();
        self.store.save(&l.meta)?;
        l.emit("room-stop", json!({ "reason": why }).to_string());
        if !note.is_empty() {
            let text = note.to_owned();
            let item = l.chat.push(now(), 0, Body::Note { text });
            l.publish(self, vec![Change::Item(item)])?;
        }
        let working = l.meta.members.iter().find(|m| m.persona == turn.member);
        let session = working.map(|m| m.session.clone()).unwrap_or_default();
        l.stopped_starting = working.is_some() && session.is_empty();
        drop(l);
        if session.is_empty() {
            Ok(())
        } else {
            self.cancel(&session)
        }
    }

    /// Stops each room's round whose owner message is [`DEADLINE`] old at `now` (G2).
    pub(super) fn deadlines(&self, now: u64) {
        let due: Vec<String> = self
            .all()
            .iter()
            .filter_map(|live| {
                let l = lock(live);
                let due = !l.meta.round.is_empty() && now >= l.meta.asked_at + DEADLINE;
                due.then(|| l.meta.id.clone())
            })
            .collect();
        for room in due {
            let note = "10 minutes passed: this message's turns stopped";
            if let Err(e) = self.stop_round(&room, "deadline", note) {
                eprintln!("cooee: room {room}: {e}");
            }
        }
    }
}
