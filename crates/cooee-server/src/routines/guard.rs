use cooee_core::{
    history::{self, Watermark},
    routines::{Cap, Change, Held, Outcome, Place, Routine, Skip, Tally, Turns, day},
};

use super::Runner;
use crate::{http::refused, hub::HubError};

/// Settings › routines (#690): the default cap and each routine with its cap and today's tally.
pub struct Guards {
    /// The default cap.
    pub per_day: Cap,
    /// The routines.
    pub rows: Vec<Guarded>,
}

/// A routine as Settings › routines shows it.
pub struct Guarded {
    /// The routine: its `per_day` says whether the cap is its own.
    pub routine: Routine,
    /// When it runs, with its end.
    pub when: String,
    /// Its cap; none for a `post` script.
    pub cap: Option<Cap>,
    /// What it did today.
    pub today: Tally,
}

impl Runner {
    fn mark<T>(&self, r: &Routine, f: impl FnOnce(&mut Watermark) -> T) -> Option<T> {
        let key = format!("routine-{}", r.id);
        history::watermark(&self.data, &key, f)
            .inspect_err(|e| cooee_core::note!("cooee: routines: {key}: {e}"))
            .ok()
    }

    pub(super) fn cap(&self, r: &Routine) -> Cap {
        r.cap(self.file.load().ok().and_then(|b| b.per_day))
    }

    pub(super) fn admit_due(&self, r: &Routine, now: u64) -> Result<(), Held> {
        if r.turns() != Turns::Always {
            return Ok(());
        }
        let day = day(now, self.offset(now));
        let w = self.mark(r, |w| w.clone()).ok_or(Held::Idle)?;
        let change = w
            .fresh_or_new_day(day)
            .or_else(|| self.wrote(r, w.line).then_some(Change::Said));
        let verdict = w.check(day, self.cap(r), change).map(|_| ());
        if let Err(why) = verdict {
            self.mark(r, |w| w.held(day, why));
        }
        verdict
    }

    pub(super) fn admit_event(&self, r: &Routine, now: u64) -> Result<(), Skip> {
        if r.turns() != Turns::Always {
            return Ok(());
        }
        self.admit_now(r, Change::Event, now)
    }

    pub(super) fn admit_wake(&self, r: &Routine, now: u64) -> Result<(), Skip> {
        self.admit_now(r, Change::Woke, now)
    }

    fn admit_now(&self, r: &Routine, change: Change, now: u64) -> Result<(), Skip> {
        let (day, cap) = (day(now, self.offset(now)), self.cap(r));
        let admitted = self.mark(r, |w| match w.check(day, cap, Some(change)) {
            Ok(_) => {
                w.ran(day, now);
                true
            }
            Err(why) => {
                w.held(day, why);
                false
            }
        });
        if admitted == Some(true) {
            Ok(())
        } else {
            Err(Skip::Capped)
        }
    }

    pub(super) fn count(&self, r: &Routine, outcome: &Outcome, now: u64) {
        if r.turns() == Turns::Always && matches!(outcome, Outcome::Ran { .. }) {
            let day = day(now, self.offset(now));
            self.mark(r, |w| w.ran(day, now));
        }
    }

    pub(super) fn fresh(&self, r: &Routine) {
        self.mark(r, |w| w.line = 0);
    }

    pub(super) fn forget(&self, r: &Routine) {
        self.mark(r, |w| *w = Watermark::default());
    }

    fn wrote(&self, r: &Routine, line: u64) -> bool {
        let profile = match &r.place {
            Place::Chat { thread } => self.hub.thread(thread).map(|(t, _)| t.profile),
            Place::NewThread { who } => Ok(who.profile.clone()),
        };
        profile
            .and_then(|p| self.hub.news(&p, (line, false)))
            .map_or(true, |news| news.is_some())
    }

    /// Settings › routines at `now`: the default cap and each routine's cap and tally.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read.
    pub fn guards(&self, now: u64) -> Result<Guards, HubError> {
        let book = self.file.load().map_err(refused)?;
        let day = day(now, self.offset(now));
        let rows = book.routines.into_iter().map(|r| {
            let today = self.mark(&r, |w| w.today(day)).unwrap_or_default();
            Guarded {
                when: self.when(&r, now),
                cap: (r.turns() != Turns::Never).then(|| r.cap(book.per_day)),
                today,
                routine: r,
            }
        });
        Ok(Guards {
            per_day: book.per_day.unwrap_or(Cap::DEFAULT),
            rows: rows.collect(),
        })
    }

    /// Settings › routines' default cap, set to `per_day`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for `BAD_CAP` or when the file can't be written.
    pub fn set_default(&self, per_day: &str) -> Result<Cap, HubError> {
        let cap = Cap::parse(per_day).map_err(refused)?;
        self.change(|b| {
            b.per_day = Some(cap);
            Ok(cap)
        })
    }
}
