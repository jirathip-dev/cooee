use std::sync::Arc;

use futures_util::{Stream, StreamExt, stream};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, RouterBuilder,
        content::{
            Json,
            sse::{Event, KeepAlive, Sse, last_event_id},
        },
        error::not_found,
        parse_query_params, path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    Hub, HubError, Out, Resume, Slice, Upload,
    http::{Answer, Created, Name, Tag, blocking, hub, image, logins, plain, thread_id},
    turns::{self, Again},
};

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
    let warm = Arc::clone(&hub);
    drop(tokio::task::spawn_blocking(move || {
        if let Err(e) = warm.pre_warm(&id) {
            cooee_core::note!("cooee: {id}: {e}");
        }
    }));
    let (first, live): (Vec<Event>, bool) = match resume {
        Resume::Replay(outs) => (outs.iter().map(|o| sse(&hub, o)).collect(), true),
        Resume::Reset => (vec![Event::new().event("reset").data("{}")], false),
    };
    let rest = stream::unfold((hub, rx, live), |(hub, mut rx, live)| async move {
        if !live {
            return None;
        }
        match rx.recv().await {
            Ok(out) => Some((Ok(sse(&hub, &out)), (hub, rx, live))),
            Err(RecvError::Lagged(_) | RecvError::Closed) => None,
        }
    });
    Ok(Sse::new(stream::iter(first.into_iter().map(Ok)).chain(rest)).keep_alive(KeepAlive::new()))
}

#[route(GET "/home/events")]
async fn home_events(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let rest = stream::unfold(hub(cx).home(), |mut rx| async move {
        match rx.recv().await {
            Ok(()) | Err(RecvError::Lagged(_)) => Some(((), rx)),
            Err(RecvError::Closed) => None,
        }
    });
    let all = stream::iter([()]).chain(rest);
    let all = all.map(|()| Ok(Event::new().event("changed").data("{}")));
    Ok(Sse::new(all).keep_alive(KeepAlive::new()))
}

#[derive(Deserialize)]
struct Of {
    persona: Option<String>,
    room: Option<String>,
}

#[route(GET "/threads")]
async fn thread_list(cx: &Cx) -> Result<Answer<Vec<cooee_core::Summary>>> {
    let of: Option<Of> = parse_query_params(cx).ok();
    let (persona, room) = of.map_or((None, None), |o| (o.persona, o.room));
    let seats = |r: &cooee_core::Summary| {
        let mut seats = r.members.clone();
        seats.sort();
        seats
    };
    Ok(Answer(
        blocking(cx, move |hub| {
            let mut rows = hub.list();
            let room = room.map(|id| rows.iter().find(|r| r.id == id).map(seats));
            rows.retain(|r| persona.as_ref().is_none_or(|p| &r.persona == p));
            rows.retain(|r| {
                room.as_ref()
                    .is_none_or(|m| m.as_ref().is_some_and(|m| !m.is_empty() && &seats(r) == m))
            });
            Ok(rows)
        })
        .await,
    ))
}

#[route(POST "/threads")]
async fn create(cx: &Cx, Json(new): Json<turns::Start>) -> Result<Answer<Created>> {
    Ok(Answer(turns::start(cx, new).await.map(|id| Created { id })))
}

async fn off<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, HubError> + Send + 'static,
) -> Result<Answer<T>> {
    Ok(Answer(turns::off(f).await))
}

#[route(POST "/threads/{id}/retry")]
async fn retry(cx: &Cx) -> Result<Answer<()>> {
    let a = Again::of(cx, thread_id(cx));
    off(move || a.retry()).await
}

#[route(POST "/threads/{id}/restart")]
async fn restart(cx: &Cx) -> Result<Answer<()>> {
    let a = Again::of(cx, thread_id(cx));
    off(move || a.restart()).await
}

#[derive(Deserialize)]
struct Message {
    text: String,
    random_id: Tag,
    #[serde(default)]
    files: Vec<File>,
    via: Option<cooee_core::Via>,
}

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
    let a = Again::of(cx, thread_id(cx));
    off(move || a.send((&m.text, &m.random_id.0), &m.uploads()?, m.via)).await
}

#[route(POST "/threads/{id}/handback")]
async fn hand_back(cx: &Cx, Json(m): Json<Message>) -> Result<Answer<cooee_core::Item>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.resume_handoff(&id, &m.text, &m.random_id.0)
        })
        .await,
    ))
}

#[route(POST "/threads/{id}/queue")]
async fn queue(cx: &Cx, Json(m): Json<Message>) -> Result<Answer<Vec<cooee_core::Queued>>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.queue(&id, &m.text, &m.random_id.0, &m.uploads()?, m.via)
        })
        .await,
    ))
}

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

#[route(GET "/threads/{id}/files/{name}")]
async fn thread_file(cx: &Cx) -> Result<Response> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    let found = blocking(cx, move |hub| hub.file(&id, &name)).await;
    let Ok(path) = found else {
        return plain(404, "No such file.");
    };
    let kind = image(&path);
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

#[route(GET "/threads/{id}/frames/{name}")]
async fn thread_frame(cx: &Cx) -> Result<Response> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    let Ok(bytes) = blocking(cx, move |hub| hub.frame_image(&id, &name)).await else {
        return plain(404, "No such frame.");
    };
    let kind = if bytes.starts_with(&[0xff, 0xd8]) {
        "image/jpeg"
    } else {
        "image/png"
    };
    Ok(Response::builder()
        .header("Content-Type", kind)
        .header("X-Content-Type-Options", "nosniff")
        .header("Cache-Control", "private, no-store")
        .body(Body::from(bytes))?)
}

#[route(POST "/threads/{id}/approve")]
async fn approve(cx: &Cx, Json(a): Json<turns::Approve>) -> Result<Response> {
    match turns::answer(cx, thread_id(cx), a).await? {
        Ok(done) => Answer(done).into_response(cx),
        Err(why) => plain(403, why),
    }
}

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
            logins.effort(&agent, &p.effort)?;
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

#[derive(Deserialize)]
struct Read {
    item: u64,
}

#[route(POST "/threads/{id}/typing")]
async fn typing(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.pre_warm(&id)).await))
}

#[route(POST "/threads/{id}/read")]
async fn read(cx: &Cx, Json(r): Json<Read>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.read(&id, r.item)).await))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(thread_list)
        .route(messages)
        .route(events)
        .route(home_events)
        .route(typing)
        .route(create)
        .route(retry)
        .route(restart)
        .route(send)
        .route(hand_back)
        .route(queue)
        .route(unqueue)
        .route(steer)
        .route(retitle)
        .route(archive)
        .route(thread_file)
        .route(thread_frame)
        .route(approve)
        .route(cancel)
        .route(model)
        .route(close)
        .route(read)
}
