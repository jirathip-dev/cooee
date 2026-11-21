use std::{collections::HashMap, sync::Mutex};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use cooee_core::{Config, browser::Control};
use cooee_desk::{Desks, Name};
use serde::Serialize;

use super::{Hub, HubError, Input, lock, now};

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

#[derive(Default)]
pub(super) struct State {
    leases: Leases,
}

impl Hub {
    fn desk(&self, desk: &str) -> Result<(Desks, Name), HubError> {
        let config = self.config.as_deref().map(Config::load).transpose();
        let container = config.map_err(|e| refused(&e))?.and_then(|c| c.container);
        let desks = Desks::new(
            container.ok_or_else(|| refused(&"no desks here"))?,
            Vec::new(),
        );
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
        self.desk(desk)?;
        Ok(self.held(desk))
    }

    /// The owner takes control of desk `desk` for 10 minutes, or 10 more while they hold it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a desk cooee didn't make.
    pub fn desk_take(&self, desk: &str) -> Result<Control, HubError> {
        self.desk(desk)?;
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
        let (desks, name) = self.desk(desk)?;
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
        let (desks, name) = self.desk(desk)?;
        let jpeg = desks.frame(&name).map_err(|e| refused(&e))?;
        Ok(DeskFrame {
            jpeg: STANDARD.encode(jpeg),
        })
    }
}
