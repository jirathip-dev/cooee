use std::{sync::Arc, time::Duration};

use cooee_core::browser::Control;
use futures_util::{Stream, stream};
use serde::{Deserialize, Serialize};
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
    Hub, HubError,
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

type Shoot = fn(&Hub, &str) -> Event;

fn shot<F: Serialize>(frame: Result<F, HubError>) -> Event {
    match frame {
        Ok(f) => Event::new()
            .event("frame")
            .data(serde_json::to_string(&f).unwrap_or_default()),
        Err(e) => Event::new()
            .event("gone")
            .data(serde_json::json!({ "why": e.to_string() }).to_string()),
    }
}

async fn next(
    (hub, id, shoot): (Arc<Hub>, String, Shoot),
) -> Option<(Result<Event>, (Arc<Hub>, String, Shoot))> {
    tokio::time::sleep(EVERY).await;
    let (h, i) = (Arc::clone(&hub), id.clone());
    let event = tokio::task::spawn_blocking(move || shoot(&h, &i))
        .await
        .ok()?;
    Some((Ok(event), (hub, id, shoot)))
}

fn stream(cx: &Cx, shoot: Shoot) -> Sse<impl Stream<Item = Result<Event>> + use<>> {
    let s = stream::unfold((hub(cx), thread_id(cx), shoot), next);
    Sse::new(s).keep_alive(KeepAlive::new())
}

#[route(GET "/personas/{id}/screen/live")]
async fn live(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    Ok(stream(cx, |h, i| shot(h.frame(i))))
}

#[route(GET "/desks/{id}/screen")]
async fn desk(cx: &Cx) -> Result<Answer<Control>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.desk_screen(&id)).await))
}

#[route(POST "/desks/{id}/screen/control")]
async fn desk_take(cx: &Cx) -> Result<Answer<Control>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.desk_take(&id)).await))
}

#[route(POST "/desks/{id}/screen/back")]
async fn desk_back(cx: &Cx) -> Result<Answer<Control>> {
    let id = thread_id(cx);
    Ok(Answer(blocking(cx, move |hub| hub.desk_back(&id)).await))
}

#[route(POST "/desks/{id}/screen/input")]
async fn desk_input(cx: &Cx, Json(i): Json<Input>) -> Result<Answer<()>> {
    let id = thread_id(cx);
    Ok(Answer(
        blocking(cx, move |hub| hub.desk_input(&id, &i)).await,
    ))
}

#[route(GET "/desks/{id}/screen/live")]
async fn desk_live(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    Ok(stream(cx, |h, i| shot(h.desk_frame(i))))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(screen)
        .route(take)
        .route(back)
        .route(input)
        .route(live)
        .route(desk)
        .route(desk_take)
        .route(desk_back)
        .route(desk_input)
        .route(desk_live)
}
