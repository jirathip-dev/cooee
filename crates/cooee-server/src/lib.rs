//! cooee's HTTP server: the native app's JSON API (docs/api.md).

#![forbid(unsafe_code)]

mod gate;
mod http;
mod hub;
mod layout;
mod login;
mod look;
pub mod machine;
mod manage;
pub mod mcp;
mod pairing;
mod passkey;
mod plugin;
mod push;
mod root;
mod routes;
mod routines;
mod screens;
mod serve;
mod themes;

pub use root::*;
