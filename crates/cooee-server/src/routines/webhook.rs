use std::sync::Arc;

use cooee_core::{
    Body,
    routines::{Inbound, Routine, Trigger, local},
    secrets,
};
use topcoat::{
    Result,
    context::Cx,
    router::{Body as Bytes, request::Bytes as Raw, response::Response, route},
};

use super::{Runner, runner};
use crate::{
    http::refused,
    hub::{HubError, now},
};

impl Runner {
    pub(super) fn note(&self, r: &Routine, line: &str) {
        let text = format!("⟳ {}: {line}", r.name);
        if let Err(e) = self.hub.say(&r.made_in, Body::Note { text }) {
            cooee_core::note!("cooee: routines: {}: {e}", r.id);
        }
    }

    /// Routine `r`'s last signed POST, as its row shows it, when it has a silence rule.
    #[must_use]
    pub fn heard(&self, r: &Routine, now: u64) -> Option<String> {
        let Trigger::Webhook(h) = &r.trigger else {
            return None;
        };
        h.silence?;
        Some(match r.heard {
            None => "no signed POST yet".to_owned(),
            Some(at) => format!("last POST {}", local(at, self.offset(now))),
        })
    }

    pub(crate) fn hear(&self, id: &str, now: u64) {
        let back = self.change(|b| {
            let r = b.routines.iter_mut().find(|r| r.id == id);
            Ok(r.and_then(|r| r.hear(now).map(|m| (r.clone(), m))))
        });
        match back {
            Ok(Some((r, m))) => self.note(
                &r,
                &format!("signed POSTs are back after {m} min of silence"),
            ),
            Ok(None) => {}
            Err(e) => cooee_core::note!("cooee: routines: {id}: {e}"),
        }
    }

    /// `POST /hooks/in/<id>`'s answer for a POST with headers `Idempotency-Key` and `X-Signature-256`: 404 for no
    /// such webhook routine, 503 without its secret, 401 for a wrong signature, 200 `alive` for a `{"kind":"alive"}`
    /// body (it only feeds the silence clock), else [`Runner::accept`]'s.
    #[must_use]
    pub fn inbound(
        self: &Arc<Self>,
        id: &str,
        headers: (&str, &str),
        body: &[u8],
        now: u64,
    ) -> (u16, String) {
        let list = self.list().unwrap_or_default();
        let Some(Trigger::Webhook(h)) = list.into_iter().find(|r| r.id == id).map(|r| r.trigger)
        else {
            return (404, format!("no webhook routine {id}"));
        };
        let secret = match secrets::get(&self.data, &h.secret) {
            Ok(Some(s)) => s,
            Ok(None) => return (503, format!("no secret:{}", h.secret)),
            Err(e) => return (503, e.to_string()),
        };
        match Inbound::signed(headers, body, secret.as_bytes()) {
            Ok(i) => {
                self.hear(id, now);
                if i.alive() {
                    return (200, "alive".to_owned());
                }
                self.accept(id, &i, now)
            }
            Err(e) => (e.status(), e.to_string()),
        }
    }

    /// Settings' test button: routine `id` runs on a test POST as a signed one does.
    ///
    /// # Errors
    /// [`HubError::Refused`] for no such webhook routine, without its secret, or when it isn't run.
    pub fn test(self: &Arc<Self>, id: &str, now: u64) -> Result<String, HubError> {
        let list = self.list()?;
        let Some(Trigger::Webhook(h)) = list.into_iter().find(|r| r.id == id).map(|r| r.trigger)
        else {
            return Err(refused(format!("no webhook routine {id}")));
        };
        if !secrets::get(&self.data, &h.secret).is_ok_and(|s| s.is_some()) {
            return Err(refused(format!("no secret:{}", h.secret)));
        }
        match self.accept(id, &Inbound::test(now), now) {
            (202, text) => Ok(text),
            (_, why) => Err(refused(why)),
        }
    }
}

#[route(POST "/hooks/in/{id}")]
pub(super) async fn inbound_hook(cx: &Cx, body: Raw) -> Result<Response> {
    let h = |n: &str| crate::http::header(cx, n).unwrap_or_default().to_owned();
    let (id, key, sig) = (
        crate::http::thread_id(cx),
        h("idempotency-key"),
        h("x-signature-256"),
    );
    let r = runner(cx);
    let (status, text) =
        tokio::task::spawn_blocking(move || r.inbound(&id, (&key, &sig), &body, now()))
            .await
            .unwrap_or_else(|e| (500, e.to_string()));
    Ok(Response::builder().status(status).body(Bytes::from(text))?)
}
