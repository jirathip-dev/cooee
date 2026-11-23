use std::{collections::HashMap, sync::Mutex};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use cooee_core::{
    Config,
    browser::{self, Control},
};
use cooee_desk::{Desks, Name, State as Life};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Hub, HubError, Input, lock, now};

/// An agent's `desk` (#172): one action on its persona's desk.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum DeskAsk {
    /// A picture of the whole screen, for the agent.
    Screenshot {},
    /// A click at `x`, `y` pixels from the top left.
    Click {
        /// Pixels from the left.
        x: u32,
        /// Pixels from the top.
        y: u32,
    },
    /// Text typed into what has the focus.
    Type {
        /// What is typed.
        text: String,
    },
}

/// What a desk action answers the agent: a line, and the screen when it asked for a screenshot.
#[derive(Debug, Serialize)]
pub struct DeskSeen {
    /// What was done.
    pub text: String,
    /// The PNG, base64, of a screenshot the agent asked for.
    pub image: Option<String>,
}

#[derive(Default)]
pub(super) struct Leases(Mutex<HashMap<String, Control>>);

/// One frame of a desk's screen (#171): its JPEG, base64.
#[derive(Debug, Serialize)]
pub struct DeskFrame {
    jpeg: String,
}

fn refused(e: &impl ToString) -> HubError {
    HubError::Refused(e.to_string())
}

const IDLE: u64 = 15 * 60_000;

#[derive(Default)]
pub(super) struct State {
    leases: Leases,
    used: Mutex<HashMap<String, u64>>,
}

impl Hub {
    pub(crate) fn pre_warm(&self, id: &str) -> Result<(), HubError> {
        let profile = self.thread(id)?.0.profile;
        let Some(desk) = self.persona(&profile).desk else {
            return Ok(());
        };
        self.awake(&desk).map(drop)
    }

    pub(super) fn sleep_idle(&self, now: u64) {
        let awake = self.desks().and_then(|d| d.made().map_err(|e| refused(&e)));
        let mut used = lock(&self.areas.desk.used);
        for d in awake.iter().flatten().filter(|d| d.state == Life::Awake) {
            used.entry(d.name.clone()).or_insert(now);
        }
        let mut idle = Vec::new();
        used.retain(|desk, at| {
            let keep = now < at.saturating_add(IDLE);
            if !keep {
                idle.push(desk.clone());
            }
            keep
        });
        drop(used);
        for desk in idle {
            let slept = self
                .made(&desk)
                .and_then(|(desks, name)| desks.sleep(&name).map_err(|e| refused(&e)));
            if let Err(e) = slept {
                cooee_core::note!("cooee: desk {desk}: {e}");
            }
        }
    }

    fn awake(&self, desk: &str) -> Result<(Desks, Name), HubError> {
        let (desks, name) = self.made(desk)?;
        lock(&self.areas.desk.used).insert(desk.to_owned(), now());
        if desks.state(&name).map_err(|e| refused(&e))? == Life::Asleep {
            desks.wake(&name).map_err(|e| refused(&e))?;
        }
        Ok((desks, name))
    }

    fn desks(&self) -> Result<Desks, HubError> {
        let config = self.config.as_deref().map(Config::load).transpose();
        let container = config.map_err(|e| refused(&e))?.and_then(|c| c.container);
        Ok(Desks::new(
            container.ok_or_else(|| refused(&"no desks here"))?,
            Vec::new(),
        ))
    }

    fn made(&self, desk: &str) -> Result<(Desks, Name), HubError> {
        let desks = self.desks()?;
        let name = Name::new(desk).map_err(|e| refused(&e))?;
        if !desks
            .made()
            .map_err(|e| refused(&e))?
            .iter()
            .any(|d| d.name == desk)
        {
            return Err(refused(&format!("no desk {desk}")));
        }
        Ok((desks, name))
    }

    fn held(&self, desk: &str) -> Control {
        let map = lock(&self.areas.desk.leases.0);
        let held = map
            .get(desk)
            .filter(|c| matches!(c, Control::Owner { until } if now() < *until));
        held.cloned().unwrap_or_default()
    }

    /// Who drives desk `desk`: the agent, or the owner while their lease runs.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a desk cooee didn't make.
    pub fn desk_screen(&self, desk: &str) -> Result<Control, HubError> {
        self.made(desk)?;
        Ok(self.held(desk))
    }

    /// The owner takes control of desk `desk` for 10 minutes, or 10 more while they hold it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a desk cooee didn't make.
    pub fn desk_take(&self, desk: &str) -> Result<Control, HubError> {
        self.awake(desk)?;
        let taken = self.held(desk).take(now());
        lock(&self.areas.desk.leases.0).insert(desk.to_owned(), taken.clone());
        Ok(taken)
    }

    /// The owner hands desk `desk` back.
    ///
    /// # Errors
    /// [`HubError::Refused`] unless they hold it.
    pub fn desk_back(&self, desk: &str) -> Result<Control, HubError> {
        match lock(&self.areas.desk.leases.0).remove(desk) {
            Some(Control::Owner { .. }) => Ok(Control::Agent),
            _ => Err(refused(&format!("you don't hold desk {desk}"))),
        }
    }

    /// The owner's `input` on desk `desk`, only while they hold its lease.
    ///
    /// # Errors
    /// [`HubError::Refused`] unless they hold it, or when the desk doesn't take it.
    pub fn desk_input(&self, desk: &str, input: &Input) -> Result<(), HubError> {
        if self.held(desk) == Control::Agent {
            return Err(refused(&format!("take control of desk {desk} first")));
        }
        let (desks, name) = self.awake(desk)?;
        match input {
            Input::Click { x, y } => desks.click(&name, *x, *y),
            Input::Text(text) => desks.type_text(&name, text),
        }
        .map_err(|e| refused(&e))
    }

    /// The live view's next frame of desk `desk`, captured inside it now.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a desk cooee didn't make, or when the capture fails.
    pub fn desk_frame(&self, desk: &str) -> Result<DeskFrame, HubError> {
        let (desks, name) = self.made(desk)?;
        let jpeg = desks.frame(&name).map_err(|e| refused(&e))?;
        Ok(DeskFrame {
            jpeg: STANDARD.encode(jpeg),
        })
    }

    /// An agent's `desk` in thread `id`, on the desk its persona names (one the provider made): the action is
    /// logged in the persona's replay log, and the screen after it kept as the thread's replay frame, encrypted for
    /// 14 days; only a screenshot it asked for reaches the agent.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, no desk, one the provider didn't make, or the desk's own error.
    pub fn desk(&self, id: &str, token: &str, ask: &DeskAsk) -> Result<DeskSeen, HubError> {
        let persona = self.persona(&self.token_of(id, token)?.profile);
        let no_desk = |why: &dyn std::fmt::Display| HubError::Refused(format!("NO_DESK: {why}"));
        let desk = persona
            .desk
            .as_deref()
            .ok_or_else(|| no_desk(&format!("{} has no desk", persona.id)))?;
        let (desks, name) = self.awake(desk).map_err(|e| no_desk(&e))?;
        let (caption, asked) = match ask {
            DeskAsk::Screenshot {} => ("screenshot".to_owned(), true),
            DeskAsk::Click { x, y } => (format!("click {x},{y}"), false),
            DeskAsk::Type { text } => (format!("type {text:?}"), false),
        };
        let line = json!({ "at": now(), "thread": id, "desk": desk, "tool": caption });
        let profile = browser::profile(&self.data, &persona.id);
        browser::log(&profile, &line).map_err(|e| super::refused(&profile, &e))?;
        match ask {
            DeskAsk::Screenshot {} => Ok(()),
            DeskAsk::Click { x, y } => desks.click(&name, f64::from(*x), f64::from(*y)),
            DeskAsk::Type { text } => desks.type_text(&name, text),
        }
        .map_err(|e| refused(&e))?;
        let shot = desks.screenshot(&name).map_err(|e| refused(&e))?;
        self.seal(id, (&caption, asked), &shot)?;
        Ok(DeskSeen {
            text: format!("{caption} on {desk}: done"),
            image: asked.then(|| STANDARD.encode(&shot)),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc};

    use cooee_core::{Config, Shed};

    use super::super::tests::{scratch, seed};
    use super::super::{Hub, Seed, now};
    use super::IDLE;

    const CONTAINER: &str = r#"#!/bin/sh
d=$(dirname "$0")
case "$1" in
ls) printf '[{"configuration":{"id":"ann-desk","labels":{"cooee.provider":"container"}},"status":"%s"}]' "$(cat "$d/status")" ;;
start) printf '%s\n' "$*" >>"$d/calls.log"; echo running >"$d/status" ;;
stop) printf '%s\n' "$*" >>"$d/calls.log"; echo stopped >"$d/status" ;;
*) exit 1 ;;
esac
"#;

    fn desk_hub(dir: &Path, status: &str) -> Arc<Hub> {
        let bin = dir.join("bin");
        fs::create_dir_all(&bin).unwrap();
        fs::write(bin.join("container"), CONTAINER).unwrap();
        fs::set_permissions(bin.join("container"), fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(bin.join("status"), status).unwrap();
        let ann = dir.join("agents/ann");
        fs::create_dir_all(&ann).unwrap();
        let persona = "name = \"ann\"\nagent = \"fake-acp\"\ndesk = \"ann-desk\"\n";
        fs::write(ann.join("agent.toml"), persona).unwrap();
        let config = dir.join("config.toml");
        let container = bin.join("container");
        fs::write(&config, format!("container = {container:?}\n")).unwrap();
        let (looks, _) = crate::look::Looks::load(None, None);
        Hub::open(Seed {
            shed: Shed::new(dir.join("no-shed"), Some(dir.join("no-shed-dir"))),
            providers: cooee_core::Providers::default(),
            data: &dir.join("data"),
            config: &Config::load(&config).unwrap(),
            theme: looks.themes[&looks.default].clone(),
            plugins: Vec::new(),
        })
        .unwrap()
    }

    fn calls(dir: &Path) -> String {
        fs::read_to_string(dir.join("bin/calls.log")).unwrap_or_default()
    }

    #[test]
    fn a_pre_warmed_desk_wakes_once_and_sleeps_after_15_idle_minutes() {
        let dir = scratch("desk-idle");
        let meta = serde_json::json!({ "format": 1, "thread": { "id": "t", "title": "t", "messages": [], "terminal": "cooee-t", "profile": "ann" } });
        seed(&dir, "t", "");
        fs::write(dir.join("data/threads/t.json"), meta.to_string()).unwrap();
        let hub = desk_hub(&dir, "stopped");

        hub.pre_warm("t").unwrap();
        hub.pre_warm("t").unwrap();
        let t0 = now();
        assert_eq!(calls(&dir), "start ann-desk\n", "woken once");
        hub.watch(t0 + IDLE - 60_000);
        assert_eq!(calls(&dir), "start ann-desk\n", "not idle yet");
        hub.watch(t0 + IDLE);
        assert_eq!(calls(&dir), "start ann-desk\nstop ann-desk\n");
        hub.watch(t0 + 2 * IDLE);
        assert_eq!(
            calls(&dir),
            "start ann-desk\nstop ann-desk\n",
            "an asleep desk is left"
        );
    }

    #[test]
    fn a_desk_found_awake_sleeps_15_minutes_after_it_was_first_seen() {
        let dir = scratch("desk-seen");
        let hub = desk_hub(&dir, "running");
        let t0 = now();
        hub.watch(t0);
        hub.watch(t0 + IDLE - 60_000);
        assert_eq!(calls(&dir), "", "its idle clock starts when it is seen");
        hub.watch(t0 + IDLE);
        assert_eq!(calls(&dir), "stop ann-desk\n");
    }
}
