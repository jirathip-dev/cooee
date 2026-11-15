//! A GitHub webhook delivery (#261), proven by its HMAC-SHA256 signature or by GitHub's own delivery log.

use hkdf::hmac::{Hmac, KeyInit, Mac};
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
        let sig = signature.strip_prefix("sha256=").and_then(unhex);
        let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| Rejected::BadSignature)?;
        mac.update(body);
        mac.verify_slice(&sig.ok_or(Rejected::BadSignature)?)
            .map_err(|_| Rejected::BadSignature)?;
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
        match self.payload["action"].as_str() {
            Some(a) => format!("{}.{a}", self.event),
            None => self.event.clone(),
        }
    }

    /// The payload's `repository.full_name`.
    #[must_use]
    pub fn repo(&self) -> &str {
        self.payload["repository"]["full_name"]
            .as_str()
            .unwrap_or_default()
    }

    /// The payload's value at dotted `path` as text: a string as it is, others as JSON, a missing one empty.
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

/// Hex `h` as bytes.
fn unhex(h: &str) -> Option<Vec<u8>> {
    let pairs = h.as_bytes().chunks(2);
    pairs
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).ok()?, 16).ok())
        .collect()
}
