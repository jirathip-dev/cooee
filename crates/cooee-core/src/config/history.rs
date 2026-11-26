use serde::Deserialize;

/// `[history]`: thread retention (#364): which items stay in full in `cooee.db` and which move to `<data>/archive/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct History {
    /// An ended thread quiet for this many days moves to its month's archive (default 30).
    pub hot_days: u64,
    /// A tool item this many days old is trimmed when over `trim_over_kb` (default 7).
    pub trim_after_days: u64,
    /// A tool item's text over this many KB keeps its head and tail; the archive keeps it whole (default 4).
    pub trim_over_kb: u64,
}

impl Default for History {
    fn default() -> Self {
        Self {
            hot_days: 30,
            trim_after_days: 7,
            trim_over_kb: 4,
        }
    }
}
