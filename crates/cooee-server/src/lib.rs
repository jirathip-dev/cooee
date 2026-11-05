//! cooee's HTTP server on localhost: the web UI's pages (`cooee-web`), the threads' JSON API and their live events
//! over SSE (replayed from the last seen event on reconnect). Pairing and push come later.

#![forbid(unsafe_code)]

mod hub;

use std::{
    io,
    net::{Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use cooee_core::Config;
use futures_util::{Stream, StreamExt, stream};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, Router,
        content::{
            Json,
            sse::{Event, KeepAlive, Sse, last_event_id},
        },
        error::not_found,
        page, parse_query_params, path_param,
        response::{IntoResponse, Response},
        route,
    },
    view::View,
};

pub use hub::{Hub, HubError, Out, Resume, Slice};

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

/// The web UI (with `looks`, its themes and schemes) and the threads' API over `hub`.
#[must_use]
pub fn router(hub: Arc<Hub>, looks: cooee_web::Looks) -> Router {
    cooee_web::assets(Router::builder(), looks)
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
        .build()
}

/// Opens the threads (`config`'s data folder and shed) and the looks (the built-in and own themes and schemes), and
/// serves the web UI on `127.0.0.1:port` until Ctrl+C or
/// `SIGTERM`. `ready` gets the bound address first (port 0 picks a free one).
///
/// # Errors
///
/// Returns `Err` if the store can't be opened, the runtime can't start, the port can't be bound or accepting a
/// connection fails.
pub fn serve(config: &Config, ready: impl FnOnce(SocketAddr)) -> io::Result<()> {
    let data = config
        .data_dir()
        .ok_or_else(|| io::Error::other("no data folder: set `data` or HOME"))?;
    let shed = cooee_core::Shed::new(config.shed.clone(), config.shed_dir.clone());
    let hub = Hub::open(shed, &data, config.roots.clone())?;
    let (looks, refused) = cooee_web::Looks::load(config.dir.as_deref(), config.theme.as_deref());
    for why in refused {
        eprintln!("cooee: refused {why}");
    }
    let port = config.port;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port)).await?;
            ready(listener.local_addr()?);
            topcoat::serve(listener, router(hub, looks)).await
        })
}
