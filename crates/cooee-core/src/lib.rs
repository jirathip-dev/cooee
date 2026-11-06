//! cooee's core: configuration, threads, their store and chat, paired devices, provider sign-ins and the shed client. No HTTP.

#![forbid(unsafe_code)]

mod chat;
mod config;
mod devices;
mod login;
mod shed;
mod store;

pub use chat::{Body, Change, Chat, Item, Summary, Turn};
pub use config::{Config, ConfigError, Plugin, ROOTS_FIX, save_roots};
pub use devices::{Claim, Device, Devices, random, sha256, write_private};
pub use login::{Provider, Status, links, redact};
pub use shed::{AGENTS_FIX, Agent, Shed, ShedError};
pub use shed_proto;
pub use store::{FORMAT, Message, Role, Store, StoreError, Thread};
