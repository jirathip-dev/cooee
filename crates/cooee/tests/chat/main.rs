//! Chat end to end: `cooee serve` against a fake shed server on a Unix socket (tests/support). A new thread, a
//! prompt and its streamed reply, a tool call, an approval, a cancel, a reconnect with `Last-Event-ID`, a restart
//! that re-attaches, closing, and a shed of another version.

#[path = "../support/mod.rs"]
mod support;

mod protocol;
mod reconnect;
mod screen;
mod setup;
mod turns;
