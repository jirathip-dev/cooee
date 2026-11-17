use std::{path::PathBuf, sync::Arc};

use cooee_machines::Machines;
use futures_util::{Stream, StreamExt, stream};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
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
    http::{
        Answer, Created, Data, Name, Paired, allowed, blocking, hub, logins, plain, profiles,
        provider_name, recipe, thread_id,
    },
    pairing, plugin,
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

#[route(GET "/threads")]
async fn thread_list(cx: &Cx) -> Result<Answer<Vec<cooee_core::Summary>>> {
    Ok(Answer(blocking(cx, |hub| Ok(hub.list())).await))
}

#[derive(Deserialize)]
struct New {
    agent: String,
    #[serde(default)]
    folder: PathBuf,
    text: String,
    random_id: String,
    #[serde(default)]
    account: Option<String>,
    #[serde(default)]
    background: bool,
    #[serde(default)]
    profile: Option<String>,
    #[serde(default)]
    model: String,
    #[serde(default)]
    effort: String,
    #[serde(default)]
    machine: Option<String>,
    #[serde(default)]
    setup: bool,
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
            (picked, new.setup),
        )?;
        if let Some(m) = new.machine {
            recipe.machine = m;
        }
        allowed(&machines, &recipe.machine).map_err(HubError::Refused)?;
        let id = if new.background {
            let name = provider_name(&agent, &logins.statuses());
            hub.create_later(
                (&agent, &name),
                &folder,
                (&new.text, &new.random_id),
                recipe,
            )?
        } else {
            hub.create(&agent, &folder, &new.text, &new.random_id, &recipe)?
        };
        let ours = list.into_iter().find(|a| a.id == profile).and_then(|a| {
            let name = a.id.split_once('/')?.1.to_owned();
            Some((hub.backend(&a.lent?.plugin)?, name))
        });
        if let Some((b, name)) = ours
            && let Err(e) = plugin::started(&b, &id, &name)
        {
            cooee_core::note!("cooee: plugin {}: thread-started: {e}", b.name);
        }
        Ok(id)
    });
    let id = id.await.map_err(|e| HubError::Refused(e.to_string()))?;
    Ok(Answer(id.map(|id| Created { id })))
}

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
            ((t.model, t.effort), t.setup),
        )?;
        let name = provider_name(&t.agent, &logins.statuses());
        hub.retry(&id, recipe, &name)
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

#[derive(Deserialize)]
struct Message {
    text: String,
    random_id: String,
    #[serde(default)]
    files: Vec<File>,
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

#[derive(Deserialize)]
struct Approve {
    item: u64,
    option: String,
    #[serde(default)]
    value: Option<String>,
    #[serde(default)]
    passkey: Option<pairing::Assertion>,
}

#[route(POST "/threads/{id}/approve")]
async fn approve(cx: &Cx, Json(a): Json<Approve>) -> Result<Response> {
    let id = thread_id(cx);
    if a.option == "approve" && hub(cx).is_proposal(&id, a.item) {
        let Some(p) = a.passkey else {
            return plain(403, "Approving this needs Face ID.");
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

#[route(POST "/threads/{id}/read")]
async fn read(cx: &Cx, Json(r): Json<Read>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.read(&id, r.item)).await))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(thread_list)
        .route(messages)
        .route(events)
        .route(create)
        .route(retry)
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
}
