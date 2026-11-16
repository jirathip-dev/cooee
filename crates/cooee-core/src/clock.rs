//! The one clock (#203): CI's arch-check bans `SystemTime::now`, `Instant::now` and `thread::sleep` elsewhere.

use std::{
    thread,
    time::{Duration, Instant, SystemTime},
};

/// The wall clock's time now.
#[must_use]
pub fn now() -> SystemTime {
    SystemTime::now()
}

/// The monotonic clock's instant now.
#[must_use]
pub fn instant() -> Instant {
    Instant::now()
}

/// Blocks this thread for `d`.
pub fn sleep(d: Duration) {
    thread::sleep(d);
}
