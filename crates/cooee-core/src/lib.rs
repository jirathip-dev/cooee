//! cooee's core: configuration, threads, their store and chat, paired devices and the shed client. No HTTP.

#![forbid(unsafe_code)]

mod chat;
mod config;
mod devices;
mod shed;
mod store;

pub use chat::{Body, Change, Chat, Item, Summary, Turn};
pub use config::{Config, ConfigError};
pub use devices::{Device, Devices, random, sha256, write_private};
pub use shed::{Agent, Shed, ShedError};
pub use shed_proto;
pub use store::{FORMAT, Message, Role, Store, StoreError, Thread};
