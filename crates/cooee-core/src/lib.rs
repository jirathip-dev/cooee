//! cooee's core: configuration, threads and their store. No sockets, no HTTP.

#![forbid(unsafe_code)]

mod config;
mod store;

pub use config::{Config, ConfigError};
pub use store::{FORMAT, Message, Role, Store, StoreError, Thread};
