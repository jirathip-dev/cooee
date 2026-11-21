use std::sync::Arc;

use cooee_core::{
    routines::{Delivery, Fields, Inbound, Outcome, Routine},
    secrets,
};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Body as Bytes, Router, request::Bytes as Raw, response::Response, route},
};

use super::{Runner, webhook};
use crate::{
    http::refused,
    hub::{HubError, now},
};

type Picked = (Routine, String, String);

impl Runner {
    /// `POST /hooks/github`'s answer for a delivery with headers `X-GitHub-Event`, `X-GitHub-Delivery` and
    /// `X-Hub-Signature-256`: 503 without `secret:GITHUB_WEBHOOK_SECRET` (GitHub marks it failed; the catch-up takes
    /// it), 400 or 401 when [`Delivery::signed`] refuses it, else [`Runner::deliver`]'s.
    #[must_use]
    pub fn hook(
        self: &Arc<Self>,
        headers: (&str, &str, &str),
        body: &[u8],
        now: u64,
    ) -> (u16, String) {
        let secret = match secrets::get(&self.data, "GITHUB_WEBHOOK_SECRET") {
            Ok(Some(s)) => s,
            Ok(None) => return (503, "no secret:GITHUB_WEBHOOK_SECRET".to_owned()),
            Err(e) => return (503, e.to_string()),
        };
        match Delivery::signed(headers, body, secret.as_bytes()) {
            Ok(d) => self.deliver(&d, now),
            Err(e) => (e.status(), e.to_string()),
        }
    }

    /// Delivery `d`, once per guid: 200 when seen before or a ping; else 202, and each routine it matches runs on a
    /// worker (paused, over the rate or at its cap: skipped), so the answer comes before any run starts.
    pub fn deliver(self: &Arc<Self>, d: &Delivery, now: u64) -> (u16, String) {
        if d.event() == "ping" {
            return (200, "pong".to_owned());
        }
        let picked = self.change(|b| {
            if !b.see(d.guid(), now) {
                return Ok(None);
            }
            let mut runs = Vec::new();
            for r in b.routines.iter_mut().filter(|r| r.matches(d)) {
                let key = r.key(d.guid());
                match r.admit(now).and_then(|()| self.admit_event(r, now)) {
                    Ok(()) => runs.push((r.clone(), r.text(&Fields::Event(d)), key)),
                    Err(why) => self.record(r, key, Outcome::Skipped { why }, now),
                }
            }
            Ok(Some(runs))
        });
        self.later(picked)
    }

    /// Signed POST `i` to routine `id`, once per key: 200 when seen before; else 202 and the routine runs on a
    /// worker (paused, over the rate or at its cap: skipped), so the answer comes before the run starts.
    pub fn accept(self: &Arc<Self>, id: &str, i: &Inbound, now: u64) -> (u16, String) {
        let picked = self.change(|b| {
            if !b.see(&format!("{id}:{}", i.key()), now) {
                return Ok(None);
            }
            let mut runs = Vec::new();
            for r in b.routines.iter_mut().filter(|r| r.id == id) {
                let key = r.key(i.key());
                match r.admit(now).and_then(|()| self.admit_event(r, now)) {
                    Ok(()) => runs.push((r.clone(), r.text(&Fields::Inbound(i)), key)),
                    Err(why) => self.record(r, key, Outcome::Skipped { why }, now),
                }
            }
            Ok(Some(runs))
        });
        self.later(picked)
    }

    fn later(self: &Arc<Self>, picked: Result<Option<Vec<Picked>>, HubError>) -> (u16, String) {
        let runs = match picked {
            Ok(Some(runs)) => runs,
            Ok(None) => return (200, "seen".to_owned()),
            Err(e) => return (500, e.to_string()),
        };
        let n = runs.len();
        let runner = Arc::clone(self);
        std::thread::spawn(move || {
            for (r, text, key) in runs {
                let outcome = runner.run(&r, &text, &key);
                let saved = runner.change(|b| {
                    if let Some(o) = b.routines.iter_mut().find(|o| o.id == r.id) {
                        runner.record(o, key, outcome, crate::hub::now());
                    }
                    Ok(())
                });
                if let Err(e) = saved {
                    cooee_core::note!("cooee: routines: {e}");
                }
            }
        });
        (202, format!("runs: {n}"))
    }
}

pub(super) fn runner(cx: &Cx) -> Arc<Runner> {
    Arc::clone(app_context::<Arc<Runner>>(cx))
}

pub(crate) async fn on<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Arc<Runner>) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let r = runner(cx);
    tokio::task::spawn_blocking(move || f(&r))
        .await
        .map_err(refused)?
}

#[route(POST "/hooks/github")]
async fn github_hook(cx: &Cx, body: Raw) -> Result<Response> {
    let h = |n: &str| crate::http::header(cx, n).unwrap_or_default().to_owned();
    let headers = (
        h("x-github-event"),
        h("x-github-delivery"),
        h("x-hub-signature-256"),
    );
    let r = runner(cx);
    let (status, text) = tokio::task::spawn_blocking(move || {
        r.hook((&headers.0, &headers.1, &headers.2), &body, now())
    })
    .await
    .unwrap_or_else(|e| (500, e.to_string()));
    Ok(Response::builder().status(status).body(Bytes::from(text))?)
}

/// `[hooks] bind`'s router: `POST /hooks/github`, `POST /hooks/in/<id>`, `POST /hooks/mail/<persona>` and nothing else; no Gate, no devices, no other route.
#[must_use]
pub fn hooks(runner: Arc<Runner>) -> Router {
    Router::builder()
        .app_context(runner)
        .route(github_hook)
        .route(webhook::inbound_hook)
        .route(webhook::mail_hook)
        .build()
}
