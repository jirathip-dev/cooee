#![allow(dead_code, reason = "each test file uses part of the support")]

mod agent;
mod cooee;
mod events;
mod shed;

#[allow(unused_imports, reason = "a test file may use none of an area's items")]
pub use {agent::*, cooee::*, events::*, shed::*};
