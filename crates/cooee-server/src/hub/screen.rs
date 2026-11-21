use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::{Condvar, Mutex},
};

use cooee_core::{
    browser::{self, Control, Did, Gate},
    cdp::{self, Tab},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{Hub, HubError, lock, now};

#[derive(Default)]
struct Desk {
    control: Control,
    waiting: usize,
    steps: Vec<String>,
}

const LABEL: &str = "(e=>e?((e.labels&&e.labels[0]&&e.labels[0].innerText)||e.getAttribute('aria-label')||e.getAttribute('placeholder')||e.getAttribute('name')||(e.isContentEditable||e.matches('input,textarea,select')?'':e.innerText)||e.id||'').trim().slice(0,80):'')";

const TEACH: &str = "The owner asks you to save what they just did as a skill: draft it from these steps and propose \
                     it with your skill tool. What they typed is left out: name the field, never a value.";

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

#[derive(Default)]
pub(super) struct State {
    desks: Desks,
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
        let map = lock(&self.areas.screen.desks.map);
        let desk = map.get(persona);
        Ok(Screen {
            control: desk.map(|d| d.control.clone()).unwrap_or_default(),
            waiting: desk.map_or(0, |d| d.waiting),
        })
    }

    /// The owner takes control of `persona`'s browser for 10 minutes, or 10 more while they hold it; its agent's browser
    /// calls wait meanwhile.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown persona.
    pub fn take_control(&self, persona: &str) -> Result<Screen, HubError> {
        self.known(persona)?;
        let mut map = lock(&self.areas.screen.desks.map);
        let desk = map.entry(persona.to_owned()).or_default();
        let at = now();
        if !matches!(desk.control, Control::Owner { until } if until > at) {
            *desk = Desk::default();
        }
        desk.control = desk.control.take(at);
        drop(map);
        self.screen(persona)
    }

    /// The owner hands `persona`'s browser back with `note`: its waiting call goes on and carries the note, and with
    /// `skill` (#500) the lease's steps and the ask to propose them as a skill.
    ///
    /// # Errors
    /// [`HubError::Refused`] unless the owner holds control.
    pub fn hand_back(&self, persona: &str, note: String, skill: bool) -> Result<Screen, HubError> {
        {
            let mut map = lock(&self.areas.screen.desks.map);
            let desk = map.entry(persona.to_owned()).or_default();
            let mut note = note;
            if skill {
                write!(note, "\n\n{TEACH}").ok();
                for (n, step) in (1_u32..).zip(&desk.steps) {
                    write!(note, "\n{n}. {step}").ok();
                }
            }
            desk.control = desk
                .control
                .hand_back(note)
                .ok_or_else(|| HubError::Refused(format!("you don't hold {persona}'s browser")))?;
            desk.steps.clear();
        }
        self.areas.screen.desks.handed.notify_all();
        self.screen(persona)
    }

    pub(super) fn wait_turn(&self, persona: &str) -> Option<String> {
        let mut map = lock(&self.areas.screen.desks.map);
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
                .areas
                .screen
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

    /// The owner's `input` on `persona`'s current tab, only while they hold control; it joins the lease's steps by the
    /// label of what it clicked or typed into.
    ///
    /// # Errors
    /// [`HubError::Refused`] unless the owner holds control, or when the browser doesn't take it.
    pub fn input(&self, persona: &str, input: &Input) -> Result<(), HubError> {
        let held = matches!(
            lock(&self.areas.screen.desks.map).get(persona).map(|d| &d.control),
            Some(Control::Owner { until }) if now() < *until
        );
        if !held {
            return Err(HubError::Refused(format!(
                "take control of {persona}'s browser first"
            )));
        }
        let (calls, at, did): (_, _, fn(String) -> Did) = match input {
            Input::Click { x, y } => (
                ["mousePressed", "mouseReleased"]
                    .map(|kind| {
                        let e = json!({ "type": kind, "x": x, "y": y, "button": "left", "clickCount": 1 });
                        ("Input.dispatchMouseEvent", e)
                    })
                    .to_vec(),
                format!("document.elementFromPoint({x}, {y})"),
                Did::Click,
            ),
            Input::Text(text) => (
                vec![("Input.insertText", json!({ "text": text }))],
                "document.activeElement".to_owned(),
                Did::Type,
            ),
        };
        let tab = self.tab(persona)?;
        let ask = json!({ "expression": format!("{LABEL}({at})"), "returnByValue": true });
        let label = tab.run(&[("Runtime.evaluate", ask)]).ok();
        let label = label.as_ref().and_then(|l| l.pointer("/result/value"));
        let label = label.and_then(Value::as_str).unwrap_or_default().to_owned();
        tab.run(&calls)
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let sites = self.persona(persona).sites;
        let step = browser::step(&sites, &tab.url, &tab.title, &did(label));
        if let Some(desk) = lock(&self.areas.screen.desks.map).get_mut(persona) {
            desk.steps.push(step);
        }
        Ok(())
    }
}
