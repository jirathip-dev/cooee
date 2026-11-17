pub use crate::accounts::{Account, Accounts, Sheet, Usage, Window};
pub use crate::chat::{Body, Card, Change, Chat, Item, Summary, Turn};
pub use crate::config::{
    Apns, ApnsEnv, Config, ConfigError, Perf, Plugin, ROOTS_FIX, env_var, save_roots,
};
pub use crate::devices::{
    Asking, Claim, Credential, Device, Devices, Passkey, append_line, label, random, sha256,
    write_private,
};
pub use crate::login::{Ran, Spec, Status, links, redact};
pub use crate::profiles::Profile;
pub use crate::provider::{Entry, Plain, Provider, Providers, agents_fix};
pub use crate::shed::{Agent, Launch, SHED_CARGO, SHED_LABEL, Shed, ShedError, shed_fix};
pub use crate::store::{
    FORMAT, Kind, Member, Message, Queued, Reason, Role, RoomTurn, Store, StoreError, Thread,
    Workflow,
};
pub use shed_proto;

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
            writeln!(out, "{} = {v}", key(k)).ok();
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
            writeln!(out, "\n{open}{path}{close}").ok();
            toml_table(out, &format!("{path}."), item);
        }
    }
}
