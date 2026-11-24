use serde::{Deserialize, Serialize};

use crate::routines::Schedule;

/// The consolidation routine's mark on its runs (#219), and its watermark's key with the persona's id after a `-`.
pub const CONSOLIDATE: &str = "consolidate";

/// What a consolidation run is told (#219).
pub const CONSOLIDATION: &str = "This is your consolidation run. Call history once with no arguments: it gives what \
was said since the last run. Then propose changes to your notes with remember, preferring to replace or merge a note \
over adding one, and a skill with skill propose only for something done at least twice. Each is a card the owner \
approves; nothing is saved before. Stop after this one pass.";

/// A persona's consolidation routine (#219): off unless `on`; at `when` (default nightly, 03:00) a run reads its new
/// history and proposes notes and skills, on `model` and `effort` (empty: its provider's cheapest).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Consolidate {
    /// Switched on.
    pub on: bool,
    /// A 5-field cron in the Mac's local time (none: nightly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<Schedule>,
    /// Its model (empty: the provider's cheapest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Its effort (empty: the provider's lowest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
}

impl Consolidate {
    /// When it runs: `when`, else nightly at 03:00.
    #[must_use]
    pub fn schedule(&self) -> Option<Schedule> {
        self.when
            .clone()
            .or_else(|| Schedule::parse("0 3 * * *").ok())
    }
}

/// The heartbeat's mark on its runs (#498), and its watermark's key with the persona's id after a `-`.
pub const HEARTBEAT: &str = "heartbeat";

/// What a heartbeat run is told (#498), its notify rule and what is new after it.
pub const BEAT: &str = "This is your heartbeat run. Below is what is new in your threads and your routines' results \
since the last run; look at your attached connectors too where they bear on it. Decide in this one pass and act on \
nothing: anything you would do waits on a card the owner answers. End your reply with one last line: NOTIFY when \
what you found matches the owner's notify rule, else SILENT: and why.";

/// A persona's heartbeat (#498): off unless `on`; at `when` (default hourly) a run looks at what is new and stays
/// silent unless its notify rule matches, on `model` and `effort` (empty: its provider's cheapest), at most `per_day`
/// runs a local day (none: [`Heartbeat::PER_DAY`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Heartbeat {
    /// Switched on.
    pub on: bool,
    /// A 5-field cron in the Mac's local time (none: hourly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<Schedule>,
    /// Its model (empty: the provider's cheapest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Its effort (empty: the provider's lowest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
    /// At most this many runs a local day.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_day: Option<crate::routines::Cap>,
}

impl Heartbeat {
    /// The runs a local day when `per_day` is unset.
    pub const PER_DAY: crate::routines::Cap = crate::routines::Cap(4);

    /// When it runs: `when`, else hourly on the hour.
    #[must_use]
    pub fn schedule(&self) -> Option<Schedule> {
        self.when
            .clone()
            .or_else(|| Schedule::parse("0 * * * *").ok())
    }
}
