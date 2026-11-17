use std::{
    collections::HashMap,
    sync::{Condvar, Mutex},
};

use cooee_core::{
    browser::{self, Control, Gate},
    cdp::{self, Tab},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Hub, HubError, lock, now};

#[derive(Default)]
struct Desk {
    control: Control,
    waiting: usize,
}

#[derive(Default)]
pub(super) struct Desks {
    map: Mutex<HashMap<String, Desk>>,
    handed: Condvar,
}

/// A persona's browser as the Screen shows it (#162): who drives it and how many agent calls wait.
#[derive(Debug, Serialize)]
pub struct Screen {
    control: Control,
    waiting: usize,
}

/// One frame of the live view: the current tab and its JPEG, base64.
#[derive(Debug, Serialize)]
pub struct Frame {
    #[serde(flatten)]
    tab: Tab,
    jpeg: String,
}

/// The owner's input while they hold control.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Input {
    /// A left click at a point of the page.
    Click {
        /// From the left, in CSS pixels.
        x: f64,
        /// From the top, in CSS pixels.
        y: f64,
    },
    /// Text typed into the focused field.
    Text(String),
}

impl Hub {
    fn known(&self, persona: &str) -> Result<(), HubError> {
        if self.persona(persona).id == persona {
            return Ok(());
        }
        Err(HubError::Refused(format!("no persona {persona}")))
    }

    /// Persona `persona`'s browser: who drives it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown persona.
    pub fn screen(&self, persona: &str) -> Result<Screen, HubError> {
        self.known(persona)?;
        let map = lock(&self.desks.map);
        let desk = map.get(persona);
        Ok(Screen {
            control: desk.map(|d| d.control.clone()).unwrap_or_default(),
            waiting: desk.map_or(0, |d| d.waiting),
        })
    }

    /// The owner takes control of `persona`'s browser for 10 minutes; its agent's browser calls wait meanwhile.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown persona.
    pub fn take_control(&self, persona: &str) -> Result<Screen, HubError> {
        self.known(persona)?;
        lock(&self.desks.map)
            .entry(persona.to_owned())
            .or_default()
            .control = Control::take(now());
        self.screen(persona)
    }

    /// The owner hands `persona`'s browser back with `note`: its waiting call goes on and carries the note.
    ///
    /// # Errors
    /// [`HubError::Refused`] unless the owner holds control.
    pub fn hand_back(&self, persona: &str, note: String) -> Result<Screen, HubError> {
        {
            let mut map = lock(&self.desks.map);
            let desk = map.entry(persona.to_owned()).or_default();
            desk.control = desk
                .control
                .hand_back(note)
                .ok_or_else(|| HubError::Refused(format!("you don't hold {persona}'s browser")))?;
        }
        self.desks.handed.notify_all();
        self.screen(persona)
    }

    pub(super) fn wait_turn(&self, persona: &str) -> Option<String> {
        let mut map = lock(&self.desks.map);
        let mut waited = false;
        loop {
            let desk = map.entry(persona.to_owned()).or_default();
            if waited {
                desk.waiting = desk.waiting.saturating_sub(1);
            }
            let d = match desk.control.gate(now()) {
                Gate::Go(note) => return note,
                Gate::Wait(d) => d,
            };
            desk.waiting += 1;
            waited = true;
            map = self
                .desks
                .handed
                .wait_timeout(map, d)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }

    fn tab(&self, persona: &str) -> Result<Tab, HubError> {
        self.known(persona)?;
        let profile = browser::profile(&self.data, persona);
        let port = browser::port(&profile)
            .ok_or_else(|| HubError::Refused(format!("{persona}'s browser isn't running")))?;
        cdp::current(port).map_err(|e| HubError::Refused(format!("{persona}'s browser: {e}")))
    }

    /// The live view's next frame of `persona`'s browser: its current tab, captured now.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the browser isn't running or doesn't answer.
    pub fn frame(&self, persona: &str) -> Result<Frame, HubError> {
        let tab = self.tab(persona)?;
        let shot = json!({ "format": "jpeg", "quality": 60 });
        let r = tab
            .run(&[("Page.captureScreenshot", shot)])
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let jpeg = r
            .get("data")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Ok(Frame { tab, jpeg })
    }

    /// The owner's `input` on `persona`'s current tab, only while they hold control.
    ///
    /// # Errors
    /// [`HubError::Refused`] unless the owner holds control, or when the browser doesn't take it.
    pub fn input(&self, persona: &str, input: &Input) -> Result<(), HubError> {
        let held = matches!(
            lock(&self.desks.map).get(persona).map(|d| &d.control),
            Some(Control::Owner { until }) if now() < *until
        );
        if !held {
            return Err(HubError::Refused(format!(
                "take control of {persona}'s browser first"
            )));
        }
        let calls = match input {
            Input::Click { x, y } => ["mousePressed", "mouseReleased"]
                .map(|kind| {
                    let e =
                        json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 });
                    ("Input.dispatchMouseEvent", e)
                })
                .to_vec(),
            Input::Text(text) => vec![("Input.insertText", json!({ "text": text }))],
        };
        let tab = self.tab(persona)?;
        tab.run(&calls)
            .map(drop)
            .map_err(|e| HubError::Refused(e.to_string()))
    }
}
