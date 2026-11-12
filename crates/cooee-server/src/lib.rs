//! cooee's HTTP server: the web UI's pages (`cooee-web`), the threads' JSON API and their live events over SSE
//! (replayed from the last seen event on reconnect), provider logins, pairing and web push. Every request must name one of the
//! server's own hosts (`Host`, and `Origin` when sent: no DNS rebinding), and every page and API call but pairing
//! and the static files needs a paired device's cookie; a plugin's backend posts with its token instead. Plugin tabs
//! (docs/plugins.md) run their module in a sandboxed frame and reach their backend through cooee's proxy.

#![forbid(unsafe_code)]

mod hub;
mod login;
mod manage;
pub mod mcp;
mod pairing;
mod passkey;
mod plugin;
mod push;
mod routines;
mod themes;

use std::{
    collections::BTreeMap,
    io,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Config, Device, Devices, Plugin, ShedError,
    personas::{self, Persona},
    skills,
};
use cooee_machines::Machines;
use futures_util::{Stream, StreamExt, stream};
use pairing::devices;
use passkey::Unlocks;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{
        Body, BodyLimit, Layer, LayerFuture, Next, Path, Router,
        content::{
            Form, Json,
            sse::{Event, KeepAlive, Sse, last_event_id},
        },
        error::not_found,
        page, parse_query_params, path_param,
        request::{headers, method, uri},
        response::{IntoResponse, Response},
        route, try_endpoint,
    },
    view::View,
};

pub use hub::{CardSpec, Hub, HubError, Listing, Out, Recipe, Resume, Slice, Upload};
pub use login::{LOGIN_TIMEOUT, Logins, View as LoginView};
pub use manage::{Backoff, ISSUES, Manager, Plan, Row};
pub use push::{Note, Push, Sub, encrypt, jwt, note};
pub use themes::{Pack, Pinned, Themes, stamp};

/// The paths an unpaired browser may load: pairing and the static files (and the themes' files under `/look/`).
const OPEN: [&str; 17] = [
    "/pair",
    "/look",
    "/look.js",
    "/pair.js",
    "/pair/request",
    "/pair/claim",
    "/pair/cancel",
    "/passkey/challenge",
    "/passkey/signin",
    "/manifest.webmanifest",
    "/icon.svg",
    "/icon-192.png",
    "/icon-512.png",
    "/app.css",
    "/chat.js",
    "/md.js",
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
            // A thread's MCP proxy carries the thread's token, checked by the call.
            if path.starts_with("/mcp/agent/") {
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
                        || path == "/new"
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

/// Times every request (#53): its method, route pattern, status and duration go to `perf.log`.
struct Timing;

impl Layer for Timing {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let start = std::time::Instant::now();
            let response = next.run(cx, body).await.into_response(cx)?;
            let route = try_endpoint(cx).map_or("(none)", |e| match e.path().as_str() {
                "" => "/",
                p => p,
            });
            let status = response.status().as_u16();
            cooee_core::perf::request(method(cx).as_str(), route, status, start);
            Ok(response)
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

/// The plugin tabs (a plugin with screens only has none): each one's name, label and dot.
fn tabs(cx: &Cx) -> Vec<(String, String, bool)> {
    let hub = app_context::<Arc<Hub>>(cx);
    let dots = app_context::<Arc<Dots>>(cx).0.lock().map(|d| d.clone());
    let dots = dots.unwrap_or_default();
    let tabs = hub.plugins().into_iter().filter(|p| !p.module.is_empty());
    tabs.map(|p| {
        let dot = dots.contains(&p.name);
        (p.name, p.label, dot)
    })
    .collect()
}

/// The sidebar's and a new chat's data (#86).
async fn side(cx: &Cx) -> Result<cooee_web::Side> {
    let providers = on_logins(cx, |l| Ok(l.statuses()));
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let rest = blocking(cx, move |hub| {
        let mut profiles = profiles(hub, (&data, config.as_deref()));
        profiles.retain(|p| !p.hidden);
        Ok((
            hub.list(),
            hub.agents(),
            hub.folders(),
            profiles,
            hub.models(),
        ))
    });
    let (providers, (rows, mut agents, folders, profiles, models)) =
        futures_util::future::try_join(providers, rest).await?;
    // A new chat runs on the main login: its models (#193).
    for a in agents.iter_mut().flatten() {
        let main = models
            .iter()
            .find(|o| o.agent == a.id && o.account.is_empty());
        a.models = main
            .map(cooee_core::models::Offered::picks)
            .unwrap_or_default();
    }
    Ok(cooee_web::Side {
        rows,
        profiles,
        agents,
        providers,
        folders,
        tabs: tabs(cx),
    })
}

/// `/new?agent=<ACP id>` (Plain) or `?profile=<id>`: a new chat with that agent; on `/` (`?list` keeps a phone on
/// the agents list) the agent on show.
#[derive(Deserialize)]
struct NewChat {
    list: Option<String>,
    agent: Option<String>,
    profile: Option<String>,
}

/// The query's agent as a [`cooee_web::Mate`] key (empty: none), and whether it has `?list`.
fn agent_key(cx: &Cx) -> (String, bool) {
    let q = parse_query_params::<NewChat>(cx).ok();
    let list = q.as_ref().is_some_and(|q| q.list.is_some());
    let key = match q {
        Some(NewChat {
            profile: Some(p), ..
        }) => format!("profile={p}"),
        Some(NewChat { agent: Some(a), .. }) => format!("agent={a}"),
        _ => String::new(),
    };
    (key, list)
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let (key, list) = agent_key(cx);
    let pane = cooee_web::Pane::Home { list, key };
    Ok(cooee_web::page(cx, side(cx).await?, pane))
}

#[page("/new")]
async fn new_chat(cx: &Cx) -> Result<impl View> {
    let (key, _) = agent_key(cx);
    Ok(cooee_web::page(
        cx,
        side(cx).await?,
        cooee_web::Pane::New { key },
    ))
}

/// `POST /agents` (a form): `+ new agent`, a name and the profile it starts from (empty: Plain on the first
/// installed agent); saved as a profile of its own, then its new chat opens.
#[derive(Deserialize)]
struct NewAgent {
    name: String,
    #[serde(default)]
    profile: String,
}

#[route(POST "/agents")]
async fn new_agent(cx: &Cx, Form(a): Form<NewAgent>) -> Result<Response> {
    let data = app_context::<Data>(cx);
    let (data, config) = (data.0.clone(), data.1.clone());
    let made = blocking(cx, move |hub| {
        let first = hub.agents().ok().and_then(|a| a.into_iter().next());
        let agent = first.map(|a| a.id).unwrap_or_default();
        let config = config.ok_or_else(|| refused("there is no config folder for profiles"))?;
        cooee_core::profiles::create(&config, &data, &a.name, &a.profile, &agent).map_err(refused)
    })
    .await;
    match made {
        Ok(p) => Ok(Response::builder()
            .status(303)
            .header("Location", format!("/new?profile={}", p.id()))
            .body(Body::from(()))?),
        Err(e) => plain(400, &e.to_string()),
    }
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

/// `POST /logins/{id}/signout`: account `account` (empty: the system default).
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

/// `POST /logins`: the sign-ins card opened; checks the accounts' status again, never waiting on it (#106).
#[route(POST "/logins")]
async fn logins_check(cx: &Cx) -> Result<Answer<()>> {
    logins(cx).ask();
    Ok(Answer(Ok(())))
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
    let side = side(cx).await?;
    let profile = side.profiles.iter().find(|p| p.id() == thread.profile);
    let label = profile.map_or(thread.profile.clone(), |p| p.label.clone());
    let models = hub(cx).offered(&thread.agent, &thread.account);
    let models = models.map(|o| o.picks()).unwrap_or_default();
    let choices = (models, logins(cx).efforts(&thread.agent));
    let folder = thread.folder.clone();
    let pane = cooee_web::Pane::Chat {
        thread: Box::new(thread),
        turn,
        profile: label,
        folder,
        choices,
    };
    Ok(cooee_web::page(cx, side, pane))
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

/// `GET /threads`: every thread's row, newest first (docs/api.md).
#[route(GET "/threads")]
async fn thread_list(cx: &Cx) -> Result<Answer<Vec<cooee_core::Summary>>> {
    Ok(Answer(blocking(cx, |hub| Ok(hub.list())).await))
}

/// `POST /threads`: a new thread.
#[derive(Deserialize)]
struct New {
    agent: String,
    /// None or empty: the thread's own scratch folder (#48).
    #[serde(default)]
    folder: PathBuf,
    text: String,
    random_id: String,
    /// The agent's provider account (none: the system default).
    #[serde(default)]
    account: Option<String>,
    /// Answer with the id at once and connect behind it (#48: the page opens the chat on send); else once the
    /// agent took the first message.
    #[serde(default)]
    background: bool,
    /// The agent profile's id (none or empty: Plain, with `agent` and `account`).
    #[serde(default)]
    profile: Option<String>,
    /// The model and effort (#553 row 34; empty: the agent's default).
    #[serde(default)]
    model: String,
    #[serde(default)]
    effort: String,
    /// The machine it runs on (#148): none, its persona's (this host without one); empty: this host.
    #[serde(default)]
    machine: Option<String>,
}

/// A new thread's id.
#[derive(Serialize)]
struct Created {
    id: String,
}

#[route(POST "/threads")]
async fn create(cx: &Cx, Json(new): Json<New>) -> Result<Answer<Created>> {
    let (logins, machines) = (logins(cx), Arc::clone(app_context::<Arc<Machines>>(cx)));
    let data = app_context::<Data>(cx);
    let (data, config, hub) = (data.0.clone(), data.1.clone(), hub(cx));
    let id = tokio::task::spawn_blocking(move || {
        let profile = new.profile.as_deref().unwrap_or_default();
        let list = profiles(&hub, (&data, config.as_deref()));
        let picked = (new.model, new.effort);
        let (agent, folder, mut recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list),
            (&new.agent, new.account.as_deref(), profile),
            new.folder,
            picked,
        )?;
        if let Some(m) = new.machine {
            recipe.machine = m;
        }
        allowed(&machines, &recipe.machine).map_err(HubError::Refused)?;
        let id = if new.background {
            let name = cooee_web::named(&agent, &logins.statuses());
            hub.create_later(
                (&agent, &name),
                &folder,
                (&new.text, &new.random_id),
                recipe,
            )?
        } else {
            hub.create(&agent, &folder, &new.text, &new.random_id, &recipe)?
        };
        // A plugin's profile: its backend is told the thread's id (so it can attach it).
        let ours = list.into_iter().find(|p| p.id() == profile);
        let ours = ours.and_then(|p| Some((backend(&hub, &data, p.plugin.as_deref()?)?, p.name)));
        if let Some((b, name)) = ours
            && let Err(e) = plugin::started(&b, &id, &name)
        {
            eprintln!("cooee: plugin {}: thread-started: {e}", b.name);
        }
        Ok(id)
    });
    let id = id.await.map_err(|e| HubError::Refused(e.to_string()))?;
    Ok(Answer(id.map(|id| Created { id })))
}

/// `POST /threads/{id}/retry`: starts a thread whose start failed again, with its first message (#48).
#[route(POST "/threads/{id}/retry")]
async fn retry(cx: &Cx) -> Result<Answer<()>> {
    let (id, hub, logins) = (thread_id(cx), hub(cx), logins(cx));
    let data = app_context::<Data>(cx);
    let (data, config) = (data.0.clone(), data.1.clone());
    let done = tokio::task::spawn_blocking(move || {
        let (t, _) = hub.thread(&id)?;
        let list = profiles(&hub, (&data, config.as_deref()));
        let (_, _, recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list),
            (&t.agent, Some(&t.account), &t.profile),
            PathBuf::new(),
            (t.model, t.effort),
        )?;
        let name = cooee_web::named(&t.agent, &logins.statuses());
        hub.retry(&id, recipe, &name)
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

/// `POST /threads/{id}/folder`: "Use a folder…" (#86).
#[derive(Deserialize)]
struct Attach {
    folder: PathBuf,
}

#[route(POST "/threads/{id}/folder")]
async fn attach(cx: &Cx, Json(a): Json<Attach>) -> Result<Answer<cooee_core::Item>> {
    let (id, hub, logins) = (thread_id(cx), hub(cx), logins(cx));
    let data = app_context::<Data>(cx);
    let (data, config) = (data.0.clone(), data.1.clone());
    let done = tokio::task::spawn_blocking(move || {
        let (t, _) = hub.thread(&id)?;
        let list = profiles(&hub, (&data, config.as_deref()));
        let (_, _, recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list),
            (&t.agent, Some(&t.account), &t.profile),
            PathBuf::new(),
            (t.model, t.effort),
        )?;
        hub.attach(&id, &a.folder, &recipe)
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

/// How a thread of `agent` starts in `folder`: Plain with `account`, or as agent profile `profile` (non-empty: its
/// agent, and its folder if it has one; one of `list`, not hidden), on the model and effort `picked`. Returns its
/// agent and folder too.
fn recipe(
    logins: &Logins,
    (data, config, list): (
        &std::path::Path,
        Option<&std::path::Path>,
        &[cooee_core::Profile],
    ),
    (agent, account, profile): (&str, Option<&str>, &str),
    folder: PathBuf,
    mut picked: (String, String),
) -> Result<(String, PathBuf, Recipe), HubError> {
    // A persona (#126): its agent, as the profile it runs as or Plain, in its folder, on its model and effort.
    let persona = personas::find(config, data, profile).ok();
    if let Some(p) = &persona {
        for (pick, default) in [(&mut picked.0, &p.model), (&mut picked.1, &p.effort)] {
            if pick.is_empty() {
                pick.clone_from(default);
            }
        }
    }
    let (agent, profile, folder) = match &persona {
        Some(p) if !["", "scratch"].contains(&p.folder.as_str()) => (
            p.agent.as_str(),
            p.profile.as_deref().unwrap_or(""),
            PathBuf::from(&p.folder),
        ),
        Some(p) => (p.agent.as_str(), p.profile.as_deref().unwrap_or(""), folder),
        None => (agent, profile, folder),
    };
    let (agent, folder, mut recipe) = if profile.is_empty() {
        let (account, launch) = logins.home(agent, account)?;
        let recipe = Recipe {
            account,
            launch,
            ..Recipe::default()
        };
        (agent.to_owned(), folder, recipe)
    } else {
        let p = list.iter().find(|p| p.id() == profile).cloned();
        let p = p.ok_or_else(|| refused(format!("no profile {profile}")))?;
        if p.hidden {
            return Err(refused(format!("profile {profile} is hidden: {}", p.why)));
        }
        let (account, launch, instructions) = logins.profile(&p, data)?;
        let recipe = Recipe {
            account,
            profile: p.id(),
            launch,
            instructions,
            mcp: p.mcp.iter().map(|k| k.server.clone()).collect(),
            ..Recipe::default()
        };
        (p.agent, p.folder.unwrap_or(folder), recipe)
    };
    recipe.launch.pick(logins.picks(&agent, &picked.1)?);
    (recipe.model, recipe.effort) = picked;
    if let Some(p) = persona {
        recipe.instructions = p.brief(data);
        recipe.profile = p.id;
        recipe.machine = p.machine;
    }
    Ok((agent, folder, recipe))
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

/// `POST /threads/{id}/send` and `/queue`: the text and its files.
#[derive(Deserialize)]
struct Message {
    text: String,
    random_id: String,
    #[serde(default)]
    files: Vec<File>,
}

/// A file sent with a message, its bytes in base64.
#[derive(Deserialize)]
struct File {
    name: String,
    data: String,
}

impl Message {
    fn uploads(&self) -> Result<Vec<Upload>, HubError> {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::STANDARD;
        let decode = |f: &File| b64.decode(&f.data).map(|data| (f.name.clone(), data));
        self.files
            .iter()
            .map(|f| decode(f).map(|(name, data)| Upload { name, data }))
            .collect::<Result<_, _>>()
            .map_err(|e| HubError::Refused(format!("a file isn't base64: {e}")))
    }
}

#[route(POST "/threads/{id}/send")]
async fn send(cx: &Cx, Json(m): Json<Message>) -> Result<Answer<cooee_core::Item>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.send(&id, &m.text, &m.random_id, &m.uploads()?)
        })
        .await,
    ))
}

#[route(POST "/threads/{id}/queue")]
async fn queue(cx: &Cx, Json(m): Json<Message>) -> Result<Answer<Vec<cooee_core::Queued>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.queue(&id, &m.text, &m.random_id, &m.uploads()?)
        })
        .await,
    ))
}

/// `POST /threads/{id}/unqueue` and `/steer`: a queued message.
#[derive(Deserialize)]
struct Queued {
    random_id: String,
}

#[route(POST "/threads/{id}/unqueue")]
async fn unqueue(cx: &Cx, Json(q): Json<Queued>) -> Result<Answer<Vec<cooee_core::Queued>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.unqueue(&id, &q.random_id, false)).await,
    ))
}

#[route(POST "/threads/{id}/steer")]
async fn steer(cx: &Cx, Json(q): Json<Queued>) -> Result<Answer<Vec<cooee_core::Queued>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.unqueue(&id, &q.random_id, true)).await,
    ))
}

/// `POST /threads/{id}/title`.
#[derive(Deserialize)]
struct Title {
    title: String,
}

#[route(POST "/threads/{id}/title")]
async fn retitle(cx: &Cx, Json(t): Json<Title>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.retitle(&id, &t.title)).await,
    ))
}

/// A room member's session starts as its persona's chat does (#154).
fn starter(
    logins: Arc<Logins>,
    data: &std::path::Path,
    config: Option<&std::path::Path>,
) -> hub::Starter {
    let (data, config) = (data.to_owned(), config.map(ToOwned::to_owned));
    Box::new(move |hub, persona| {
        let list = profiles(hub, (&data, config.as_deref()));
        let empty = (String::new(), String::new());
        let ask = ("", None, persona);
        recipe(
            &logins,
            (&data, config.as_deref(), &list),
            ask,
            PathBuf::new(),
            empty,
        )
    })
}

/// The routines' runner (#75): a `new_thread` run starts as the chat that made it did.
fn runner(
    hub: &Arc<Hub>,
    logins: Arc<Logins>,
    data: &std::path::Path,
    config: Option<&std::path::Path>,
) -> Arc<routines::Runner> {
    let (dir, config) = (data.to_owned(), config.map(ToOwned::to_owned));
    let data = dir.clone();
    let start: routines::Start = Box::new(move |hub, who| {
        let list = profiles(hub, (&data, config.as_deref()));
        let (agent, folder, mut recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list),
            (&who.agent, Some(&who.account), &who.profile),
            who.folder.clone(),
            (who.model.clone(), who.effort.clone()),
        )?;
        if recipe.machine.is_empty() {
            recipe.machine.clone_from(&who.machine);
        }
        let name = cooee_web::named(&agent, &logins.statuses());
        Ok((agent, name, folder, recipe))
    });
    Arc::new(routines::Runner::new(Arc::clone(hub), &dir, start))
}

/// `POST /rooms` (#154): a room of member personas, in room order.
#[derive(Deserialize)]
struct NewRoom {
    #[serde(default)]
    title: String,
    members: Vec<String>,
    random_id: String,
}

#[route(POST "/rooms")]
async fn room_new(cx: &Cx, Json(r): Json<NewRoom>) -> Result<Answer<Created>> {
    let id = blocking(cx, move |hub| {
        hub.create_room(&r.title, &r.members, &r.random_id)
    });
    Ok(Answer(id.await.map(|id| Created { id })))
}

/// `POST /threads/{id}/members`: a persona to add to the room, or to take out (its session closed).
#[derive(Deserialize)]
struct Seat {
    persona: String,
    #[serde(default)]
    remove: bool,
}

#[route(POST "/threads/{id}/members")]
async fn members(cx: &Cx, Json(m): Json<Seat>) -> Result<Answer<Vec<String>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.member(&id, &m.persona, m.remove)).await,
    ))
}

/// The room calls (#154).
fn room_routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(room_new).route(members)
}

/// `POST /threads/{id}/archive`: `on` archives it, else it is restored.
#[derive(Deserialize)]
struct Archive {
    on: bool,
}

#[route(POST "/threads/{id}/archive")]
async fn archive(cx: &Cx, Json(a): Json<Archive>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.archive(&id, a.on)).await,
    ))
}

/// `GET /threads/{id}/files/{name}`: a file sent in the thread. Only images show as themselves; anything else
/// downloads, and nothing runs (`sandbox`, `nosniff`).
#[route(GET "/threads/{id}/files/{name}")]
async fn thread_file(cx: &Cx) -> Result<Response> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    let found = blocking(cx, move |hub| hub.file(&id, &name)).await;
    let Ok(path) = found else {
        return plain(404, "No such file.");
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    let kind = match ext.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "heic" => Some("image/heic"),
        "avif" => Some("image/avif"),
        _ => None,
    };
    let bytes = tokio::task::spawn_blocking(move || std::fs::read(path)).await??;
    let r = Response::builder()
        .header("Content-Type", kind.unwrap_or("application/octet-stream"))
        .header("X-Content-Type-Options", "nosniff")
        .header("Content-Security-Policy", "sandbox")
        .header("Cache-Control", "private, max-age=31536000, immutable");
    let r = if kind.is_none() {
        r.header("Content-Disposition", "attachment")
    } else {
        r
    };
    Ok(r.body(Body::from(bytes))?)
}

/// `POST /threads/{id}/approve`: a choice, an "other" text, or a secret card's `value` (passed to its plugin only).
#[derive(Deserialize)]
struct Approve {
    item: u64,
    option: String,
    #[serde(default)]
    value: Option<String>,
    /// Approving an MCP server an agent proposed needs Face ID (#56).
    #[serde(default)]
    passkey: Option<pairing::Assertion>,
}

#[route(POST "/threads/{id}/approve")]
async fn approve(cx: &Cx, Json(a): Json<Approve>) -> Result<Response> {
    let id = thread_id(cx);
    if a.option == "approve" && hub(cx).is_proposal(&id, a.item) {
        let Some(p) = a.passkey else {
            return plain(403, "Approving an MCP server needs Face ID.");
        };
        if let Err(why) = pairing::assert(cx, p).await? {
            return plain(403, why);
        }
    }
    let by = request_context::<Paired>(cx).0.label.clone();
    Answer(
        blocking(cx, move |hub| {
            hub.approve(&id, a.item, &a.option, a.value.as_deref(), &by)
        })
        .await,
    )
    .into_response(cx)
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
            logins.picks(&agent, &p.effort)?;
            hub.pick(&id, &p.model, &p.effort)
        })
        .await,
    ))
}

/// `GET /models` (#193): each agent and account's models as last seen, each short name with what it resolves to now.
#[derive(Serialize)]
struct ModelList {
    agent: String,
    account: String,
    seen: u64,
    models: Vec<[String; 2]>,
    short: BTreeMap<String, String>,
}

/// `GET /agents` (#230): each installed agent with the efforts its provider offers.
#[derive(Serialize)]
struct AgentRow {
    id: String,
    efforts: Vec<String>,
}

#[route(GET "/agents")]
async fn agent_list(cx: &Cx) -> Result<Answer<Vec<AgentRow>>> {
    let logins = logins(cx);
    let rows = hub(cx).agents().map_err(refused).map(|agents| {
        let rows = agents.into_iter().map(|a| AgentRow {
            efforts: logins.efforts(&a.id),
            id: a.id,
        });
        rows.collect()
    });
    Ok(Answer(rows))
}

#[route(GET "/models")]
async fn model_list(cx: &Cx) -> Result<Answer<Vec<ModelList>>> {
    let lists = hub(cx).models().into_iter().map(|o| ModelList {
        short: o.short_names().into_iter().collect(),
        agent: o.agent,
        account: o.account,
        seen: o.seen,
        models: o.models,
    });
    Ok(Answer(Ok(lists.collect())))
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

/// `GET /devices`: the paired devices, this one marked `me`.
#[route(GET "/devices")]
async fn device_list(cx: &Cx) -> Result<Response> {
    let (devices, me) = (devices(cx), request_context::<Paired>(cx).0.id.clone());
    let list = tokio::task::spawn_blocking(move || devices.list()).await??;
    let list: Vec<_> = list
        .into_iter()
        .map(|d| {
            serde_json::json!({ "id": d.id, "label": d.label, "created": d.created, "used": d.used, "me": d.id == me, "by": d.by })
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

/// A device's new name.
#[derive(Deserialize)]
struct Rename {
    id: String,
    label: String,
}

/// `POST /devices/rename`: names a device (or the one an approved request becomes).
#[route(POST "/devices/rename")]
async fn device_rename(cx: &Cx, Json(r): Json<Rename>) -> Result<Response> {
    let (devices, label) = (
        devices(cx),
        r.label.trim().chars().take(40).collect::<String>(),
    );
    if label.is_empty() {
        return plain(400, "Give it a name.");
    }
    let to = label.clone();
    if tokio::task::spawn_blocking(move || devices.rename(&r.id, &to)).await?? {
        plain(200, &label)
    } else {
        plain(404, "That device isn't paired any more.")
    }
}

/// `POST /passkey/options`: a challenge for this device's passkey ceremony, and its passkey's id once it has one
/// (none: the page registers one).
#[route(POST "/passkey/options")]
async fn passkey_options(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0;
    let challenge = app_context::<Arc<Unlocks>>(cx).challenge(&me.id, now())?;
    let id = me.passkey.as_ref().map(|p| p.id.clone());
    let challenge = base64::Engine::encode(&base64::prelude::BASE64_URL_SAFE_NO_PAD, challenge);
    Json(serde_json::json!({ "challenge": challenge, "id": id })).into_response(cx)
}

/// `POST /passkey/register` (the device's first passkey) and `/passkey/unlock`: either unlocks the gated tab for
/// [`passkey::IDLE`]; each unlock goes to `<data>/terminal.log` with the time and device.
#[route(POST "/passkey/{rest}")]
async fn passkey_check(cx: &Cx, Json(p): Json<passkey::Proof>) -> Result<Response> {
    let me = request_context::<Paired>(cx).0.clone();
    let (unlocks, host) = (
        app_context::<Arc<Unlocks>>(cx),
        header(cx, "host").unwrap_or_default(),
    );
    let checked = match (path_param::<Rest>(cx), &me.passkey) {
        ("register", None) => match unlocks.register(&me.id, host, &p, now()) {
            Ok(pk) => {
                let (devices, id) = (devices(cx), me.id.clone());
                if tokio::task::spawn_blocking(move || devices.set_passkey(&id, pk)).await?? {
                    unlocks.unlock(&me.id, now());
                    Ok(())
                } else {
                    Err("this device has a passkey already".to_owned())
                }
            }
            Err(e) => Err(e),
        },
        ("unlock", Some(pk)) => unlocks.assert(&me.id, pk, host, &p, now()),
        ("register", Some(_)) => Err("this device has a passkey already".to_owned()),
        ("unlock", None) => Err("register this device's passkey first".to_owned()),
        _ => return Err(not_found().into()),
    };
    if let Err(why) = checked {
        return plain(403, &format!("Not unlocked: {why}."));
    }
    let line = serde_json::json!({ "at": now(), "device": me.id, "event": "unlock" });
    let file = app_context::<Data>(cx).0.join("terminal.log");
    tokio::task::spawn_blocking(move || cooee_core::append_line(&file, &line.to_string()))
        .await??;
    plain(200, "unlocked")
}

/// The data folder, and the config's folder (its `profiles/`).
struct Data(PathBuf, Option<PathBuf>);

/// The agent profiles (Plain aside), hidden ones too; a plugin's from its backend when it serves them, a `/path`
/// avatar through its proxy.
fn profiles(
    hub: &Hub,
    (data, config): (&std::path::Path, Option<&std::path::Path>),
) -> Vec<cooee_core::Profile> {
    let mut list = cooee_core::profiles::list(config, data, |name| {
        plugin::profiles(&backend(hub, data, name)?)
    });
    for p in &mut list {
        let via = p.plugin.as_deref().and_then(|n| backend(hub, data, n));
        if let (Some(via), Some(a)) = (via, &mut p.avatar)
            && a.starts_with('/')
        {
            *a = format!("/p/{}/api{a}", via.name);
        }
    }
    list
}

/// Installed plugin `name`'s backend: its first tab's entry (its URL and token).
fn backend(hub: &Hub, data: &std::path::Path, name: &str) -> Option<Plugin> {
    let lock = cooee_core::plugins::read_lock(data).ok()?;
    let tabs = lock.into_iter().find(|l| l.name == name)?.tabs;
    hub.plugins().into_iter().find(|p| tabs.contains(&p.name))
}

path_param!(name);
path_param!(rest);

/// The plugin tab the path names (a plugin with screens only has none).
fn named(cx: &Cx) -> Result<Plugin> {
    let name = path_param::<Name>(cx);
    let hub = app_context::<Arc<Hub>>(cx);
    let found = (hub.plugins().into_iter()).find(|p| p.name == name && !p.module.is_empty());
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
    let mut pass: Vec<(&str, String)> = ["content-type", "accept", "last-event-id"]
        .into_iter()
        .filter_map(|h| Some((h, header(cx, h)?.to_owned())))
        .collect();
    // The gated tab: only an unlocked device's calls get through, and each counts as use; the backend learns which
    // device (it refuses calls without one).
    let me = request_context::<Paired>(cx).0.id.clone();
    let unlocks = Arc::clone(app_context::<Arc<Unlocks>>(cx));
    let gated = p.name == passkey::GATED;
    if gated {
        if !unlocks.touch(&me, now()) {
            return Ok(Response::builder()
                .status(403)
                .header("Cooee-Locked", "1")
                .body(Body::from("locked"))?);
        }
        pass.push(("cooee-device", me.clone()));
    }
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
    // A locked device's stream ends at its next event.
    let open = move || !gated || unlocks.open(&me, now());
    pass_on(cx, reply, relay, open).await
}

/// A backend's answer passed on: an event stream event by event (each through `relay` while `open`), else its
/// status, content type and body.
async fn pass_on(
    cx: &Cx,
    reply: plugin::Reply,
    relay: fn(plugin::Sse) -> Event,
    open: impl Fn() -> bool + Send + 'static,
) -> Result<Response> {
    if reply
        .header("content-type")
        .is_some_and(|t| t.starts_with("text/event-stream"))
    {
        reply.body.get_ref().set_read_timeout(None)?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || plugin::events(reply, |e| open() && tx.send(e).is_ok()));
        let stream = stream::unfold(rx, move |mut rx| async move {
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

/// Installed plugin `{name}`'s backend and manifest, when it has places: only those have screens (#125).
fn with_screens(cx: &Cx) -> Option<(Plugin, cooee_core::plugins::Manifest)> {
    let name = path_param::<Name>(cx);
    let data = &app_context::<Data>(cx).0;
    let lock = cooee_core::plugins::read_lock(data).ok()?;
    let m = cooee_core::plugins::manifest(data, lock.iter().find(|l| l.name == name)?).ok()?;
    let p = backend(app_context::<Arc<Hub>>(cx), data, name)?;
    (!m.places.is_empty()).then_some((p, m))
}

/// A screen stream's event as the backend sent it: its `{seq, ops}` data, its name and id.
fn ops(e: plugin::Sse) -> Event {
    let out = Event::new().data(e.data);
    let out = if e.event.is_empty() {
        out
    } else {
        out.event(e.event)
    };
    match e.id {
        Some(id) => out.id(id),
        None => out,
    }
}

/// `GET /p/<plugin>/screens/<path>` (#125): a screen from a plugin with places, off its backend with its token
/// (`Accept` and `Last-Event-ID` go along, never the cookie); a stream of block ops comes back event by event.
#[route(GET "/p/{name}/screens/{*rest}")]
async fn screen(cx: &Cx) -> Result<Response> {
    let Some((p, m)) = with_screens(cx) else {
        return plain(404, "no installed plugin with screens has this name");
    };
    let uri = uri(cx);
    let target = uri.path_and_query().map_or("/", |t| t.as_str());
    let target = target
        .strip_prefix(&format!("/p/{}", path_param::<Name>(cx)))
        .unwrap_or(target)
        .to_owned();
    let path = target.split('?').next().unwrap_or_default();
    let declared = m.places.iter().any(|pl| {
        !pl.screen.is_empty()
            && path
                .strip_prefix(pl.screen.as_str())
                .is_some_and(|r| r.is_empty() || r.starts_with('/'))
    });
    if !declared || path.contains("..") || path.to_ascii_lowercase().contains("%2e") {
        return plain(
            403,
            &format!("{path} isn't under one of {}'s places' screens", m.name),
        );
    }
    let pass: Vec<(&str, String)> = ["accept", "last-event-id"]
        .into_iter()
        .filter_map(|h| Some((h, header(cx, h)?.to_owned())))
        .collect();
    let reply = tokio::task::spawn_blocking(move || {
        let headers: Vec<(&str, &str)> = pass.iter().map(|(k, v)| (*k, v.as_str())).collect();
        plugin::call(
            &p,
            "GET",
            &target,
            &headers,
            &[],
            Some(Duration::from_secs(5)),
        )
    })
    .await?;
    match reply {
        Ok(r) => pass_on(cx, r, ops, || true).await,
        Err(e) => plain(502, &format!("{}: {e}", path_param::<Name>(cx))),
    }
}

/// One action from a plugin's screen or card (#125): the block it came from and its values.
#[derive(Deserialize)]
struct BlockAct {
    #[serde(default)]
    screen: String,
    #[serde(default)]
    block: String,
    action: String,
    #[serde(default)]
    values: serde_json::Map<String, serde_json::Value>,
}

/// `POST /p/<plugin>/action` (#125): one action to its backend's `POST /action`, with the device that sent it as
/// `by`; one not in its manifest's `[permissions]` actions is refused (403) and never sent.
#[route(POST "/p/{name}/action")]
async fn screen_action(cx: &Cx, Json(a): Json<BlockAct>) -> Result<Response> {
    let Some((p, m)) = with_screens(cx) else {
        return plain(404, "no installed plugin with screens has this name");
    };
    if !m.permissions.actions.contains(&a.action) {
        return plain(
            403,
            &format!("{} isn't in {}'s [permissions] actions", a.action, m.name),
        );
    }
    let by = request_context::<Paired>(cx).0.label.clone();
    let body = serde_json::json!({
        "screen": a.screen, "block": a.block, "action": a.action, "values": a.values, "by": by,
    });
    let reply = tokio::task::spawn_blocking(move || {
        let json = [("Content-Type", "application/json")];
        let timeout = Some(Duration::from_secs(10));
        plugin::call(
            &p,
            "POST",
            "/action",
            &json,
            body.to_string().as_bytes(),
            timeout,
        )
    })
    .await?;
    match reply {
        Ok(r) => pass_on(cx, r, ops, || true).await,
        Err(e) => plain(502, &format!("{}: {e}", m.name)),
    }
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
    let (account, launch) = logins.home(&agent.id, None)?;
    let recipe = Recipe {
        account,
        launch,
        ..Recipe::default()
    };
    hub.start(&agent.id, folder, prompt, rid, &recipe)
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
         moments, words (keys: {}) and optionally scene (element ids in scene.svg, viewBox at most \"0 0 400 320\"; scene.svg is needed with sprites). \
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
                    let sent = hub.send(&a.thread, &text, &rid, &[]).map_err(|e| e.to_string());
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
        reset: None,
        apns: None,
    };
    let hub = hub(cx);
    match tokio::task::spawn_blocking(move || hub.push().subscribe(sub)).await? {
        Ok(()) => plain(200, "alerts on"),
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => plain(400, &e.to_string()),
        Err(e) => plain(500, &e.to_string()),
    }
}

/// `POST`/`DELETE /push/apns`: this native device's APNs token (`env`: default the config's).
#[derive(Deserialize)]
struct Token {
    token: String,
    #[serde(default)]
    env: Option<cooee_core::ApnsEnv>,
}

#[route(POST "/push/apns")]
async fn apns_add(cx: &Cx, Json(t): Json<Token>) -> Result<Response> {
    let (hub, me) = (hub(cx), request_context::<Paired>(cx).0.id.clone());
    match tokio::task::spawn_blocking(move || hub.push().register_apns(&me, &t.token, t.env))
        .await?
    {
        Ok(()) => plain(200, "alerts on"),
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => plain(400, &e.to_string()),
        Err(e) => plain(500, &e.to_string()),
    }
}

#[route(DELETE "/push/apns")]
async fn apns_remove(cx: &Cx, Json(t): Json<Token>) -> Result<Response> {
    let (hub, me) = (hub(cx), request_context::<Paired>(cx).0.id.clone());
    let token = t.token.to_ascii_lowercase();
    match tokio::task::spawn_blocking(move || hub.push().unsubscribe(&me, &token)).await? {
        Ok(_) => plain(200, "alerts off"),
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

/// `GET /push/reset`: why this device's alerts stopped (`null` unless they did), so the page asks to turn them on.
#[route(GET "/push/reset")]
async fn push_reset(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0.id;
    Json(hub(cx).push().reset(me)).into_response(cx)
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

/// A thread's MCP proxy or cooee's own server asking (#56): `{thread, server}`, with the thread's token.
#[derive(Deserialize)]
struct AgentAsk {
    thread: String,
    #[serde(default)]
    server: serde_json::Value,
}

/// The `Authorization: Bearer` token.
fn bearer(cx: &Cx) -> String {
    let token = header(cx, "authorization").and_then(|a| a.strip_prefix("Bearer "));
    token.unwrap_or_default().to_owned()
}

/// `POST /mcp/agent/remember` `{thread, server: {text?, replaces?, forget?}}`: `remember`'s edit (#126, #216).
#[route(POST "/mcp/agent/remember")]
async fn mcp_remember(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, e) = (bearer(cx), serde_json::from_value(a.server));
    let e: personas::Edit = e.map_err(|e| refused(format!("remember: {e}")))?;
    Ok(Answer(
        blocking(cx, move |hub| hub.remember(&a.thread, &token, &e)).await,
    ))
}

/// A persona call (#126; docs/api.md) off the async workers, given the data and config folders.
async fn on_personas<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&std::path::Path, &std::path::Path) -> io::Result<T> + Send + 'static,
) -> Result<Answer<T>> {
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let done = blocking(cx, move |_| {
        let config = config.ok_or_else(|| refused("there is no config folder for personas"))?;
        f(&data, &config).map_err(refused)
    });
    Ok(Answer(done.await))
}

#[route(GET "/personas")]
async fn persona_list(cx: &Cx) -> Result<Answer<Vec<Persona>>> {
    on_personas(cx, |d, c| Ok(personas::list(Some(c), d))).await
}

#[route(POST "/personas")]
async fn persona_new(cx: &Cx, Json(p): Json<Persona>) -> Result<Answer<Persona>> {
    let p = Persona {
        id: String::new(),
        ..p
    };
    persona_save(cx, p).await
}

/// Saves `p`, its plugins (#129) each an installed one and its machine (#148) an allowed one.
async fn persona_save(cx: &Cx, p: Persona) -> Result<Answer<Persona>> {
    let installed = hub(cx).installed();
    let machines = Arc::clone(app_context::<Arc<Machines>>(cx));
    on_personas(cx, move |d, c| {
        let why = match p.plugins.iter().find(|n| !installed.contains(n)) {
            Some(n) => Err(format!("no installed plugin {n}")),
            None => allowed(&machines, &p.machine),
        };
        why.map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        personas::save(c, d, p)
    })
    .await
}

/// Whether `machine` may run a thread (#148): this host (empty), or one added to shed (`shed machine ls`).
fn allowed(machines: &Machines, machine: &str) -> std::result::Result<(), String> {
    if machine.is_empty() || machines.added().iter().any(|m| m == machine) {
        Ok(())
    } else {
        Err(format!(
            "no machine {machine}: add it with shed machine add"
        ))
    }
}

/// One machine a thread may run on (#148): its name (empty: this host, `host` its host name), whether shed reaches
/// it, when it last answered and its health now (the machines plugin's card).
#[derive(Serialize)]
struct MachineRow {
    name: String,
    host: String,
    online: bool,
    last_seen: Option<u64>,
    health: Option<cooee_core::shed_proto::Sample>,
    error: Option<String>,
}

/// `GET /machines`: the allowed machines, this host first.
#[route(GET "/machines")]
async fn machine_list(cx: &Cx) -> Result<Answer<Vec<MachineRow>>> {
    let machines = Arc::clone(app_context::<Arc<Machines>>(cx));
    Ok(Answer(
        blocking(cx, move |_| {
            let cards = machines.poll(cooee_machines::unix_now());
            Ok(cards
                .into_iter()
                .enumerate()
                .map(|(i, c)| MachineRow {
                    name: if i == 0 {
                        String::new()
                    } else {
                        c.name.clone()
                    },
                    host: c.name,
                    online: c.online,
                    last_seen: c.last_seen,
                    health: c.now,
                    error: c.error,
                })
                .collect())
        })
        .await,
    ))
}

#[route(GET "/personas/{id}")]
async fn persona_get(cx: &Cx) -> Result<Answer<Persona>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| personas::find(Some(c), d, &id)).await
}

#[route(PUT "/personas/{id}")]
async fn persona_put(cx: &Cx, Json(p): Json<Persona>) -> Result<Answer<Persona>> {
    let p = Persona {
        id: thread_id(cx),
        ..p
    };
    persona_save(cx, p).await
}

/// `GET /personas/{id}/places`: its header strip, its attached plugins' `header-strip` chips (#125, q984).
#[route(GET "/personas/{id}/places")]
async fn places(cx: &Cx) -> Result<Answer<Vec<personas::Place>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| personas::places(Some(c), d, &id)).await
}

#[route(DELETE "/personas/{id}")]
async fn persona_delete(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| personas::remove(c, d, &id)).await
}

#[route(GET "/personas/{id}/memory")]
async fn memory(cx: &Cx) -> Result<Answer<Vec<personas::Note>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        Ok(personas::notes(d, &personas::find(Some(c), d, &id)?.id))
    })
    .await
}

async fn owner_edit(cx: &Cx, e: personas::Edit) -> Result<Answer<Option<personas::Note>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        let id = personas::find(Some(c), d, &id)?.id;
        personas::remember(d, &id, "owner", &e, now())
    })
    .await
}

#[route(POST "/personas/{id}/memory")]
async fn memory_add(cx: &Cx, Json(n): Json<Code>) -> Result<Answer<Option<personas::Note>>> {
    let e = personas::Edit {
        text: Some(n.text),
        ..personas::Edit::default()
    };
    owner_edit(cx, e).await
}

#[route(PUT "/personas/{id}/memory/{name}")]
async fn memory_put(cx: &Cx, Json(n): Json<Code>) -> Result<Answer<Option<personas::Note>>> {
    let e = personas::Edit {
        text: Some(n.text),
        replaces: Some(path_param::<Name>(cx).to_owned()),
        ..personas::Edit::default()
    };
    owner_edit(cx, e).await
}

#[route(DELETE "/personas/{id}/memory/{name}")]
async fn memory_drop(cx: &Cx) -> Result<Answer<Option<personas::Note>>> {
    let e = personas::Edit {
        forget: vec![path_param::<Name>(cx).to_owned()],
        ..personas::Edit::default()
    };
    owner_edit(cx, e).await
}

#[route(DELETE "/personas/{id}/memory")]
async fn memory_clear(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        personas::forget(d, &personas::find(Some(c), d, &id)?.id)
    })
    .await
}

/// The persona calls (#126), the machines and models they pick from, `remember`'s, `history`'s (#217) and `skill`'s
/// (#218), their skills, and their plugins' screens (#125).
fn persona_routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(persona_list)
        .route(machine_list)
        .route(model_list)
        .route(agent_list)
        .route(persona_new)
        .route(persona_get)
        .route(persona_put)
        .route(persona_delete)
        .route(places)
        .route(screen)
        .route(screen_action)
        .route(memory)
        .route(memory_add)
        .route(memory_put)
        .route(memory_drop)
        .route(memory_clear)
        .route(mcp_remember)
        .route(mcp_history)
        .route(skill_list)
        .route(skill_add)
        .route(skill_get)
        .route(skill_put)
        .route(skill_delete)
        .route(mcp_skill)
}

#[route(GET "/personas/{id}/skills")]
async fn skill_list(cx: &Cx) -> Result<Answer<Vec<skills::Skill>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        Ok(skills::list(d, &personas::find(Some(c), d, &id)?.id))
    })
    .await
}

#[route(POST "/personas/{id}/skills")]
async fn skill_add(cx: &Cx, Json(s): Json<skills::Skill>) -> Result<Answer<skills::Skill>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        skills::save(d, &personas::find(Some(c), d, &id)?.id, s, None)
    })
    .await
}

#[route(GET "/personas/{id}/skills/{name}")]
async fn skill_get(cx: &Cx) -> Result<Answer<skills::Skill>> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |d, c| {
        skills::read(d, &personas::find(Some(c), d, &id)?.id, &name)
    })
    .await
}

#[route(PUT "/personas/{id}/skills/{name}")]
async fn skill_put(cx: &Cx, Json(s): Json<skills::Skill>) -> Result<Answer<skills::Skill>> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |d, c| {
        skills::save(d, &personas::find(Some(c), d, &id)?.id, s, Some(&name))
    })
    .await
}

#[route(DELETE "/personas/{id}/skills/{name}")]
async fn skill_delete(cx: &Cx) -> Result<Answer<()>> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |d, c| {
        skills::remove(d, &personas::find(Some(c), d, &id)?.id, &name)
    })
    .await
}

/// `POST /mcp/agent/skill` `{thread, server: {read} | {propose: {name, description, body, replaces?}}}`: `skill`'s
/// text (#218): the body read, or that the card is up.
#[route(POST "/mcp/agent/skill")]
async fn mcp_skill(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let me = (a.thread.as_str(), token.as_str());
            if let Some(name) = a.server["read"].as_str() {
                return hub.read_skill(me, name);
            }
            let p = serde_json::from_value(a.server["propose"].clone())
                .map_err(|_| HubError::Refused("give read or propose".to_owned()))?;
            hub.propose_skill(me, p).map(|_| {
                "Proposed. The owner sees a card; nothing is saved until they approve it."
                    .to_owned()
            })
        })
        .await,
    ))
}

/// `POST /mcp/agent/grant` `{thread, server}`: the server resolved, in memory, and the tools it may offer.
#[route(POST "/mcp/agent/grant")]
async fn mcp_grant(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<mcp::Grant>> {
    let (token, server) = (bearer(cx), a.server.as_str().unwrap_or_default().to_owned());
    Ok(Answer(
        blocking(cx, move |hub| hub.grant(&a.thread, &token, &server)).await,
    ))
}

/// `POST /mcp/agent/ask` `{thread, server: {persona, message, timeout}}`: `ask_persona`'s reply (#156); `timeout`
/// in seconds, 300 unless given, at most 600.
#[route(POST "/mcp/agent/ask")]
async fn mcp_ask(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, s) = (bearer(cx), a.server);
    let text = |k: &str| s[k].as_str().unwrap_or_default().to_owned();
    let (persona, message) = (text("persona"), text("message"));
    let wait = Duration::from_secs(s["timeout"].as_u64().unwrap_or(300).min(600));
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.ask_persona((&a.thread, &token), &persona, &message, wait)
        })
        .await,
    ))
}

/// `POST /mcp/agent/history` `{thread, server: {query, thread, before, limit}}`: `history`'s text (#217).
#[route(POST "/mcp/agent/history")]
async fn mcp_history(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, s) = (bearer(cx), a.server);
    let text = |k: &str| s[k].as_str().unwrap_or_default().to_owned();
    let (query, thread, before) = (text("query"), text("thread"), s["before"].as_u64());
    let limit = usize::try_from(s["limit"].as_u64().unwrap_or(5)).unwrap_or(5);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.history((&a.thread, &token), (&query, limit), &thread, before)
        })
        .await,
    ))
}

/// `POST /mcp/agent/propose` `{thread, server: {name, type, command, args, url, why}}`: `propose_server`'s card.
#[route(POST "/mcp/agent/propose")]
async fn mcp_propose(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<cooee_core::Item>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let p =
                serde_json::from_value(a.server).map_err(|e| HubError::Refused(e.to_string()))?;
            hub.propose(&a.thread, &token, p)
        })
        .await,
    ))
}

/// `GET /mcp`: Settings › mcp servers.
#[page("/mcp")]
async fn mcp_settings(cx: &Cx) -> Result<impl View> {
    let data = app_context::<Data>(cx);
    let (d, c) = (data.0.clone(), data.1.clone());
    let page = blocking(cx, move |hub| {
        let servers = hub.mcp()?;
        let on = servers.iter().filter(|s| s.enabled);
        let tools = on
            .map(|s| (s.name.clone(), hub.mcp_tools(&s.name)))
            .collect();
        let profiles = cooee_core::profiles::list(c.as_deref(), &d, |_| None);
        let profiles = profiles
            .into_iter()
            .filter(|p| p.plugin.is_none())
            .collect();
        Ok(cooee_web::McpPage {
            servers,
            profiles,
            tools,
        })
    });
    Ok(cooee_web::mcp_page(
        cx,
        page.await.map_err(io::Error::from)?,
    ))
}

/// Back to Settings › mcp servers, or the reason.
fn to_mcp<T>(done: Result<T, HubError>) -> Result<Response> {
    match done {
        Ok(_) => Ok(Response::builder()
            .status(303)
            .header("Location", "/mcp")
            .body(Body::from(()))?),
        Err(e) => plain(400, &e.to_string()),
    }
}

/// The phone's `+ server`: its name, type, command line or URL, and the Face ID assertion (JSON) its card got.
#[derive(Deserialize)]
struct NewServer {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    target: String,
    #[serde(default)]
    passkey: Option<String>,
}

/// `POST /mcp/servers`: adds a server, off, after Face ID.
#[route(POST "/mcp/servers")]
async fn mcp_add(cx: &Cx, Form(n): Form<NewServer>) -> Result<Response> {
    let Some(p) = n.passkey.and_then(|p| serde_json::from_str(&p).ok()) else {
        return plain(
            403,
            "Adding an MCP server needs Face ID: approve it on its card.",
        );
    };
    if let Err(why) = pairing::assert(cx, p).await? {
        return plain(403, why);
    }
    let mut words = n.target.split_whitespace().map(str::to_owned);
    let stdio = n.kind == "stdio";
    let p = hub::Proposal {
        name: n.name.trim().to_owned(),
        command: words.next().filter(|_| stdio).unwrap_or_default(),
        args: if stdio { words.collect() } else { Vec::new() },
        url: if stdio {
            String::new()
        } else {
            n.target.trim().to_owned()
        },
        kind: n.kind,
        why: String::new(),
    };
    match blocking(cx, move |hub| hub.add_mcp(p.server())).await {
        Ok(said) => plain(200, &said),
        Err(e) => plain(400, &e.to_string()),
    }
}

/// `POST /mcp/servers/{name}` `enabled=on|off`: turns a server on or off. Its command, args, URL, env and headers
/// are the Mac's to change: a phone that sends one is refused.
#[route(POST "/mcp/servers/{id}")]
async fn mcp_toggle(cx: &Cx, Form(f): Form<BTreeMap<String, String>>) -> Result<Response> {
    if f.keys().any(|k| k != "enabled") {
        return plain(
            403,
            "A server's command, URL, env and headers change on the Mac only: `cooee mcp add` there.",
        );
    }
    let on = match f.get("enabled").map(String::as_str) {
        Some("on") => true,
        Some("off") => false,
        _ => return plain(400, "`enabled` is on or off."),
    };
    let name = thread_id(cx);
    to_mcp(
        blocking(cx, move |hub| {
            hub.edit_mcp(|list| {
                let s = list.iter_mut().find(|s| s.name == name);
                s.ok_or(format!("no MCP server {name}"))?.enabled = on;
                Ok(String::new())
            })
        })
        .await,
    )
}

/// `POST /mcp/servers/{name}/remove`.
#[route(POST "/mcp/servers/{id}/remove")]
async fn mcp_remove(cx: &Cx) -> Result<Response> {
    let name = thread_id(cx);
    to_mcp(
        blocking(cx, move |hub| {
            hub.edit_mcp(|list| {
                let had = list.len();
                list.retain(|s| s.name != name);
                (list.len() < had)
                    .then(String::new)
                    .ok_or(format!("no MCP server {name}"))
            })
        })
        .await,
    )
}

/// A secret for `secret:<NAME>`.
#[derive(Deserialize)]
struct NewSecret {
    name: String,
    value: String,
}

/// `POST /mcp/secret` `name, value`: saved straight to the keychain, never shown again.
#[route(POST "/mcp/secret")]
async fn mcp_secret(cx: &Cx, Form(s): Form<NewSecret>) -> Result<Response> {
    let data = app_context::<Data>(cx).0.clone();
    let saved = tokio::task::spawn_blocking(move || {
        cooee_core::mcp::set_secret(&data, s.name.trim(), &s.value)
    });
    to_mcp(saved.await?.map_err(|e| HubError::Refused(e.to_string())))
}

/// `POST /profiles/{id}/mcp`: the profile's servers (`s:<server>`) and tools (`t:<server>:<tool>`); every tool of a
/// server ticked is all of them, and a server whose tools couldn't be listed keeps the ones it had.
#[route(POST "/profiles/{id}/mcp")]
async fn profile_mcp(cx: &Cx, Form(f): Form<BTreeMap<String, String>>) -> Result<Response> {
    let (id, data) = (thread_id(cx), app_context::<Data>(cx));
    let (d, c) = (data.0.clone(), data.1.clone());
    to_mcp(
        blocking(cx, move |hub| {
            let had = cooee_core::profiles::find(c.as_deref(), &d, &id)
                .map_err(|e| HubError::Refused(e.to_string()))?
                .mcp;
            let picks: Vec<cooee_core::mcp::Pick> = f
                .keys()
                .filter_map(|k| k.strip_prefix("s:"))
                .map(|s| {
                    let tools = match hub.mcp_tools(s) {
                        Ok(all) if !all.is_empty() => {
                            let on: Vec<String> = all
                                .iter()
                                .map(|t| t.0.clone())
                                .filter(|t| f.contains_key(&format!("t:{s}:{t}")))
                                .collect();
                            (on.len() < all.len()).then_some(on)
                        }
                        _ => had
                            .iter()
                            .find(|k| k.server == s)
                            .and_then(|k| k.tools.clone()),
                    };
                    cooee_core::mcp::Pick {
                        server: s.to_owned(),
                        tools,
                    }
                })
                .collect();
            hub.set_picks(&id, &picks)
        })
        .await,
    )
}

/// The names a request may give as its Host: localhost, `bind` unless it is unspecified, and `allowed_hosts`.
fn names(bind: IpAddr, allowed_hosts: &[String]) -> Vec<String> {
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
    names
}

/// The web UI (with `looks`, its themes and schemes), the threads' API over `hub`, the providers' `logins`, the plugin `manager` and pairing, behind the gate: requests must name `bind`, localhost
/// or one of `allowed_hosts`; devices are paired in `data`.
#[must_use]
pub fn router(
    (hub, runner): (Arc<Hub>, Arc<routines::Runner>),
    (logins, manager, themes, machines): (Arc<Logins>, Arc<Manager>, Arc<Themes>, Arc<Machines>),
    (data, config): (&std::path::Path, Option<&std::path::Path>),
    bind: IpAddr,
    allowed_hosts: &[String],
    looks: Arc<cooee_web::LookAssets>,
) -> Router {
    hub.set_starter(starter(Arc::clone(&logins), data, config));
    let gate = Gate {
        names: names(bind, allowed_hosts),
        devices: Devices::new(data),
        hub: Arc::clone(&hub),
    };
    routines::routes(
        runner,
        room_routes(persona_routes(pairing::routes(
            cooee_web::assets(Router::builder(), looks)
                .layer(Timing)
                .layer(gate)
                // A message's photos come as base64.
                .layer(BodyLimit::max(64 << 20).at("/threads")),
        ))),
    )
    .route(push_key)
    .route(push_subscribe)
    .route(push_unsubscribe)
    .route(apns_add)
    .route(apns_remove)
    .route(push_endpoints)
    .route(push_reset)
    .route(push_test)
    .route(device_list)
    .route(device_remove)
    .route(device_rename)
    .route(passkey_options)
    .route(passkey_check)
    .route(report)
    .page(home)
    .page(chat)
    .page(new_chat)
    .route(thread_list)
    .route(messages)
    .route(events)
    .route(create)
    .route(retry)
    .route(new_agent)
    .route(attach)
    .route(browse)
    .route(root)
    .route(new_folder)
    .route(send)
    .route(queue)
    .route(unqueue)
    .route(steer)
    .route(retitle)
    .route(archive)
    .route(thread_file)
    .route(approve)
    .route(cancel)
    .route(model)
    .route(close)
    .route(read)
    .route(login_start)
    .route(logins_check)
    .route(login_view)
    .route(login_send)
    .route(login_cancel)
    .route(login_signout)
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
    .route(mcp_grant)
    .route(mcp_propose)
    .route(mcp_ask)
    .page(mcp_settings)
    .route(mcp_add)
    .route(mcp_toggle)
    .route(mcp_remove)
    .route(mcp_secret)
    .route(profile_mcp)
    .app_context(hub)
    .app_context(Arc::new(Dots::default()))
    .app_context(Arc::new(Unlocks::default()))
    .app_context(logins)
    .app_context(manager)
    .app_context(themes)
    .app_context(machines)
    .app_context(Data(data.to_owned(), config.map(ToOwned::to_owned)))
    .app_context(Arc::new(pairing::Pairing::default()))
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
    cooee_core::perf::init(&data, config.perf.slow_ms)?;
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
    let logins = Logins::new(
        shed.clone(),
        cooee_core::Providers::new(&config.providers),
        &data,
    );
    let hub = Hub::open(
        shed,
        &data,
        (config.roots.clone(), config.path.clone()),
        theme,
        manager.tabs(),
        config.apns.clone(),
    )?;
    // The models' daily refresh (#193): each agent and account seen a day ago or more is asked again.
    let (daily, signins) = (Arc::clone(&hub), Arc::clone(&logins));
    std::thread::spawn(move || {
        loop {
            for (agent, account) in daily.stale(hub::now()) {
                let launch = signins.home(&agent, Some(&account));
                let probed = launch.and_then(|(_, l)| daily.probe(&agent, &account, &l));
                if let Err(e) = probed {
                    eprintln!("cooee: models of {agent}: {e}");
                }
            }
            std::thread::sleep(Duration::from_hours(1));
        }
    });
    let runner = runner(&hub, Arc::clone(&logins), &data, config.dir.as_deref());
    let router = router(
        (Arc::clone(&hub), Arc::clone(&runner)),
        (
            logins,
            Arc::clone(&manager),
            Arc::new(themes),
            Arc::new(Machines::new(
                config.shed.clone(),
                config.shed_dir.clone(),
                cooee_machines::local_name(),
            )),
        ),
        (&data, config.dir.as_deref()),
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
            hub.listening(format!("http://127.0.0.1:{}", addr.port()));
            manager.supervise(hub, format!("http://127.0.0.1:{}", addr.port()));
            routines::Runner::every(runner, Duration::from_secs(30));
            ready(addr);
            topcoat::serve(listener, router).await
        });
    manager.stop_all();
    served
}
