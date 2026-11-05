//! cooee's HTTP server: the web UI's pages (`cooee-web`), the threads' JSON API and their live events over SSE
//! (replayed from the last seen event on reconnect), pairing and web push. Every request must name one of the
//! server's own hosts (`Host`, and `Origin` when sent: no DNS rebinding), and every page and API call but pairing
//! and the static files needs a paired device's cookie.

#![forbid(unsafe_code)]

mod hub;
mod push;

use std::{
    io,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{Config, Device, Devices};
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

pub use hub::{Hub, HubError, Out, Resume, Slice};
pub use push::{Note, Push, Sub, encrypt, jwt, note};

/// The paths an unpaired browser may load: pairing and the static files.
const OPEN: [&str; 8] = [
    "/pair",
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

fn plain(status: u16, text: &str) -> Result<Response> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Body::from(text.to_owned()))?)
}

fn header<'a>(cx: &'a Cx, name: &str) -> Option<&'a str> {
    headers(cx).get(name).and_then(|v| v.to_str().ok())
}

/// The request's paired device, put in the request context by the [`Gate`].
struct Paired(Device);

/// Checks every request: its `Host` (and `Origin`, when sent) must be one of `names`, and a page or API call must
/// carry a paired device's cookie (an unpaired page load goes to `/pair`).
struct Gate {
    /// localhost, the bind address and `allowed_hosts`; matched exactly, so a `*` never widens them.
    names: Vec<String>,
    devices: Devices,
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
            if OPEN.contains(&path) {
                return next.run(cx, body).await;
            }
            let token = header(cx, "cookie").and_then(|c| {
                c.split(';')
                    .find_map(|c| c.trim().strip_prefix(COOKIE)?.strip_prefix('='))
            });
            match token.and_then(|t| self.devices.find(t, now())) {
                Some(d) => next.run(&cx.with(Paired(d)), body).await,
                None if method(cx) == "GET" && (path == "/" || path.starts_with("/t/")) => {
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
            Err(e @ HubError::Shed(_)) => (502, e.to_string()),
            Err(e @ HubError::Store(_)) => (500, e.to_string()),
        };
        Ok(Response::builder()
            .status(status)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Body::from(body))?)
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let (rows, agents, folders) = blocking(cx, |hub| {
        let agents = hub.agents().map_err(|e| e.to_string());
        Ok((hub.list(), agents, hub.folders()))
    })
    .await?;
    Ok(cooee_web::threads_page(cx, rows, agents, folders))
}

#[page("/t/{id}")]
async fn chat(cx: &Cx) -> Result<impl View> {
    let id = thread_id(cx);
    let Ok((thread, turn)) = blocking(cx, move |hub| hub.thread(&id)).await else {
        return Err(not_found().into());
    };
    Ok(cooee_web::chat_page(cx, thread, turn))
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

/// `POST /threads`: a new thread.
#[derive(Deserialize)]
struct New {
    agent: String,
    folder: PathBuf,
    text: String,
    random_id: String,
}

/// A new thread's id.
#[derive(Serialize)]
struct Created {
    id: String,
}

#[route(POST "/threads")]
async fn create(cx: &Cx, Json(new): Json<New>) -> Result<Answer<Created>> {
    let id = blocking(cx, move |hub| {
        hub.create(&new.agent, &new.folder, &new.text, &new.random_id)
    })
    .await;
    Ok(Answer(id.map(|id| Created { id })))
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

/// `POST /threads/{id}/approve`.
#[derive(Deserialize)]
struct Approve {
    item: u64,
    option: String,
}

#[route(POST "/threads/{id}/approve")]
async fn approve(cx: &Cx, Json(a): Json<Approve>) -> Result<Answer<Option<cooee_core::Item>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.approve(&id, a.item, &a.option)).await,
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
    let secure = header(cx, "origin").is_some_and(|o| o.starts_with("https://"));
    let cookie = format!(
        "{COOKIE}={token}; Path=/; Max-Age=31536000; HttpOnly; SameSite=Strict{}",
        if secure { "; Secure" } else { "" }
    );
    Ok(Response::builder()
        .status(303)
        .header("Location", "/")
        .header("Set-Cookie", cookie)
        .body(Body::from(()))?)
}

/// The data folder.
struct Data(PathBuf);

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

/// The web UI, the threads' API over `hub` and pairing, behind the gate: requests must name `bind`, localhost
/// or one of `allowed_hosts`; devices are paired in `data`.
#[must_use]
pub fn router(
    hub: Arc<Hub>,
    data: &std::path::Path,
    bind: IpAddr,
    allowed_hosts: &[String],
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
    };
    cooee_web::assets(Router::builder())
        .layer(gate)
        .page(pairing)
        .route(pair)
        .route(push_key)
        .route(push_subscribe)
        .page(home)
        .page(chat)
        .route(messages)
        .route(events)
        .route(create)
        .route(send)
        .route(approve)
        .route(cancel)
        .route(close)
        .route(read)
        .app_context(hub)
        .app_context(Data(data.to_owned()))
        .build()
}

/// Opens the threads (`config`'s data folder and shed) and serves the web UI on `bind:port` until Ctrl+C or
/// `SIGTERM`. `ready` gets the bound address first (port 0 picks a free one).
///
/// # Errors
///
/// Returns `Err` if `bind` isn't loopback and `allowed_hosts` is empty, the store can't be opened, the runtime
/// can't start, the port can't be bound or accepting a connection fails.
pub fn serve(config: &Config, ready: impl FnOnce(SocketAddr)) -> io::Result<()> {
    if !config.bind.is_loopback() && config.allowed_hosts.is_empty() {
        return Err(io::Error::other(format!(
            "listening on {} needs `allowed_hosts` (the names the phone uses, e.g. the Tailscale name)",
            config.bind
        )));
    }
    let data = config
        .data_dir()
        .ok_or_else(|| io::Error::other("no data folder: set `data` or HOME"))?;
    let shed = cooee_core::Shed::new(config.shed.clone(), config.shed_dir.clone());
    let hub = Hub::open(shed, &data, config.roots.clone())?;
    let router = router(hub, &data, config.bind, &config.allowed_hosts);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind((config.bind, config.port)).await?;
            ready(listener.local_addr()?);
            topcoat::serve(listener, router).await
        })
}
