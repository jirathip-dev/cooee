use cooee_core::{Body, Change, agents, history, memory};

use super::proposals::Proposed;
use super::{Hub, HubError, lock, now};

impl Hub {
    /// An agent's `remember` in thread `id` (#126, #216): an edit of its persona's notes, for its next sessions; the
    /// answer names the note's id and the characters used. In a routine's run (#219) it only raises a card, and the
    /// edit is made when the owner approves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or an edit refused.
    pub fn remember(&self, id: &str, token: &str, e: &memory::Edit) -> Result<String, HubError> {
        let home = self.home_of(id, token)?;
        let meta = self.token_of(id, token)?;
        self.scan(id, "remember", e.text.iter())?;
        if meta.routine.is_some() {
            let what = match (&e.text, &e.replaces) {
                (Some(_), Some(r)) => format!("replace {}'s note [{r}]", meta.profile),
                (Some(_), None) => format!("add a note to {}'s memory", meta.profile),
                (None, _) => format!("forget {}'s notes {}", meta.profile, e.forget.join(", ")),
            };
            let (detail, e) = (e.text.clone().unwrap_or_default(), e.clone());
            let then = Box::new(move || {
                memory::remember(&home, "persona", &e, now())
                    .map(|n| {
                        n.map_or_else(|| "forgotten".to_owned(), |n| format!("kept as [{}]", n.id))
                    })
                    .map_err(|e| e.to_string())
            });
            self.raise(id, (what, "memory", detail), Proposed::Later(then))?;
            return Ok(
                "Proposed. The owner sees a card; nothing is saved until they approve it."
                    .to_owned(),
            );
        }
        let kept = memory::remember(&home, "persona", e, now())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let used = memory::used(&memory::notes(&home));
        let what = kept.map_or_else(|| "Forgotten".to_owned(), |n| format!("Kept as [{}]", n.id));
        Ok(format!(
            "{what}; {used} of {} characters used.",
            memory::CAP
        ))
    }

    pub(crate) fn scan<'a>(
        &self,
        id: &str,
        what: &str,
        mut texts: impl Iterator<Item = &'a String>,
    ) -> Result<(), HubError> {
        let Some((text, why)) = texts.find_map(|t| memory::refusal(t).map(|why| (t, why))) else {
            return Ok(());
        };
        let text = format!(
            "The agent tried to {what}: {} (refused: it {why})",
            memory::shown(text)
        );
        let live = self.live(id)?;
        let mut live = lock(&live);
        let item = live.chat.push(now(), 0, Body::Note { text });
        live.publish(self, vec![Change::Item(item)])?;
        Err(HubError::Refused(format!(
            "refused: it {why}. The owner sees it in the thread and can add it by hand."
        )))
    }

    /// What persona `persona`'s threads said after line `mark` (#219), routine runs left out: none when the owner
    /// said nothing since. For its `heartbeat` (#498) routines' results count and only its own runs are left out:
    /// none when nothing was said since.
    ///
    /// # Errors
    /// [`HubError::Refused`] when a thread can't be read.
    pub fn news(
        &self,
        persona: &str,
        (mark, heartbeat): (u64, bool),
    ) -> Result<Option<(String, u64)>, HubError> {
        let mine: Vec<String> = self
            .all()
            .iter()
            .map(|l| lock(l).meta.clone())
            .filter(|m| {
                m.profile == persona
                    && m.routine
                        .as_ref()
                        .is_none_or(|r| heartbeat && r.id != agents::HEARTBEAT)
            })
            .map(|m| m.id)
            .collect();
        history::since(&self.store, &mine, (mark, !heartbeat))
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    /// An agent's `history` in thread `id` (#217): a search of its persona's threads (`query`, at most `limit`
    /// hits), or a page of one of them (`thread`, before item `before`). In Chief's own chat (#664) `whose` names
    /// another persona to search, and any thread may be read.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a thread of another persona.
    pub fn history(
        &self,
        (id, token, whose): (&str, &str, &str),
        (query, limit): (&str, usize),
        thread: &str,
        before: Option<u64>,
    ) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let chief = self.chief_of(id, token).is_ok();
        let whose = match (whose, chief) {
            ("", _) => meta.profile.as_str(),
            (w, true) => w,
            _ => {
                return Err(HubError::Refused(
                    "NOT_CHIEF: only Chief reads another persona's history".to_owned(),
                ));
            }
        };
        let persona = agents::find(self.config_dir(), &self.data, whose)
            .map_err(|_| HubError::Refused(format!("no persona {whose}")))?;
        let refused = |e: cooee_core::StoreError| HubError::Refused(e.to_string());
        let run = meta
            .routine
            .as_ref()
            .is_some_and(|r| r.id == agents::CONSOLIDATE);
        if run && thread.is_empty() && query.trim().is_empty() {
            let key = format!("{}-{}", agents::CONSOLIDATE, persona.id);
            let read = history::watermark(&self.data, &key, |w| {
                let news = self.news(&persona.id, (w.line, false));
                if let Ok(Some((_, newest))) = &news {
                    w.line = *newest;
                }
                news
            });
            return read
                .map_err(|e| HubError::Refused(e.to_string()))?
                .map(|n| {
                    n.map_or_else(
                        || "nothing new since the last run".to_owned(),
                        |(text, _)| text,
                    )
                });
        }
        if thread.is_empty() && query.trim().is_empty() {
            return Err(HubError::Refused("give a query or a thread".to_owned()));
        }
        if thread.is_empty() {
            let mine: Vec<String> = self
                .all()
                .iter()
                .map(|l| lock(l).meta.clone())
                .filter(|m| m.profile == persona.id)
                .map(|m| m.id)
                .collect();
            return cooee_core::history::search(&self.store, &mine, query, limit).map_err(refused);
        }
        match self.thread(thread) {
            Ok((t, _)) if chief || t.profile == persona.id => {
                cooee_core::history::read(&self.store, thread, before).map_err(refused)
            }
            _ => Err(HubError::Refused(format!(
                "thread {thread} isn't one of {}'s",
                persona.id
            ))),
        }
    }
}
