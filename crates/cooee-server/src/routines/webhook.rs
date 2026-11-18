use std::sync::Arc;

use cooee_core::{
    routines::{Inbound, Trigger},
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
    /// `POST /hooks/in/<id>`'s answer for a POST with headers `Idempotency-Key` and `X-Signature-256`: 404 for no
    /// such webhook routine, 503 without its secret, 401 for a wrong signature, else [`Runner::accept`]'s.
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
            Ok(i) => self.accept(id, &i, now),
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
