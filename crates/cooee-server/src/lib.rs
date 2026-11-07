//! cooee's HTTP server: the web UI's pages (`cooee-web`), the threads' JSON API and their live events over SSE
//! (replayed from the last seen event on reconnect), provider logins, pairing and web push. Every request must name one of the
//! server's own hosts (`Host`, and `Origin` when sent: no DNS rebinding), and every page and API call but pairing
//! and the static files needs a paired device's cookie; a plugin's backend posts with its token instead. Plugin tabs
//! (docs/plugins.md) run their module in a sandboxed frame and reach their backend through cooee's proxy.

#![forbid(unsafe_code)]

mod hub;
mod login;
mod manage;
mod plugin;
mod push;
mod themes;

use std::{
    io,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{Claim, Config, Device, Devices, Plugin, ShedError};
use futures_util::{Stream, StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{
        Body, Layer, LayerFuture, Next, Path, Router,
        content::{
            Form, Json,
            sse::{Event, KeepAlive, Sse, last_event_id},
        },
        error::not_found,
        page, parse_query_params, path_param,
        request::{headers, method, uri},
        response::{IntoResponse, Response},
        route,
    },
    view::View,
};

pub use hub::{CardSpec, Hub, HubError, Launch, Listing, Out, Resume, Slice};
pub use login::{LOGIN_TIMEOUT, Logins, View as LoginView};
pub use manage::{Backoff, ISSUES, Manager, Plan, Row};
pub use push::{Note, Push, Sub, encrypt, jwt, note};
pub use themes::{Pack, Pinned, Themes, stamp};

/// The paths an unpaired browser may load: pairing and the static files (and the themes' files under `/look/`).
const OPEN: [&str; 13] = [
    "/pair",
    "/look",
    "/look.js",
    "/pair/request",
    "/pair/claim",
    "/pair/cancel",
    "/manifest.webmanifest",
    "/icon.svg",
    "/icon-192.png",
    "/icon-512.png",
    "/app.css",
    "/chat.js",
    "/sw.js",
];

/// The pairing cookie.
const COOKIE: &str = "cooee";

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// cooee's own error: one plain sentence, marked so the page shows it (`chat.ts` words any other failure itself).
fn plain(status: u16, text: &str) -> Result<Response> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Cooee-Error", "1")
        .body(Body::from(text.to_owned()))?)
}

fn header<'a>(cx: &'a Cx, name: &str) -> Option<&'a str> {
    headers(cx).get(name).and_then(|v| v.to_str().ok())
}

/// The request's paired device, put in the request context by the [`Gate`].
struct Paired(Device);

/// The plugin whose token a `/plugin/` request carries, put in the request context by the [`Gate`].
struct Caller(Plugin);

/// Checks every request: its `Host` (and `Origin`, when sent) must be one of `names`, and a page or API call must
/// carry a paired device's cookie (an unpaired page load goes to `/pair`).
struct Gate {
    /// localhost, the bind address and `allowed_hosts`; matched exactly, so a `*` never widens them.
    names: Vec<String>,
    devices: Devices,
    /// Its plugins are the ones a backend's token may name.
    hub: Arc<Hub>,
}

impl Gate {
    /// Whether `host` (`name` or `name:port`) is one of ours.
    fn ours(&self, host: &str) -> bool {
        let name = if host.starts_with('[') {
            host.split_inclusive(']').next().unwrap_or(host)
        } else {
            host.split(':').next().unwrap_or(host)
        };
        self.names.iter().any(|n| n.eq_ignore_ascii_case(name))
    }
}

impl Layer for Gate {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let host = header(cx, "host").or_else(|| uri(cx).host());
            let origin = header(cx, "origin").map(|o| {
                o.strip_prefix("https://")
                    .or_else(|| o.strip_prefix("http://"))
                    .is_some_and(|o| self.ours(o))
            });
            if !host.is_some_and(|h| self.ours(h)) || origin == Some(false) {
                return plain(403, "forbidden: foreign Host or Origin");
            }
            let path = uri(cx).path();
            if OPEN.contains(&path) || path.starts_with("/look/") {
                return next.run(cx, body).await;
            }
            if path.starts_with("/plugin/") {
                let token = header(cx, "authorization").and_then(|a| a.strip_prefix("Bearer "));
                let token = token.map(|t| cooee_core::sha256(t.as_bytes()));
                let found = self.hub.plugins().into_iter().find(|p| {
                    token.as_deref() == Some(cooee_core::sha256(p.token.as_bytes()).as_str())
                });
                return match found {
                    Some(p) => next.run(&cx.with(Caller(p)), body).await,
                    None => plain(401, "a plugin's token is needed"),
                };
            }
            let token = header(cx, "cookie").and_then(|c| {
                c.split(';')
                    .find_map(|c| c.trim().strip_prefix(COOKIE)?.strip_prefix('='))
            });
            // A store that can't be read is a 503: never taken for "unpaired" (herdsman#501).
            let found = match token.map(|t| self.devices.find(t, now())).transpose() {
                Ok(found) => found.flatten(),
                Err(e) => {
                    eprintln!("cooee: devices: {e}");
                    return plain(503, "cooee can't read its paired devices: try again soon.");
                }
            };
            match found {
                Some(d) => next.run(&cx.with(Paired(d)), body).await,
                None if method(cx) == "GET"
                    && (path == "/"
                        || path.starts_with("/t/")
                        || path.strip_prefix("/p/").is_some_and(|p| !p.contains('/'))) =>
                {
                    Ok(Response::builder()
                        .status(303)
                        .header("Location", "/pair")
                        .body(Body::from(()))?)
                }
                None => plain(
                    401,
                    "pair this device first: run `cooee pair` on the machine",
                ),
            }
        })
    }
}

path_param!(id: String);

fn thread_id(cx: &Cx) -> String {
    match path_param::<Id>(cx) {
        Ok(id) => id.clone(),
        Err(never) => match *never {},
    }
}

fn hub(cx: &Cx) -> Arc<Hub> {
    Arc::clone(app_context::<Arc<Hub>>(cx))
}

/// Runs a blocking hub call off the async workers.
async fn blocking<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Hub) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let hub = hub(cx);
    tokio::task::spawn_blocking(move || f(&hub))
        .await
        .map_err(|e| HubError::Refused(e.to_string()))?
}

/// A JSON answer: the value, or the error's status and message.
struct Answer<T>(Result<T, HubError>);

impl<T: Serialize> IntoResponse for Answer<T> {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let (status, body) = match self.0 {
            Ok(value) => return Json(value).into_response(cx),
            Err(e @ HubError::NoThread(_)) => (404, e.to_string()),
            Err(e @ HubError::Refused(_)) => (400, e.to_string()),
            // An answer or message to a session that is gone says so (herdsman#325).
            Err(HubError::Shed(e)) if e.no_such_terminal() => (
                410,
                "Not delivered: this thread's agent is gone.".to_owned(),
            ),
            // shed's own errors go to the log; the page gets a sentence.
            Err(HubError::Shed(
                e @ (ShedError::Rpc(_) | ShedError::Bad(_) | ShedError::Cli { .. }),
            )) => {
                eprintln!("cooee: {e}");
                (
                    502,
                    "shed couldn't do that: run cooee doctor on the Mac.".to_owned(),
                )
            }
            Err(e @ HubError::Shed(_)) => (502, e.to_string()),
            Err(e @ HubError::Store(_)) => (500, e.to_string()),
        };
        plain(status, &body)
    }
}

fn logins(cx: &Cx) -> Arc<Logins> {
    Arc::clone(app_context::<Arc<Logins>>(cx))
}

/// Runs a blocking login call off the async workers.
async fn on_logins<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Arc<Logins>) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let logins = logins(cx);
    tokio::task::spawn_blocking(move || f(&logins))
        .await
        .map_err(|e| HubError::Refused(e.to_string()))?
}

/// The plugins whose tab shows a dot (`POST /plugin/dot`), by name; kept in memory only.
#[derive(Default)]
struct Dots(std::sync::Mutex<std::collections::BTreeSet<String>>);

/// The plugin tabs: each one's name, label and dot.
fn tabs(cx: &Cx) -> Vec<(String, String, bool)> {
    let hub = app_context::<Arc<Hub>>(cx);
    let dots = app_context::<Arc<Dots>>(cx).0.lock().map(|d| d.clone());
    let dots = dots.unwrap_or_default();
    let tabs = hub.plugins().into_iter();
    tabs.map(|p| {
        let dot = dots.contains(&p.name);
        (p.name, p.label, dot)
    })
    .collect()
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let providers = on_logins(cx, |l| Ok(l.statuses()));
    let rest = blocking(cx, |hub| {
        let agents = hub.agents();
        Ok((hub.list(), agents, hub.folders()))
    });
    let (providers, (rows, agents, folders)) =
        futures_util::future::try_join(providers, rest).await?;
    Ok(cooee_web::threads_page(
        cx,
        rows,
        agents,
        folders,
        providers,
        tabs(cx),
    ))
}

/// `POST /logins/{id}`: a login for `account` (none: the system default), or with `new` for a new account.
#[derive(Deserialize, Default)]
struct Start {
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    new: bool,
}

#[route(POST "/logins/{id}")]
async fn login_start(cx: &Cx, Json(s): Json<Option<Start>>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    let s = s.unwrap_or_default();
    Ok(Answer(
        on_logins(cx, move |l| l.start(&name, s.account.as_deref(), s.new)).await,
    ))
}

/// `POST /logins/{id}/signout` and `/default`: account `account` (empty: the system default).
#[derive(Deserialize)]
struct Pick {
    account: String,
}

#[route(POST "/logins/{id}/signout")]
async fn login_signout(cx: &Cx, Json(p): Json<Pick>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(
        on_logins(cx, move |l| l.sign_out(&name, &p.account)).await,
    ))
}

#[route(POST "/logins/{id}/default")]
async fn login_default(cx: &Cx, Json(p): Json<Pick>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(
        on_logins(cx, move |l| l.set_default(&name, &p.account)).await,
    ))
}

#[route(GET "/logins/{id}")]
async fn login_view(cx: &Cx) -> Result<Answer<LoginView>> {
    let name = thread_id(cx);
    Ok(Answer(on_logins(cx, move |l| l.view(&name)).await))
}

/// `POST /logins/{id}/send`: a code from the sign-in page.
#[derive(Deserialize)]
struct Code {
    text: String,
}

#[route(POST "/logins/{id}/send")]
async fn login_send(cx: &Cx, Json(c): Json<Code>) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(on_logins(cx, move |l| l.send(&name, &c.text)).await))
}

#[route(POST "/logins/{id}/cancel")]
async fn login_cancel(cx: &Cx) -> Result<Answer<()>> {
    let name = thread_id(cx);
    Ok(Answer(on_logins(cx, move |l| l.cancel(&name)).await))
}

#[page("/t/{id}")]
async fn chat(cx: &Cx) -> Result<impl View> {
    let id = thread_id(cx);
    let Ok((thread, turn)) = blocking(cx, move |hub| hub.thread(&id)).await else {
        return Err(not_found().into());
    };
    let choices = logins(cx).choices(&thread.agent);
    Ok(cooee_web::chat_page(cx, thread, turn, choices))
}

/// `?around=<id>&before=N&after=N`.
#[derive(Deserialize)]
struct SliceQuery {
    around: Option<u64>,
    #[serde(default)]
    before: Option<usize>,
    #[serde(default)]
    after: Option<usize>,
}

#[route(GET "/threads/{id}/messages")]
async fn messages(cx: &Cx) -> Result<Answer<Slice>> {
    let id = thread_id(cx);
    let q: SliceQuery =
        parse_query_params(cx).map_err(|e| topcoat::router::error::bad_request(e.to_string()))?;
    let (before, after) = (
        q.before.unwrap_or(50).min(500),
        q.after.unwrap_or(0).min(500),
    );
    Ok(Answer(
        blocking(cx, move |hub| hub.slice(&id, q.around, before, after)).await,
    ))
}

fn sse(hub: &Hub, out: &Out) -> Event {
    Event::new()
        .id(hub.event_id(out))
        .event(out.event)
        .data(out.data.as_str())
}

/// `?since=<cursor>`, for the first connection (a reconnect sends `Last-Event-ID`).
#[derive(Deserialize)]
struct Since {
    since: Option<String>,
}

#[route(GET "/threads/{id}/events")]
async fn events(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let id = thread_id(cx);
    let query: Option<Since> = parse_query_params(cx).ok();
    let cursor = last_event_id(cx)
        .map(str::to_owned)
        .or_else(|| query.and_then(|q| q.since))
        .unwrap_or_default();
    let hub = hub(cx);
    let (resume, rx) = hub.subscribe(&id, &cursor).map_err(|_| not_found())?;
    let (first, live): (Vec<Event>, bool) = match resume {
        Resume::Replay(outs) => (outs.iter().map(|o| sse(&hub, o)).collect(), true),
        Resume::Reset => (vec![Event::new().event("reset").data("{}")], false),
    };
    let rest = stream::unfold((hub, rx, live), |(hub, mut rx, live)| async move {
        if !live {
            return None;
        }
        // A page that fell behind gets disconnected and resumes from its last event.
        match rx.recv().await {
            Ok(out) => Some((Ok(sse(&hub, &out)), (hub, rx, live))),
            Err(RecvError::Lagged(_) | RecvError::Closed) => None,
        }
    });
    Ok(Sse::new(stream::iter(first.into_iter().map(Ok)).chain(rest)).keep_alive(KeepAlive::new()))
}

/// `GET /threads`: the thread list's cards under the device's chip, as the page draws them (its refresh, docs/api.md).
#[route(GET "/threads")]
async fn thread_list(cx: &Cx) -> Result<Answer<Vec<cooee_web::Card>>> {
    let list = cooee_web::chip(cx);
    Ok(Answer(
        blocking(cx, move |hub| Ok(cooee_web::cards(hub.list(), list))).await,
    ))
}

/// `POST /threads`: a new thread.
#[derive(Deserialize)]
struct New {
    agent: String,
    folder: PathBuf,
    text: String,
    random_id: String,
    /// The agent's provider account (none: its default).
    #[serde(default)]
    account: Option<String>,
    /// The model and effort (#553 row 34; empty: the agent's default).
    #[serde(default)]
    model: String,
    #[serde(default)]
    effort: String,
}

/// A new thread's id.
#[derive(Serialize)]
struct Created {
    id: String,
}

#[route(POST "/threads")]
async fn create(cx: &Cx, Json(new): Json<New>) -> Result<Answer<Created>> {
    let logins = logins(cx);
    let id = blocking(cx, move |hub| {
        let (account, env) = logins.home(&new.agent, new.account.as_deref())?;
        let words = logins.picks(&new.agent, &new.model, &new.effort)?;
        let launch = Launch {
            account: &account,
            env: env.as_ref().map(|(k, dir)| (k.as_str(), dir.as_path())),
            model: &new.model,
            effort: &new.effort,
            words,
        };
        hub.create(&new.agent, &new.folder, &new.text, &new.random_id, &launch)
    })
    .await;
    Ok(Answer(id.map(|id| Created { id })))
}

/// `?path=<folder>&hidden=1`: Settings › Folders.
#[derive(Deserialize)]
struct Browse {
    path: Option<PathBuf>,
    hidden: Option<String>,
}

#[route(GET "/folders")]
async fn browse(cx: &Cx) -> Result<Answer<Listing>> {
    let q: Option<Browse> = parse_query_params(cx).ok();
    let (path, hidden) = q.map_or((None, false), |q| (q.path, q.hidden.is_some()));
    Ok(Answer(
        blocking(cx, move |hub| hub.browse(path.as_deref(), hidden)).await,
    ))
}

/// `POST /roots`: a folder to add as a root, or to remove.
#[derive(Deserialize)]
struct Root {
    path: PathBuf,
    #[serde(default)]
    remove: bool,
}

#[route(POST "/roots")]
async fn root(cx: &Cx, Json(r): Json<Root>) -> Result<Answer<Vec<PathBuf>>> {
    Ok(Answer(
        blocking(cx, move |hub| hub.set_root(&r.path, !r.remove)).await,
    ))
}

/// `POST /folders/new`: a folder to make inside `parent`.
#[derive(Deserialize)]
struct Folder {
    parent: PathBuf,
    name: String,
}

#[route(POST "/folders/new")]
async fn new_folder(cx: &Cx, Json(f): Json<Folder>) -> Result<Answer<PathBuf>> {
    Ok(Answer(
        blocking(cx, move |hub| hub.new_folder(&f.parent, &f.name)).await,
    ))
}

/// `POST /threads/{id}/send`.
#[derive(Deserialize)]
struct Message {
    text: String,
    random_id: String,
}

#[route(POST "/threads/{id}/send")]
async fn send(cx: &Cx, Json(m): Json<Message>) -> Result<Answer<cooee_core::Item>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.send(&id, &m.text, &m.random_id)).await,
    ))
}

/// `POST /threads/{id}/approve`: a choice, an "other" text, or a secret card's `value` (passed to its plugin only).
#[derive(Deserialize)]
struct Approve {
    item: u64,
    option: String,
    #[serde(default)]
    value: Option<String>,
}

#[route(POST "/threads/{id}/approve")]
async fn approve(cx: &Cx, Json(a): Json<Approve>) -> Result<Answer<Option<cooee_core::Item>>> {
    let id = thread_id(cx);
    let by = request_context::<Paired>(cx).0.label.clone();
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.approve(&id, a.item, &a.option, a.value.as_deref(), &by)
        })
        .await,
    ))
}

/// `POST /threads/{id}/model` `{model, effort}`: the composer's chip (#553 row 34; empty: the default).
#[derive(Deserialize)]
struct ModelPick {
    #[serde(default)]
    model: String,
    #[serde(default)]
    effort: String,
}

#[route(POST "/threads/{id}/model")]
async fn model(cx: &Cx, Json(p): Json<ModelPick>) -> Result<Answer<cooee_core::Item>> {
    let (id, logins) = (thread_id(cx), logins(cx));
    Ok(Answer(
        blocking(cx, move |hub| {
            let agent = hub.thread(&id)?.0.agent;
            logins.picks(&agent, &p.model, &p.effort)?;
            hub.pick(&id, &p.model, &p.effort)
        })
        .await,
    ))
}

#[route(POST "/threads/{id}/cancel")]
async fn cancel(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.cancel(&id)).await))
}

#[route(POST "/threads/{id}/close")]
async fn close(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.close(&id)).await))
}

/// `POST /threads/{id}/read`.
#[derive(Deserialize)]
struct Read {
    item: u64,
}

#[route(POST "/threads/{id}/read")]
async fn read(cx: &Cx, Json(r): Json<Read>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.read(&id, r.item)).await))
}

/// `?code=` (the QR code's link fills it in) and `?bad` after a wrong code.
#[derive(Deserialize)]
struct PairQuery {
    code: Option<String>,
    bad: Option<String>,
}

#[page("/pair")]
async fn pairing(cx: &Cx) -> Result<impl View> {
    let q: Option<PairQuery> = parse_query_params(cx).ok();
    let (code, bad) = q.map_or((None, false), |q| (q.code, q.bad.is_some()));
    Ok(cooee_web::pair_page(cx, code.unwrap_or_default(), bad))
}

/// `POST /pair`: the code `cooee pair` printed.
#[derive(Deserialize)]
struct PairForm {
    code: String,
}

#[route(POST "/pair")]
async fn pair(cx: &Cx, Form(f): Form<PairForm>) -> Result<Response> {
    let devices = Devices::new(&app_context::<Data>(cx).0);
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let token = tokio::task::spawn_blocking(move || devices.claim(&f.code, &agent, now()))
        .await
        .map_err(io::Error::other)??;
    let Some(token) = token else {
        return Ok(Response::builder()
            .status(303)
            .header("Location", "/pair?bad=1")
            .body(Body::from(()))?);
    };
    Ok(Response::builder()
        .status(303)
        .header("Location", "/")
        .header("Set-Cookie", cookie(cx, &token))
        .body(Body::from(()))?)
}

/// The pairing cookie for `token` (`Secure` behind https).
fn cookie(cx: &Cx, token: &str) -> String {
    let secure = header(cx, "origin").is_some_and(|o| o.starts_with("https://"));
    format!(
        "{COOKIE}={token}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Strict{}",
        if secure { "; Secure" } else { "" }
    )
}

fn devices(cx: &Cx) -> Devices {
    Devices::new(&app_context::<Data>(cx).0)
}

/// `POST /pair/request`: a code for the phone to show, approved on the Mac with `cooee pair <code>`.
#[route(POST "/pair/request")]
async fn pair_request(cx: &Cx) -> Result<Response> {
    let devices = devices(cx);
    let (code, claim) = tokio::task::spawn_blocking(move || devices.request(now())).await??;
    Json(serde_json::json!({ "code": code, "claim": claim })).into_response(cx)
}

/// The phone's claim on its request.
#[derive(Deserialize)]
struct ClaimBody {
    claim: String,
}

/// `POST /pair/claim`: 200 with the cookie once approved, 202 while it waits, 410 when the request is gone.
#[route(POST "/pair/claim")]
async fn pair_claim(cx: &Cx, Json(c): Json<ClaimBody>) -> Result<Response> {
    let devices = devices(cx);
    let agent = header(cx, "user-agent").unwrap_or_default().to_owned();
    let found = tokio::task::spawn_blocking(move || devices.collect(&c.claim, &agent, now()));
    match found.await?? {
        Claim::Paired(token) => Ok(Response::builder()
            .header("Set-Cookie", cookie(cx, &token))
            .body(Body::from(()))?),
        Claim::Waiting => plain(202, "waiting for the Mac"),
        Claim::Gone => plain(410, "That code expired: ask for a new one."),
    }
}

#[route(POST "/pair/cancel")]
async fn pair_cancel(cx: &Cx, Json(c): Json<ClaimBody>) -> Result<Response> {
    let devices = devices(cx);
    tokio::task::spawn_blocking(move || devices.cancel(&c.claim)).await??;
    plain(200, "cancelled")
}

/// `GET /devices`: the paired devices, this one marked `me`.
#[route(GET "/devices")]
async fn device_list(cx: &Cx) -> Result<Response> {
    let (devices, me) = (devices(cx), request_context::<Paired>(cx).0.id.clone());
    let list = tokio::task::spawn_blocking(move || devices.list()).await??;
    let list: Vec<_> = list
        .into_iter()
        .map(|d| {
            serde_json::json!({ "id": d.id, "label": d.label, "created": d.created, "used": d.used, "me": d.id == me })
        })
        .collect();
    Json(list).into_response(cx)
}

/// A device to unpair.
#[derive(Deserialize)]
struct DeviceId {
    id: String,
}

/// `POST /devices/remove`: unpairs a device (its pushes stop too).
#[route(POST "/devices/remove")]
async fn device_remove(cx: &Cx, Json(d): Json<DeviceId>) -> Result<Response> {
    let devices = devices(cx);
    let id = d.id.clone();
    if tokio::task::spawn_blocking(move || devices.remove(&id)).await?? {
        plain(200, &format!("removed {}", d.id))
    } else {
        plain(404, &format!("No device {}.", d.id))
    }
}

/// The data folder.
struct Data(PathBuf);

path_param!(name);
path_param!(rest);

/// The plugin the path names.
fn named(cx: &Cx) -> Result<Plugin> {
    let name = path_param::<Name>(cx);
    let hub = app_context::<Arc<Hub>>(cx);
    let found = hub.plugins().into_iter().find(|p| p.name == name);
    Ok(found.ok_or_else(not_found)?)
}

#[page("/p/{name}")]
async fn tab(cx: &Cx) -> Result<impl View> {
    let p = named(cx)?;
    Ok(cooee_web::plugin_page(cx, tabs(cx), p.name, p.label))
}

#[route(GET "/p/{name}/frame")]
async fn frame(cx: &Cx) -> Result<Response> {
    let p = named(cx)?;
    match tokio::task::spawn_blocking(move || plugin::frame(&p, cooee_web::FRAME_JS)).await? {
        Ok((page, csp)) => Ok(Response::builder()
            .header("Content-Type", "text/html; charset=utf-8")
            .header("Content-Security-Policy", csp)
            .header("Cache-Control", "no-store")
            .body(Body::from(page))?),
        Err(e) => plain(502, &format!("plugin {}: {e}", path_param::<Name>(cx))),
    }
}

/// A backend's event as one `{event, data}` message (a page's `EventSource` hears only those), its id kept.
fn relay(e: plugin::Sse) -> Event {
    let event = if e.event.is_empty() {
        "message"
    } else {
        &e.event
    };
    let out = Event::new().data(serde_json::json!({ "event": event, "data": e.data }).to_string());
    match e.id {
        Some(id) => out.id(id),
        None => out,
    }
}

/// `/p/<name>/api/<path>`: `<path>` on the plugin's backend with its token. Only the content type, `Accept` and
/// `Last-Event-ID` go along (never the cookie); an event stream comes back as `{event, data}` messages.
#[route([GET, POST] "/p/{name}/api/{*rest}")]
async fn proxy(cx: &Cx, body: String) -> Result<Response> {
    let p = named(cx)?;
    let prefix = format!("/p/{}/api", p.name);
    let uri = uri(cx);
    let target = uri.path_and_query().map_or("/", |t| t.as_str());
    let target = target.strip_prefix(&prefix).unwrap_or(target).to_owned();
    let method = method(cx).to_string();
    let pass: Vec<(&str, String)> = ["content-type", "accept", "last-event-id"]
        .into_iter()
        .filter_map(|h| Some((h, header(cx, h)?.to_owned())))
        .collect();
    let reply = tokio::task::spawn_blocking(move || {
        let headers: Vec<(&str, &str)> = pass.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let timeout = Some(Duration::from_secs(30));
        plugin::call(&p, &method, &target, &headers, body.as_bytes(), timeout)
    })
    .await?;
    let reply = match reply {
        Ok(r) => r,
        Err(e) => return plain(502, &format!("{prefix}: {e}")),
    };
    if reply
        .header("content-type")
        .is_some_and(|t| t.starts_with("text/event-stream"))
    {
        reply.body.get_ref().set_read_timeout(None)?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || plugin::events(reply, |e| tx.send(e).is_ok()));
        let stream = stream::unfold(rx, |mut rx| async move {
            let e = rx.recv().await?;
            Some((Result::<Event>::Ok(relay(e)), rx))
        });
        return Sse::new(stream)
            .keep_alive(KeepAlive::new())
            .into_response(cx);
    }
    let status = reply.status;
    let kind = reply
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_owned();
    let bytes = tokio::task::spawn_blocking(move || reply.bytes()).await??;
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", kind)
        .header("Cache-Control", "no-store")
        .body(Body::from(bytes))?)
}

fn manager(cx: &Cx) -> Arc<Manager> {
    Arc::clone(app_context::<Arc<Manager>>(cx))
}

fn refused(e: impl std::fmt::Display) -> HubError {
    HubError::Refused(e.to_string())
}

/// `GET /plugins`: Settings › Plugins: the installed plugins with their state, the index, the jobs in progress.
#[route(GET "/plugins")]
async fn plugin_list(cx: &Cx) -> Result<Answer<serde_json::Value>> {
    let m = manager(cx);
    let list = tokio::task::spawn_blocking(move || {
        let rows = m.list().map_err(refused)?;
        let index = m.index();
        Ok(
            serde_json::json!({ "installed": rows, "index": index, "jobs": m.jobs(), "issues": ISSUES }),
        )
    });
    Ok(Answer(
        list.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

/// `?name=`: whose log.
#[derive(Deserialize)]
struct LogQuery {
    name: String,
}

/// `GET /plugins/log?name=`: the end of a plugin's log (its builds' and backend's output).
#[route(GET "/plugins/log")]
async fn plugin_log(cx: &Cx) -> Result<Response> {
    let q: LogQuery =
        parse_query_params(cx).map_err(|e| topcoat::router::error::bad_request(e.to_string()))?;
    let installed = manager(cx).installed()?;
    if !installed.iter().any(|l| l.name == q.name) {
        return plain(404, &format!("{} isn't installed.", q.name));
    }
    let text = std::fs::read(manager(cx).log(&q.name)).unwrap_or_default();
    let tail = String::from_utf8_lossy(&text[text.len().saturating_sub(16 << 10)..]).into_owned();
    plain(
        200,
        if tail.is_empty() {
            "(no log yet)"
        } else {
            &tail
        },
    )
}

/// `POST /plugins/<action>`: Settings › Plugins' buttons.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Act {
    name: String,
    source: String,
    rev: Option<String>,
    /// The commit the owner confirmed (add, update).
    commit: String,
    purge: bool,
    on: bool,
    text: String,
}

/// `[improve]`: a thread with the first agent (its default account) in the plugin's checkout, on its
/// `cooee-improve` branch, with the owner's `text`.
fn improve(
    m: &Manager,
    hub: &Hub,
    logins: &Logins,
    name: &str,
    text: &str,
) -> Result<String, HubError> {
    let (repo, manifest) = m.improve(name).map_err(refused)?;
    let ask = if text.trim().is_empty() {
        "Ask me what to change."
    } else {
        text.trim()
    };
    let prompt = format!(
        "You're improving the cooee plugin {name}: this folder is its checkout, on the branch cooee-improve. \
         cooee-plugin.toml says how it builds (`{}`) and runs; docs/plugins.md in cooee's repo has the plugin API. \
         Make the change here and build it to check it; don't push. When I approve in Settings › Plugins, cooee \
         commits the branch, builds it and swaps it in.\n\n{ask}",
        manifest.build
    );
    let rid = format!("improve-{name}-{}", now());
    let thread = agent_thread(hub, logins, &repo, &prompt, &rid)?;
    m.improving(name, Some(thread.clone())).map_err(refused)?;
    Ok(thread)
}

/// A thread with the first agent in shed (its default account) in `folder`, starting with `prompt`.
fn agent_thread(
    hub: &Hub,
    logins: &Logins,
    folder: &std::path::Path,
    prompt: &str,
    rid: &str,
) -> Result<String, HubError> {
    let agents = hub.agents().map_err(refused)?;
    let agent = agents.first().ok_or_else(|| {
        HubError::Refused("No agents in shed: run cooee doctor on the Mac.".to_owned())
    })?;
    let (account, env) = logins.home(&agent.id, None)?;
    let env = env.as_ref().map(|(k, dir)| (k.as_str(), dir.as_path()));
    let launch = Launch {
        account: &account,
        env,
        ..Launch::default()
    };
    hub.start(&agent.id, folder, prompt, rid, &launch)
}

fn themes(cx: &Cx) -> Arc<Themes> {
    Arc::clone(app_context::<Arc<Themes>>(cx))
}

fn looks(cx: &Cx) -> Arc<cooee_web::LookAssets> {
    Arc::clone(app_context::<Arc<cooee_web::LookAssets>>(cx))
}

/// `POST /themes/<action>`, Settings › look › themes' forms and a theme thread's `[keep]` (no script): `add`
/// (`source`), `remove` and `publish` (`name`), `new` and `remix` (`name`, `from`, `text`), `keep` (`name`, `thread`).
#[derive(Deserialize, Default)]
#[serde(default)]
struct ThemeAct {
    name: String,
    source: String,
    from: String,
    text: String,
    thread: String,
}

/// What the agent building theme `name` is told, before the owner's `text`.
fn theme_prompt(name: &str, text: &str) -> String {
    let ask = if text.trim().is_empty() {
        "Ask me what it should look like."
    } else {
        text.trim()
    };
    format!(
        "Theme {name}: {ask}\n\nYou're building the cooee theme {name}: this folder is its pack \
         (docs/themes.md in cooee's repo). theme.json has label, scheme {{dark, light}} (scheme names), sprites, \
         moments, words (keys: {}) and optionally scene (element ids in scene.svg, viewBox \"0 0 390 320\"). \
         sprites.svg holds <symbol>s {} (each s-<state>). Your own colour schemes go in schemes/<name>.json (label, \
         color-scheme, tokens {}), each passing WCAG AA. My phone shows a live preview that reloads on each save. \
         When I tap [keep], cooee runs the theme gate and lists here anything to fix.",
        cooee_web::WORDS.join(" "),
        cooee_web::SPRITES.map(|s| format!("s-{s}")).join(" "),
        cooee_web::TOKENS.join(" ")
    )
}

#[route(POST "/themes/{id}")]
async fn theme_act(cx: &Cx, Form(a): Form<ThemeAct>) -> Result<Response> {
    let (act, t, hub, logins, looks) = (thread_id(cx), themes(cx), hub(cx), logins(cx), looks(cx));
    let done = tokio::task::spawn_blocking(move || {
        let e = |e: io::Error| e.to_string();
        let back = "/#settings/look".to_owned();
        let done = match act.as_str() {
            "add" => t.plan(&a.source, None).and_then(|p| t.add(&p)).map_err(e).map(|()| back),
            "remove" => t.remove(&a.name).map_err(e).map(|()| back),
            "publish" => t.publish(&a.name).map_err(e).map(|(dir, commit)| {
                format!(
                    "{} is a git repo now, at {}: {}. Push it (gh repo create --source {2} --push), then \
                     propose it for cooee's plugins/index.toml by PR.",
                    a.name,
                    &commit[..7],
                    dir.display()
                )
            }),
            "new" | "remix" => {
                let from = (act == "remix").then_some(a.from.as_str());
                let dir = t.start(&a.name, from).map_err(e)?;
                let rid = format!("theme-{}-{}", a.name, now());
                let prompt = theme_prompt(&a.name, &a.text);
                let thread = agent_thread(&hub, &logins, &dir, &prompt, &rid);
                if thread.is_err() {
                    let _ = std::fs::remove_dir_all(&dir);
                }
                thread.map(|id| format!("/t/{id}")).map_err(|e| e.to_string())
            }
            "keep" => match t.keep(&a.name) {
                Ok(()) => Ok(back),
                Err(why) if !a.thread.is_empty() => {
                    let text = format!("[keep] {why}\n\nFix these; I'll tap keep again.");
                    let rid = format!("keep-{}", now());
                    let sent = hub.send(&a.thread, &text, &rid).map_err(|e| e.to_string());
                    sent.map(|_| format!("/t/{}", a.thread))
                }
                Err(why) => Err(why.to_string()),
            },
            _ => Err(format!("no theme action {act}")),
        };
        looks.reload();
        done
    });
    match done.await? {
        Ok(to) if to.starts_with('/') => Ok(Response::builder()
            .status(303)
            .header("Location", to)
            .body(Body::from(()))?),
        // `publish` says where its repo is.
        Ok(said) => plain(200, &said),
        Err(why) => plain(409, &why),
    }
}

/// `GET /themes/dev/<name>`: the live preview of a theme being built (`?thread=`: its chat too), its gate's result
/// and `[keep]`.
#[derive(Deserialize)]
struct PreviewQuery {
    thread: Option<String>,
}

#[page("/themes/dev/{id}")]
async fn preview(cx: &Cx) -> Result<impl View> {
    let name = thread_id(cx);
    let looks = looks(cx);
    let fails = tokio::task::spawn_blocking(move || {
        looks.reload();
        looks.looks().dev.get(&name).cloned()
    });
    let fails = fails.await?.ok_or_else(not_found)?;
    let q: Option<PreviewQuery> = parse_query_params(cx).ok();
    let thread = q.and_then(|q| q.thread).unwrap_or_default();
    Ok(cooee_web::preview_page(cx, thread_id(cx), fails, thread))
}

/// `GET /themes/dev/<name>/events`: a `saved` event each time a file of the theme changes, after the looks reload.
#[route(GET "/themes/dev/{id}/events")]
async fn theme_saves(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let name = thread_id(cx);
    let looks = looks(cx);
    if !looks.looks().dev.contains_key(&name) {
        return Err(not_found().into());
    }
    let dir = themes(cx).dev_dir(&name);
    let was = stamp(&dir);
    let saves = stream::unfold((dir, was, looks), |(dir, was, looks)| async move {
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let now = stamp(&dir);
            // Kept or gone: nothing more to preview.
            if !dir.exists() {
                return None;
            }
            if now != was {
                looks.reload();
                let saved = Event::new().event("saved").data("{}");
                return Some((Ok(saved), (dir, now, looks)));
            }
        }
    });
    Ok(Sse::new(saves).keep_alive(KeepAlive::new()))
}

#[route(POST "/plugins/{id}")]
async fn plugin_act(cx: &Cx, Json(a): Json<Act>) -> Result<Answer<serde_json::Value>> {
    let (act, m, hub, logins) = (thread_id(cx), manager(cx), hub(cx), logins(cx));
    let done = tokio::task::spawn_blocking(move || {
        let said = |what: &str| Ok(serde_json::json!(what));
        // A confirm holds for the exact commit it showed.
        let confirmed = |p: &Plan| {
            let again = "It changed since you confirmed: confirm again.";
            (p.commit == a.commit)
                .then_some(())
                .ok_or_else(|| HubError::Refused(again.to_owned()))
        };
        let done = match act.as_str() {
            "plan" if a.name.is_empty() => Ok(serde_json::json!(
                m.plan(&a.source, a.rev.as_deref()).map_err(refused)?
            )),
            "plan" => Ok(serde_json::json!(m.plan_update(&a.name).map_err(refused)?)),
            "add" => {
                let p = m.plan(&a.source, a.rev.as_deref()).map_err(refused)?;
                confirmed(&p)?;
                let name = p.manifest.name.clone();
                m.job(&name, "installing", move |m| m.add(&p))
                    .map_err(refused)?;
                said("installing")
            }
            "update" => {
                let p = m.plan_update(&a.name).map_err(refused)?;
                confirmed(&p)?;
                m.job(&a.name, "updating", move |m| m.update(&p))
                    .map_err(refused)?;
                said("updating")
            }
            "approve" => {
                let name = a.name.clone();
                let build = move |m: &Manager| m.approve(&name);
                m.job(&a.name, "building the improvement", build)
                    .map_err(refused)?;
                said("building the improvement")
            }
            "remove" => m
                .remove(&a.name, a.purge)
                .map_err(refused)
                .and_then(|()| said("removed")),
            "enable" => m
                .enable(&a.name, a.on)
                .map_err(refused)
                .and_then(|()| said(if a.on { "on" } else { "off" })),
            "rollback" => m
                .rollback(&a.name)
                .map_err(refused)
                .and_then(|()| said("rolled back")),
            "improve" => improve(&m, &hub, &logins, &a.name, &a.text)
                .map(|t| serde_json::json!({ "thread": t })),
            _ => Err(HubError::Refused(format!("no plugin action {act}"))),
        };
        // The tabs follow at once (the supervisor would within half a second).
        hub.set_plugins(m.tabs());
        done
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

/// `POST /plugin/push`: a line from a plugin; a tap opens its tab (at `#<path>`, when given). A newer push of the same
/// `tag` (default `plugin-<name>`) replaces the older one.
#[derive(Deserialize)]
struct PluginPush {
    text: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    tag: String,
}

#[route(POST "/plugin/push")]
async fn plugin_push(cx: &Cx, Json(b): Json<PluginPush>) -> Result<Response> {
    let p = &request_context::<Caller>(cx).0;
    if b.text.trim().is_empty() {
        return plain(400, "a push needs text");
    }
    let hash = if b.path.is_empty() {
        String::new()
    } else {
        format!("#{}", b.path)
    };
    let note = Note {
        title: p.label.clone(),
        body: b.text.chars().take(120).collect(),
        tag: if b.tag.is_empty() {
            format!("plugin-{}", p.name)
        } else {
            b.tag
        },
        url: format!("/p/{}{hash}", p.name),
        item: 0,
    };
    hub(cx).push().send(&note);
    Json(note).into_response(cx)
}

/// `POST /plugin/dot` `{"on": bool}`: shows or clears the dot on the plugin's tab.
#[derive(Deserialize)]
struct Dot {
    on: bool,
}

#[route(POST "/plugin/dot")]
async fn plugin_dot(cx: &Cx, Json(d): Json<Dot>) -> Result<Response> {
    let name = request_context::<Caller>(cx).0.name.clone();
    let dots = app_context::<Arc<Dots>>(cx);
    let mut dots = dots.0.lock().map_err(|e| io::Error::other(e.to_string()))?;
    if d.on {
        dots.insert(name);
    } else {
        dots.remove(&name);
    }
    Json(d.on).into_response(cx)
}

/// `POST /plugin/card`: a plugin's card in a thread ([`CardSpec`]).
#[route(POST "/plugin/card")]
async fn plugin_card(cx: &Cx, Json(c): Json<CardSpec>) -> Result<Answer<cooee_core::Item>> {
    let p = request_context::<Caller>(cx).0.clone();
    Ok(Answer(blocking(cx, move |hub| hub.card(&p, c)).await))
}

/// `POST /plugin/handoff` `{from, to, text}`: `from` shows `continued in` `to`, which gets `text` `from` it; `POST
/// /plugin/line` `{thread, from, text}`: a line flowing back into `thread` from thread `from` (#553 row 35).
#[derive(Deserialize)]
struct Handoff {
    #[serde(alias = "thread")]
    to: String,
    from: String,
    text: String,
}

#[route(POST "/plugin/handoff")]
async fn plugin_handoff(cx: &Cx, Json(h): Json<Handoff>) -> Result<Answer<cooee_core::Item>> {
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.link(&h.from, &h.to, "", true)?;
            hub.link(&h.to, &h.from, &h.text, false)
        })
        .await,
    ))
}

#[route(POST "/plugin/line")]
async fn plugin_line(cx: &Cx, Json(h): Json<Handoff>) -> Result<Answer<cooee_core::Item>> {
    if h.text.trim().is_empty() {
        return Ok(Answer(Err(HubError::Refused(
            "a line needs text".to_owned(),
        ))));
    }
    Ok(Answer(
        blocking(cx, move |hub| hub.link(&h.to, &h.from, &h.text, false)).await,
    ))
}

#[route(GET "/push/key")]
async fn push_key(cx: &Cx) -> Result<Response> {
    let hub = hub(cx);
    match tokio::task::spawn_blocking(move || hub.push().public_key()).await? {
        Ok(key) => Json(serde_json::json!({ "key": key })).into_response(cx),
        Err(e) => plain(500, &e.to_string()),
    }
}

/// `POST /push/subscribe`: the browser's subscription (`PushSubscription.toJSON()`'s endpoint and keys).
#[derive(Deserialize)]
struct Subscribe {
    endpoint: String,
    p256dh: String,
    auth: String,
}

#[route(POST "/push/subscribe")]
async fn push_subscribe(cx: &Cx, Json(s): Json<Subscribe>) -> Result<Response> {
    let sub = Sub {
        device: request_context::<Paired>(cx).0.id.clone(),
        endpoint: s.endpoint,
        p256dh: s.p256dh,
        auth: s.auth,
    };
    let hub = hub(cx);
    match tokio::task::spawn_blocking(move || hub.push().subscribe(sub)).await? {
        Ok(()) => plain(200, "alerts on"),
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => plain(400, &e.to_string()),
        Err(e) => plain(500, &e.to_string()),
    }
}

/// A subscription to drop.
#[derive(Deserialize)]
struct Endpoint {
    endpoint: String,
}

#[route(POST "/push/unsubscribe")]
async fn push_unsubscribe(cx: &Cx, Json(e): Json<Endpoint>) -> Result<Response> {
    let (hub, me) = (hub(cx), request_context::<Paired>(cx).0.id.clone());
    match tokio::task::spawn_blocking(move || hub.push().unsubscribe(&me, &e.endpoint)).await? {
        Ok(_) => plain(200, "alerts off"),
        Err(e) => plain(500, &e.to_string()),
    }
}

/// `GET /push/endpoints`: this device's subscriptions, so the page says truly whether alerts are on.
#[route(GET "/push/endpoints")]
async fn push_endpoints(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0.id;
    Json(hub(cx).push().endpoints(me)).into_response(cx)
}

/// `POST /push/test`: "alerts are on" to this device only.
#[route(POST "/push/test")]
async fn push_test(cx: &Cx) -> Result<Response> {
    let (hub, me) = (hub(cx), request_context::<Paired>(cx).0.id.clone());
    if hub.push().endpoints(&me).is_empty() {
        return plain(400, "Alerts are off on this device: turn them on first.");
    }
    let note = Note {
        title: "cooee".to_owned(),
        body: "alerts are on".to_owned(),
        tag: "test".to_owned(),
        url: "/".to_owned(),
        item: 0,
    };
    hub.push().send_to(Some(&me), &note);
    plain(200, "test alert sent")
}

/// `POST /report`: a problem reported from the page (report mode): the note and the element it is about.
#[derive(Deserialize, Serialize)]
struct Report {
    note: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    html: String,
    #[serde(default)]
    look: String,
    #[serde(default)]
    viewport: String,
}

/// Appends the report to `<data>/reports.jsonl`, with when and which device; `cooee reports` lists them.
#[route(POST "/report")]
async fn report(cx: &Cx, Json(r): Json<Report>) -> Result<Response> {
    if r.note.trim().is_empty() {
        return plain(400, "A report needs a note.");
    }
    let cut = |s: &str, n| s.chars().take(n).collect::<String>();
    let line = serde_json::json!({
        "at": now(), "device": request_context::<Paired>(cx).0.id, "note": cut(&r.note, 2000),
        "path": cut(&r.path, 300), "target": cut(&r.target, 300), "html": cut(&r.html, 600),
        "look": cut(&r.look, 200), "viewport": cut(&r.viewport, 40),
    });
    let file = app_context::<Data>(cx).0.join("reports.jsonl");
    tokio::task::spawn_blocking(move || {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)?;
        writeln!(f, "{line}")
    })
    .await??;
    plain(200, "report sent")
}

/// The web UI (with `looks`, its themes and schemes), the threads' API over `hub`, the providers' `logins`, the plugin `manager` and pairing, behind the gate: requests must name `bind`, localhost
/// or one of `allowed_hosts`; devices are paired in `data`.
#[must_use]
pub fn router(
    hub: Arc<Hub>,
    (logins, manager, themes): (Arc<Logins>, Arc<Manager>, Arc<Themes>),
    data: &std::path::Path,
    bind: IpAddr,
    allowed_hosts: &[String],
    looks: Arc<cooee_web::LookAssets>,
) -> Router {
    let mut names: Vec<String> = ["localhost", "127.0.0.1", "[::1]"]
        .map(str::to_owned)
        .into();
    if !bind.is_unspecified() {
        names.push(match bind {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => format!("[{ip}]"),
        });
    }
    names.extend(allowed_hosts.iter().cloned());
    let gate = Gate {
        names,
        devices: Devices::new(data),
        hub: Arc::clone(&hub),
    };
    cooee_web::assets(Router::builder(), looks)
        .layer(gate)
        .page(pairing)
        .route(pair)
        .route(push_key)
        .route(push_subscribe)
        .route(push_unsubscribe)
        .route(push_endpoints)
        .route(push_test)
        .route(pair_request)
        .route(pair_claim)
        .route(pair_cancel)
        .route(device_list)
        .route(device_remove)
        .route(report)
        .page(home)
        .page(chat)
        .route(thread_list)
        .route(messages)
        .route(events)
        .route(create)
        .route(browse)
        .route(root)
        .route(new_folder)
        .route(send)
        .route(approve)
        .route(cancel)
        .route(model)
        .route(close)
        .route(read)
        .route(login_start)
        .route(login_view)
        .route(login_send)
        .route(login_cancel)
        .route(login_signout)
        .route(login_default)
        .page(tab)
        .route(frame)
        .route(proxy)
        .route(plugin_push)
        .route(plugin_card)
        .route(plugin_handoff)
        .route(plugin_line)
        .route(plugin_list)
        .route(plugin_log)
        .route(plugin_act)
        .route(plugin_dot)
        .route(theme_act)
        .page(preview)
        .route(theme_saves)
        .app_context(hub)
        .app_context(Arc::new(Dots::default()))
        .app_context(logins)
        .app_context(manager)
        .app_context(themes)
        .app_context(Data(data.to_owned()))
        .build()
}

/// Opens the threads (`config`'s data folder and shed) and the looks (the built-in and own themes and schemes), and
/// serves the web UI on `bind:port` until Ctrl+C or
/// `SIGTERM`. `ready` gets the bound address first (port 0 picks a free one).
///
/// # Errors
///
/// Returns `Err` if `bind` isn't loopback and `allowed_hosts` is empty, a plugin is misconfigured (no token, …),
/// the store can't be opened, the runtime can't start, the port can't be bound or accepting a connection fails.
pub fn serve(config: &Config, ready: impl FnOnce(SocketAddr)) -> io::Result<()> {
    if !config.bind.is_loopback() && config.allowed_hosts.is_empty() {
        return Err(io::Error::other(format!(
            "listening on {} needs `allowed_hosts` (the names the phone uses, e.g. the Tailscale name)",
            config.bind
        )));
    }
    let manager = Manager::new(config)?;
    plugin::check(&manager.tabs())?;
    let data = config
        .data_dir()
        .ok_or_else(|| io::Error::other("no data folder: set `data` or HOME"))?;
    let shed = cooee_core::Shed::new(config.shed.clone(), config.shed_dir.clone());
    let themes = Themes::new(config)?;
    let (looks, refused) = cooee_web::LookAssets::load(
        config.dir.as_deref(),
        config.theme.as_deref(),
        themes.index().to_vec(),
    );
    for why in refused {
        eprintln!("cooee: refused {why}");
    }
    // A theme added, removed or saved anywhere (the CLI, an agent) shows within a second.
    if let Some(dir) = config.dir.clone() {
        let looks = Arc::clone(&looks);
        std::thread::spawn(move || {
            let seen =
                || stamp(&dir.join("themes")) ^ stamp(&dir.join("themes-dev")).rotate_left(1);
            let mut was = seen();
            loop {
                std::thread::sleep(Duration::from_secs(1));
                if seen() != was {
                    was = seen();
                    looks.reload();
                }
            }
        });
    }
    let theme = looks.looks().themes[&looks.looks().default].clone();
    let logins = Logins::new(shed.clone(), config.providers.clone(), &data);
    let hub = Hub::open(
        shed,
        &data,
        (config.roots.clone(), config.path.clone()),
        theme,
        manager.tabs(),
    )?;
    let router = router(
        Arc::clone(&hub),
        (logins, Arc::clone(&manager), Arc::new(themes)),
        &data,
        config.bind,
        &config.allowed_hosts,
        looks,
    );
    let served = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind((config.bind, config.port)).await?;
            let addr = listener.local_addr()?;
            manager.supervise(hub, format!("http://127.0.0.1:{}", addr.port()));
            ready(addr);
            topcoat::serve(listener, router).await
        });
    manager.stop_all();
    served
}
