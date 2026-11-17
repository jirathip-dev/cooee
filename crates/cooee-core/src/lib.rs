//! cooee's core: the model and its files. No HTTP.

#![forbid(unsafe_code)]

mod accounts;
pub mod action;
pub mod agents;
mod blocks;
mod chat;
pub mod clock;
mod config;
pub mod connectors;
mod devices;
pub mod disk;
pub mod gitignore;
pub mod group;
pub mod history;
pub mod library;
mod login;
pub mod logs;
pub mod mcp;
pub mod memory;
pub mod migrate;
pub mod models;
pub mod out;
pub mod perf;
pub mod plugins;
pub mod profiles;
pub mod provider;
mod providers;
mod root;
pub mod routines;
pub mod runner;
pub mod scratch;
pub mod secrets;
mod shed;
pub mod skills;
mod store;
pub mod swag;

pub use root::*;
