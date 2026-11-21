use base64::{Engine, engine::general_purpose::STANDARD};
use cooee_core::{Config, browser};
use cooee_desk::{Desks, Name};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::{Hub, HubError, now, refused};

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

fn no(e: impl std::fmt::Display) -> HubError {
    HubError::Refused(e.to_string())
}

impl Hub {
    /// An agent's `desk` in thread `id`, on the desk its persona names (one the provider made): the action is
    /// logged in the persona's replay log, and the screen after it kept as the thread's replay frame, encrypted for
    /// 14 days; only a screenshot it asked for reaches the agent.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, no desk, one the provider didn't make, or the desk's own error.
    pub fn desk(&self, id: &str, token: &str, ask: &DeskAsk) -> Result<DeskSeen, HubError> {
        let persona = self.persona(&self.token_of(id, token)?.profile);
        let config = self.config.as_deref().map(Config::load).transpose();
        let container = config.map_err(no)?.and_then(|c| c.container);
        let (Some(container), Some(desk)) = (container, persona.desk.as_deref()) else {
            return Err(no(format!("NO_DESK: {} has no desk", persona.id)));
        };
        let desks = Desks::new(container, Vec::new());
        if !desks.made().map_err(no)?.iter().any(|d| d.name == desk) {
            return Err(no(format!("NO_DESK: no desk {desk} was made here")));
        }
        let name = Name::new(desk).map_err(no)?;
        let (caption, asked) = match ask {
            DeskAsk::Screenshot {} => ("screenshot".to_owned(), true),
            DeskAsk::Click { x, y } => (format!("click {x},{y}"), false),
            DeskAsk::Type { text } => (format!("type {text:?}"), false),
        };
        let line = json!({ "at": now(), "thread": id, "desk": desk, "tool": caption });
        let profile = browser::profile(&self.data, &persona.id);
        browser::log(&profile, &line).map_err(|e| refused(&profile, &e))?;
        match ask {
            DeskAsk::Screenshot {} => Ok(()),
            DeskAsk::Click { x, y } => desks.click(&name, *x, *y),
            DeskAsk::Type { text } => desks.type_text(&name, text),
        }
        .map_err(no)?;
        let shot = desks.screenshot(&name).map_err(no)?;
        self.seal(id, (&caption, asked), &shot)?;
        Ok(DeskSeen {
            text: format!("{caption} on {desk}: done"),
            image: asked.then(|| STANDARD.encode(&shot)),
        })
    }
}
