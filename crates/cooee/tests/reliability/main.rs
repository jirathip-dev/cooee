//! Reliability (#24, herdsman#553's "Reliability" rows): herdsman's fixed bug classes that apply to cooee, each
//! run against `cooee serve` and the fake shed (tests/support). Rows proven in a crate's own tests name it here.
//! R2: cooee-core `upgrade_compat`; R6, R15: cooee-server `hub`; R16: cooee-core `chat`; R18: cooee-core `devices`.

#[path = "../support/mod.rs"]
mod support;

mod delivery;
mod keys;
mod requests;
mod setup;
mod starts;
