use std::{collections::HashMap, sync::Mutex};

use cooee_core::{action::Action, mail, rules::Effect};
use serde::Deserialize;

use super::{Hub, HubError, lock};

/// An agent's `mail` (#220): one of list, search, read or send.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MailAsk {
    /// The unread mails.
    pub list: bool,
    /// Mails by sender or date.
    pub search: Option<Search>,
    /// One mail by id.
    pub read: Option<String>,
    /// A mail to send.
    pub send: Option<Outgoing>,
}

/// A search: the sender contains `from`, the date contains `date`.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Search {
    /// Part of the sender.
    pub from: String,
    /// Part of the date.
    pub date: String,
}

/// A mail to send.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Outgoing {
    /// The one recipient.
    pub to: String,
    /// Its subject.
    pub subject: String,
    /// Its text.
    pub text: String,
}

fn refused(e: String) -> HubError {
    HubError::Refused(e)
}

#[derive(Default)]
pub(super) struct State {
    codes: Mutex<HashMap<String, String>>,
}

impl Hub {
    /// An agent's `mail` in thread `id`, on its persona's inbox: a send goes at once only when a rule allows
    /// `mail/<to>`, else on the owner's approved card; a read's verification code is kept for `secret MAIL_CODE`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, no inbox on, a denied recipient, or the backend's error.
    pub fn mail(&self, id: &str, token: &str, a: MailAsk) -> Result<String, HubError> {
        let persona = self.persona(&self.token_of(id, token)?.profile);
        let inbox = persona.mail.clone().filter(|m| m.on);
        let inbox =
            inbox.ok_or_else(|| refused("NO_INBOX: mail is off for this persona".to_owned()))?;
        let b = mail::open(&self.data, &persona.id, &inbox).map_err(refused)?;
        if let Some(s) = a.search {
            return mail::search(b.as_ref(), &s.from, &s.date).map_err(refused);
        }
        if let Some(m) = a.read {
            let (text, code) = mail::read(b.as_ref(), &m).map_err(refused)?;
            if let Some(code) = code {
                lock(&self.areas.mail.codes).insert(persona.id, code);
            }
            return Ok(text);
        }
        let Some(s) = a.send else {
            return mail::list(b.as_ref()).map_err(refused);
        };
        mail::addressed(&s.to, &s.subject).map_err(refused)?;
        let action = Action::tool("mail", &s.to);
        let folder = persona.work_folder().unwrap_or_default();
        let verdict = self
            .rules_of(&persona)
            .decide(&action, &folder, Effect::Ask);
        let sent = format!("sent to {}", s.to);
        match verdict.effect {
            Effect::Deny => Err(refused(format!("mail/{}: denied by a rule", s.to))),
            Effect::Allow => b
                .send(&s.to, &s.subject, &s.text)
                .map(|()| sent)
                .map_err(refused),
            Effect::Ask => {
                let what = format!("{} · send mail to {}: {}", persona.name, s.to, s.subject);
                let detail = s.text.clone();
                let go = Box::new(move || b.send(&s.to, &s.subject, &s.text).map(|()| sent));
                self.ask_card(id, (what, detail), go)?;
                Ok(
                    "Asked the owner: it is sent once they approve the card; nothing was sent."
                        .to_owned(),
                )
            }
        }
    }

    pub(super) fn mail_code(&self, persona: &str) -> Option<String> {
        lock(&self.areas.mail.codes).remove(persona)
    }
}
