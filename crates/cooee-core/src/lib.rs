//! cooee's core: configuration, threads, their store and chat, and the shed client. No HTTP.

#![forbid(unsafe_code)]

mod chat;
mod config;
mod shed;
mod store;

pub use chat::{Body, Change, Chat, Item, Summary, Turn};
pub use config::{Config, ConfigError};
pub use shed::{Agent, Shed, ShedError};
pub use shed_proto;
pub use store::{FORMAT, Message, Role, Store, StoreError, Thread};
