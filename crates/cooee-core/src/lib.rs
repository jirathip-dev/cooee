//! cooee's core: configuration, threads, their store and chat, paired devices, provider sign-ins and accounts, installed plugins, agent profiles and the shed client. No HTTP.

#![forbid(unsafe_code)]

mod accounts;
mod chat;
mod config;
mod devices;
mod login;
pub mod plugins;
pub mod profiles;
mod shed;
mod store;

pub use accounts::{Account, Accounts, Sheet};
pub use chat::{Body, Change, Chat, Item, Summary, Turn};
pub use config::{Config, ConfigError, Plugin, ROOTS_FIX, save_roots};
pub use devices::{Claim, Device, Devices, random, sha256, write_private};
pub use login::{Provider, Status, links, redact};
pub use profiles::Profile;
pub use shed::{AGENTS_FIX, Agent, Launch, Shed, ShedError};
pub use shed_proto;
pub use store::{FORMAT, Message, Role, Store, StoreError, Thread};
