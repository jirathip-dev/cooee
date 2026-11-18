use std::{sync::Arc, time::Duration};

use futures_util::{Stream, stream};
use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        RouterBuilder,
        content::{
            Json,
            sse::{Event, KeepAlive, Sse},
        },
        route,
    },
};

use crate::{
    Hub,
    http::{Answer, blocking, hub, thread_id},
    hub::{Input, Screen},
};

const EVERY: Duration = Duration::from_millis(500);

#[derive(Deserialize)]
struct Back {
    note: String,
    #[serde(default)]
    skill: bool,
}

#[route(GET "/personas/{id}/screen")]
async fn screen(cx: &Cx) -> Result<Answer<Screen>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.screen(&id)).await))
}

#[route(POST "/personas/{id}/screen/control")]
async fn take(cx: &Cx) -> Result<Answer<Screen>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.take_control(&id)).await))
}

#[route(POST "/personas/{id}/screen/back")]
async fn back(cx: &Cx, Json(b): Json<Back>) -> Result<Answer<Screen>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.hand_back(&id, b.note, b.skill)).await,
    ))
}

#[route(POST "/personas/{id}/screen/input")]
async fn input(cx: &Cx, Json(i): Json<Input>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.input(&id, &i)).await))
}

async fn next(hub: Arc<Hub>, id: String) -> Option<(Result<Event>, (Arc<Hub>, String))> {
    tokio::time::sleep(EVERY).await;
    let (h, i) = (Arc::clone(&hub), id.clone());
    let frame = tokio::task::spawn_blocking(move || h.frame(&i))
        .await
        .ok()?;
    let event = match frame {
        Ok(f) => Event::new()
            .event("frame")
            .data(serde_json::to_string(&f).unwrap_or_default()),
        Err(e) => Event::new()
            .event("gone")
            .data(serde_json::json!({ "why": e.to_string() }).to_string()),
    };
    Some((Ok(event), (hub, id)))
}

#[route(GET "/personas/{id}/screen/live")]
async fn live(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let s = stream::unfold((hub(cx), thread_id(cx)), |(h, i)| next(h, i));
    Ok(Sse::new(s).keep_alive(KeepAlive::new()))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(screen)
        .route(take)
        .route(back)
        .route(input)
        .route(live)
}
