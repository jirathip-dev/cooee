use std::{
    sync::{Arc, Mutex, mpsc},
    time::Duration,
};

use cooee_core::{
    agents::Agent,
    connectors::{Class, Target, Tools},
    mcp::{Pick, Server, Source},
};
use serde_json::Value;

use super::{TEN_MINUTES, lock};
use crate::Hub;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct Permit {
    pub(super) thread: String,
    pub(super) server: String,
    pub(super) tool: String,
    pub(super) args: String,
}

pub(super) type Permits = Arc<Mutex<Vec<(Permit, u64)>>>;

pub(super) enum Decision {
    Allow,
    Ask,
    Deny,
}

pub(super) fn tools(s: &Server) -> Tools {
    match s.source() {
        Ok(Source::Catalogue(e)) => e.tools.clone(),
        Ok(Source::Typed { read, .. }) => Tools {
            read: read.to_vec(),
            ..Tools::default()
        },
        Ok(Source::Command { .. }) | Err(_) => Tools::default(),
    }
}

pub(super) fn decide(s: &Server, tool: &str) -> Decision {
    let builtin = matches!(s.source(), Ok(Source::Catalogue(e)) if matches!(e.target, Target::Builtin { .. }));
    match tools(s).class(tool) {
        Class::Read => Decision::Allow,
        Class::Write if !builtin => Decision::Ask,
        Class::Write | Class::Never => Decision::Deny,
    }
}

pub(super) fn offered(mut msg: Value, s: &Server) -> Value {
    let all = tools(s);
    if let Some(list) = msg
        .pointer_mut("/result/tools")
        .and_then(Value::as_array_mut)
    {
        list.retain(|t| t["name"].as_str().map(|n| all.class(n)) != Some(Class::Never));
    }
    msg
}

impl super::Connectors {
    fn permitted(&self, key: &Permit, now: u64) -> bool {
        let mut held = lock(&self.permits);
        held.retain(|(_, at)| now.saturating_sub(*at) <= TEN_MINUTES);
        let at = held.iter().position(|(k, _)| k == key);
        at.map(|i| held.swap_remove(i)).is_some()
    }

    pub(super) fn ask(
        &self,
        hub: &Hub,
        (persona, pick): (&Agent, &Pick),
        key: &Permit,
        now: u64,
    ) -> Result<(), String> {
        if self.permitted(key, now) {
            return Ok(());
        }
        let account = pick
            .account
            .as_deref()
            .map(|a| format!(" ({a})"))
            .unwrap_or_default();
        let what = format!(
            "{} · {}{account}: {} {}",
            persona.name, key.server, key.tool, key.args
        );
        let (tx, rx) = mpsc::channel();
        let (permits, k) = (Arc::clone(&self.permits), key.clone());
        let allowed = Box::new(move || {
            lock(&permits).push((k, crate::hub::now()));
            tx.send(()).ok();
            Ok("allowed".to_owned())
        });
        hub.ask_card(&key.thread, (what, key.args.clone()), allowed)
            .map_err(|e| e.to_string())?;
        match rx.recv_timeout(Duration::from_millis(TEN_MINUTES)) {
            Ok(()) if self.permitted(key, now) => Ok(()),
            Err(mpsc::RecvTimeoutError::Timeout) => Err(format!(
                "{} waits for the owner's answer on its card: call it again with the same arguments once they allow it",
                key.tool
            )),
            _ => Err(format!(
                "the owner didn't allow {}: its card was declined or expired",
                key.tool
            )),
        }
    }
}
