//! cooee's core: configuration, threads, their store and chat, paired devices, provider sign-ins and accounts, installed plugins, agent profiles and the shed client. No HTTP.

#![forbid(unsafe_code)]

mod accounts;
pub mod agents;
mod blocks;
mod chat;
mod config;
mod devices;
pub mod group;
pub mod history;
mod login;
pub mod mcp;
pub mod memory;
pub mod migrate;
pub mod models;
pub mod perf;
pub mod plugins;
pub mod profiles;
pub mod provider;
mod providers;
pub mod routines;
pub mod secrets;
mod shed;
pub mod skills;
mod store;

pub use accounts::{Account, Accounts, Sheet, Usage, Window};
pub use chat::{Body, Card, Change, Chat, Item, Summary, Turn};
pub use config::{Apns, ApnsEnv, Config, ConfigError, Perf, Plugin, ROOTS_FIX, save_roots};
pub use devices::{
    Asking, Claim, Credential, Device, Devices, Passkey, append_line, label, random, sha256,
    write_private,
};
pub use login::{Ran, Spec, Status, links, redact};
pub use profiles::Profile;
pub use provider::{Entry, Plain, Provider, Providers, agents_fix};
pub use shed::{Agent, Launch, Shed, ShedError};
pub use shed_proto;
pub use store::{
    FORMAT, Kind, Member, Message, Queued, Reason, Role, RoomTurn, Store, StoreError, Thread,
    Workflow,
};

/// `v`'s object as a TOML file, by the existing idiom (a JSON string, number, bool or list of them is TOML): each
/// object a `[table]`, each list of objects `[[tables]]`; nulls, empty strings and lists, and `false` left out.
pub(crate) fn toml_text(v: &serde_json::Value) -> String {
    let mut out = String::new();
    toml_table(&mut out, "", v);
    out.trim_start().to_owned()
}

fn toml_table(out: &mut String, path: &str, v: &serde_json::Value) {
    use std::fmt::Write as _;

    use serde_json::Value as V;
    let bare = |k: &str| {
        !k.is_empty()
            && k.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    };
    let key = |k: &str| {
        if bare(k) {
            k.to_owned()
        } else {
            V::from(k).to_string()
        }
    };
    let nested = |v: &V| {
        v.is_object()
            || v.as_array()
                .is_some_and(|a| a.first().is_some_and(V::is_object))
    };
    let Some(m) = v.as_object() else { return };
    for (k, v) in m {
        let empty = matches!(v, V::Null | V::Bool(false))
            || v.as_str() == Some("")
            || v.as_array().is_some_and(Vec::is_empty);
        if !empty && !nested(v) {
            let _ = writeln!(out, "{} = {v}", key(k));
        }
    }
    for (k, v) in m.iter().filter(|(_, v)| nested(v)) {
        let path = format!("{path}{}", key(k));
        let (items, open, close) = match v {
            V::Array(items) => (items.iter().collect(), "[[", "]]"),
            v if v.as_object().is_some_and(|o| !o.is_empty()) => (vec![v], "[", "]"),
            _ => (Vec::new(), "", ""),
        };
        for item in items {
            let _ = writeln!(out, "\n{open}{path}{close}");
            toml_table(out, &format!("{path}."), item);
        }
    }
}
