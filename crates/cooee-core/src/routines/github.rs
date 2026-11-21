use hkdf::hmac::{Hmac, KeyInit, Mac};
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::local;
use serde_json::Value;
use sha2::Sha256;

/// One GitHub webhook delivery, proven: [`Delivery::signed`] checked its signature, [`Delivery::fetched`] read it
/// from GitHub's own delivery log with the owner's token.
#[derive(Debug, Clone)]
pub struct Delivery {
    guid: String,
    pub(super) event: String,
    payload: Value,
}

/// Why a webhook delivery was refused.
#[derive(Debug, thiserror::Error)]
pub enum Rejected {
    /// No `X-GitHub-Event` or `X-GitHub-Delivery`.
    #[error("no X-GitHub-Event or X-GitHub-Delivery")]
    NoHeaders,
    /// `X-Hub-Signature-256` isn't the body's HMAC.
    #[error("the signature doesn't check out")]
    BadSignature,
    /// The body isn't JSON.
    #[error("the body isn't JSON")]
    NotJson,
}

impl Rejected {
    /// Its HTTP status.
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self {
            Self::BadSignature => 401,
            Self::NoHeaders | Self::NotJson => 400,
        }
    }
}

impl Delivery {
    /// The delivery `body` with headers `X-GitHub-Event`, `X-GitHub-Delivery` and `X-Hub-Signature-256`, its
    /// HMAC-SHA256 under `secret` checked in constant time before the body is read.
    ///
    /// # Errors
    /// A missing header, a wrong signature, or a body that isn't JSON.
    pub fn signed(
        (event, guid, signature): (&str, &str, &str),
        body: &[u8],
        secret: &[u8],
    ) -> Result<Self, Rejected> {
        if event.is_empty() || guid.is_empty() {
            return Err(Rejected::NoHeaders);
        }
        verify(signature, body, secret)?;
        let payload = serde_json::from_slice(body).map_err(|_| Rejected::NotJson)?;
        Ok(Self::fetched(guid.to_owned(), event.to_owned(), payload))
    }

    /// A delivery read from GitHub's delivery log over TLS with the owner's token.
    #[must_use]
    pub const fn fetched(guid: String, event: String, payload: Value) -> Self {
        Self {
            guid,
            event,
            payload,
        }
    }

    /// Its `X-GitHub-Delivery`.
    #[must_use]
    pub fn guid(&self) -> &str {
        &self.guid
    }

    /// Its event, with the payload's action when it has one: `pull_request.opened`, `push`.
    #[must_use]
    pub fn event(&self) -> String {
        match self.payload.get("action").and_then(Value::as_str) {
            Some(a) => format!("{}.{a}", self.event),
            None => self.event.clone(),
        }
    }

    /// The payload's `repository.full_name`.
    #[must_use]
    pub fn repo(&self) -> &str {
        self.payload
            .pointer("/repository/full_name")
            .and_then(Value::as_str)
            .unwrap_or_default()
    }

    pub(super) fn at(&self, path: &str) -> String {
        match self
            .payload
            .pointer(&format!("/{}", path.replace('.', "/")))
        {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
        }
    }
}

pub(super) fn verify(signature: &str, body: &[u8], secret: &[u8]) -> Result<(), Rejected> {
    let sig = signature.strip_prefix("sha256=").and_then(unhex);
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| Rejected::BadSignature)?;
    mac.update(body);
    mac.verify_slice(&sig.ok_or(Rejected::BadSignature)?)
        .map_err(|_| Rejected::BadSignature)
}

fn unhex(h: &str) -> Option<Vec<u8>> {
    let pairs = h.as_bytes().chunks(2);
    pairs
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}

/// `[hooks]`: a second listener serving only `POST /hooks/github` and `POST /hooks/in/<id>`, published by Tailscale
/// Funnel.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hooks {
    /// Where it listens (`127.0.0.1:4748`).
    pub bind: std::net::SocketAddr,
    /// Its public URL, as the GitHub webhook has it (`https://<mac>.ts.net:8443/hooks/github`).
    pub url: String,
    /// GitHub's API, for the catch-up (default `https://api.github.com`).
    #[serde(default = "github_api")]
    pub api: String,
}

fn github_api() -> String {
    "https://api.github.com".to_owned()
}

/// The GitHub events a routine runs on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Github {
    /// `owner/name`, matched case-insensitively with the payload's `repository.full_name`.
    pub repo: String,
    /// `pull_request` (any action) or `pull_request.opened`; 1 to 8.
    pub events: Vec<String>,
    /// Payload paths (`workflow_run.conclusion`) and the text each must equal.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub r#match: BTreeMap<String, String>,
}

impl Github {
    pub(super) fn checked(self) -> Result<Self, String> {
        let word = |w: &str| !w.is_empty() && !w.contains(['/', ' ']);
        let repo = self
            .repo
            .split_once('/')
            .is_some_and(|(o, n)| word(o) && word(n));
        let events = (1..=8).contains(&self.events.len()) && self.events.iter().all(|e| word(e));
        if repo && events {
            Ok(self)
        } else {
            Err("BAD_GITHUB: repo is owner/name, events 1 to 8 like pull_request.opened".to_owned())
        }
    }
}

/// Why a run didn't reach an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skip {
    /// It is paused.
    Paused,
    /// It ran [`super::RATE`] times in the last hour.
    Rate,
    /// Its wake script exited 0.
    Quiet,
    /// An event or a wake came at its runs-a-day cap (#690).
    Capped,
}

/// What a prompt's `{{…}}` read: a schedule's due time (ms) in a UTC offset (minutes), or an event.
pub enum Fields<'a> {
    /// A due time.
    Due(u64, i32),
    /// A delivery.
    Event(&'a Delivery),
    /// A signed POST to `/hooks/in/<id>`: no path reads it; its body follows the prompt as data.
    Inbound(&'a super::Inbound),
}

impl Fields<'_> {
    pub(super) fn get(&self, path: &str) -> String {
        match (self, path) {
            (Self::Due(due, offset), "due") => local(*due, *offset),
            (Self::Event(d), "event") => d.event(),
            (Self::Event(d), "repo") => d.repo().to_owned(),
            (Self::Event(d), path) => d.at(path),
            (Self::Due(..) | Self::Inbound(_), _) => String::new(),
        }
    }

    pub(super) fn data(&self) -> String {
        match self {
            Self::Inbound(i) => i.data(),
            Self::Due(..) | Self::Event(_) => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routines::{Ask, Fields, HOUR, Paused, RATE, Routine, Skip};

    const MON: u64 = 1_731_283_200_000;

    #[test]
    fn a_signature_checks_against_githubs_documented_test_vector() {
        let (secret, body) = (b"It's a Secret to Everybody", b"Hello, World!");
        let sig = "sha256=757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17";
        let headers = |sig| ("ping", "g-1", sig);
        let e = Delivery::signed(headers(sig), body, secret).unwrap_err();
        assert!(matches!(e, Rejected::NotJson), "the signature passed: {e}");
        for bad in [&sig.replace("757", "758"), "sha256=75", "", &sig[7..]] {
            let e = Delivery::signed(headers(bad), body, secret).unwrap_err();
            assert_eq!(e.status(), 401, "{bad}");
        }
        let e = Delivery::signed(("", "g-1", sig), body, secret).unwrap_err();
        assert_eq!(e.status(), 400);
    }

    #[test]
    fn an_event_matches_its_repo_events_and_match_fills_the_prompt_and_is_rate_capped() {
        let github: Github = serde_json::from_value(serde_json::json!({
            "repo": "Owner/App", "events": ["pull_request.opened", "push"],
            "match": { "pull_request.draft": "false" } }))
        .unwrap();
        let chat = crate::Thread {
            id: "t-1".to_owned(),
            ..crate::Thread::default()
        };
        let ask = Ask {
            when: None,
            github: Some(github),
            script: None,
            prompt: "{{event}} on {{repo}}: {{pull_request.title}} #{{ pull_request.number }}{{nope}} {{x".to_owned(),
            name: " Monday renewals ".to_owned(),
            webhook: None,
            mail: false,
            place: String::new(),
            until: None,
        };
        let e = Routine::make(&[], ask.clone(), &chat, (MON, 0)).unwrap_err();
        assert!(e.starts_with("BAD_PROMPT"), "{e}");
        let ask = Ask {
            prompt: ask.prompt.replace(" {{x", ""),
            ..ask
        };
        let mut r = Routine::make(&[], ask.clone(), &chat, (MON, 0)).unwrap();
        assert_eq!(
            (r.when().as_str(), r.due(MON * 2, 0)),
            ("on pull_request.opened, push in Owner/App", None)
        );
        let pr = |action: &str, draft: bool, title: &str| {
            let payload = serde_json::json!({ "action": action, "repository": { "full_name": "owner/app" },
                "pull_request": { "title": title, "number": 7, "draft": draft } });
            Delivery::fetched("g".to_owned(), "pull_request".to_owned(), payload)
        };
        let d = pr("opened", false, "Fix login");
        assert!(r.matches(&d));
        assert!(!r.matches(&pr("closed", false, "x")) && !r.matches(&pr("opened", true, "x")));
        assert_eq!(
            r.text(&Fields::Event(&d)),
            "pull_request.opened on owner/app: Fix login #7"
        );
        let long = r.text(&Fields::Event(&pr("opened", false, &"t".repeat(3000))));
        assert_eq!(long.chars().filter(|c| *c == 't').count(), 2000 + 1);
        assert!(long.contains("t…"), "{long}");
        for at in 0..u64::try_from(RATE).unwrap() {
            assert_eq!(r.admit(MON + at), Ok(()));
        }
        assert_eq!(r.admit(MON + 10), Err(Skip::Rate));
        assert_eq!(r.admit(MON + HOUR), Ok(()), "an hour on, it runs again");
        r.paused = Some(Paused::Owner);
        assert_eq!(r.admit(MON + 2 * HOUR), Err(Skip::Paused));
        for (bad, why) in [(Some("x"), "BAD_GITHUB"), (None, "BAD_TRIGGER")] {
            let mut a = ask.clone();
            if let Some(repo) = bad {
                a.github.as_mut().unwrap().repo = repo.to_owned();
            } else {
                a.when = Some("0 9 * * 1".to_owned());
            }
            let e = Routine::make(&[], a, &chat, (MON, 0)).unwrap_err();
            assert!(e.starts_with(why), "{e}");
        }
    }
}
