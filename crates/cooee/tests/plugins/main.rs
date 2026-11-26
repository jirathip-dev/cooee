//! Plugins end to end: `cooee serve` with two plugins, each on a fake backend that records what it is sent. The
//! proxy carries the plugin's own token and never the device's cookie, an event stream comes through, a backend
//! posts with its token or is refused, and a choice card's answer goes to its own plugin.

#[path = "../support/mod.rs"]
mod support;

mod action_cards;
mod check;
mod choice_cards;
mod proxy;
mod push;
mod setup;
