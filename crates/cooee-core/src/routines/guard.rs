use serde::{Deserialize, Serialize};

use crate::history::Watermark;

const DAY: u64 = 86_400_000;

/// At most this many agent turns of one routine a local day: 1 to 144 (six an hour, the most an event routine can run).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Cap(pub(crate) u32);

impl Cap {
    /// The cap until the owner sets a default.
    pub const DEFAULT: Self = Self(24);

    /// `BAD_CAP: a cap is 1 to 144 runs a day` for anything else.
    ///
    /// # Errors
    /// That sentence.
    pub fn parse(text: &str) -> Result<Self, String> {
        let n = text.trim().parse::<u32>().map_err(|_| bad())?;
        Self::try_from(n)
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

fn bad() -> String {
    "BAD_CAP: a cap is 1 to 144 runs a day".to_owned()
}

impl TryFrom<u32> for Cap {
    type Error = String;

    fn try_from(n: u32) -> Result<Self, String> {
        if (1..=144).contains(&n) {
            Ok(Self(n))
        } else {
            Err(bad())
        }
    }
}

impl From<Cap> for u32 {
    fn from(c: Cap) -> Self {
        c.0
    }
}

/// When a run of a routine is an agent turn: what the guard checks and counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Turns {
    /// Every run: a prompt routine.
    Always,
    /// Only when its script exits non-zero: a `wake` script.
    OnWake,
    /// Never: a `post` script. No idle skip, no cap, no tally.
    Never,
}

/// What changed since a routine's last run: why this one may go ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// An event came: a GitHub delivery or a signed POST.
    Event,
    /// Its wake script exited non-zero.
    Woke,
    /// It has not run since it was made, its prompt or schedule was edited, or it was resumed.
    Fresh,
    /// It has not run yet this local day.
    NewDay,
    /// The owner wrote after its last run (the heartbeat: anything was said).
    Said,
}

/// Why the guard held a run back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// Nothing changed.
    Idle,
    /// It reached its cap today.
    Capped,
}

/// What a routine did on one local day.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    /// The local day, in days since 1970.
    pub day: u64,
    /// Agent turns started.
    pub ran: u32,
    /// Due times skipped because nothing changed.
    pub idle: u32,
    /// Due times, events and wakes skipped at the cap.
    pub capped: u32,
}

impl Tally {
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The local day of `now` (ms) in UTC offset `offset_min`.
#[must_use]
pub fn day(now: u64, offset_min: i32) -> u64 {
    now.saturating_add_signed(i64::from(offset_min) * 60_000) / DAY
}

impl Watermark {
    /// `day`'s tally: zeros once the day has changed.
    #[must_use]
    pub fn today(&self, day: u64) -> Tally {
        if self.today.day == day {
            self.today
        } else {
            Tally {
                day,
                ..Tally::default()
            }
        }
    }

    /// `Fresh` or `NewDay` when so; else none, and the caller asks whether something was said after `line`.
    #[must_use]
    pub fn fresh_or_new_day(&self, day: u64) -> Option<Change> {
        if self.line == 0 {
            Some(Change::Fresh)
        } else if self.today(day).ran == 0 {
            Some(Change::NewDay)
        } else {
            None
        }
    }

    /// Whether a turn may go ahead: `Idle` without a change, `Capped` at `cap` turns today. Counts nothing.
    ///
    /// # Errors
    /// Why not.
    pub fn check(&self, day: u64, cap: Cap, change: Option<Change>) -> Result<Change, Held> {
        let change = change.ok_or(Held::Idle)?;
        if self.today(day).ran >= cap.get() {
            return Err(Held::Capped);
        }
        Ok(change)
    }

    /// Counts a turn and moves the line to `line`.
    pub fn ran(&mut self, day: u64, line: u64) {
        self.today = self.today(day);
        self.today.ran += 1;
        self.line = line;
    }

    /// Counts a held turn; the line stays, so the news waits.
    pub fn held(&mut self, day: u64, why: Held) {
        self.today = self.today(day);
        match why {
            Held::Idle => self.today.idle += 1,
            Held::Capped => self.today.capped += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_says_idle_before_capped_and_counts_nothing() {
        let mut w = Watermark::default();
        let cap = Cap::parse("1").unwrap();
        assert_eq!(w.fresh_or_new_day(5), Some(Change::Fresh));
        w.ran(5, 100);
        assert_eq!(w.fresh_or_new_day(5), None);
        let was = w.clone();
        assert_eq!(w.check(5, cap, None), Err(Held::Idle));
        assert_eq!(w.check(5, cap, Some(Change::Said)), Err(Held::Capped));
        assert_eq!(w, was);
        w.held(5, Held::Capped);
        w.held(5, Held::Idle);
        let t = w.today(5);
        assert_eq!((t.ran, t.idle, t.capped, w.line), (1, 1, 1, 100));
    }

    #[test]
    fn a_new_local_day_starts_the_tally_at_zero_and_counts_as_a_change() {
        let mut w = Watermark::default();
        w.ran(5, 100);
        w.held(5, Held::Idle);
        assert_eq!(
            w.today(6),
            Tally {
                day: 6,
                ..Tally::default()
            }
        );
        assert_eq!(w.fresh_or_new_day(6), Some(Change::NewDay));
        assert_eq!(
            w.check(6, Cap::parse("1").unwrap(), Some(Change::NewDay)),
            Ok(Change::NewDay)
        );
        assert_eq!(day(DAY - 1, 0), 0);
        assert_eq!(day(DAY - 60_000, 1), 1);
        assert_eq!(day(DAY, -1), 0);
    }

    #[test]
    fn a_cap_is_one_to_one_hundred_forty_four() {
        assert_eq!(Cap::parse(" 144 ").map(Cap::get), Ok(144));
        for bad in ["0", "145", "x", "", "-1"] {
            assert!(Cap::parse(bad).unwrap_err().starts_with("BAD_CAP"), "{bad}");
        }
        assert!(serde_json::from_str::<Cap>("0").is_err());
    }
}
