#![allow(dead_code, reason = "each test file uses part of the support")]

mod accounts;
mod agent;
mod cooee;
mod events;
mod passkey;
mod shed;

#[allow(unused_imports, reason = "a test file may use none of an area's items")]
pub use {accounts::*, agent::*, cooee::*, events::*, passkey::*, shed::*};
