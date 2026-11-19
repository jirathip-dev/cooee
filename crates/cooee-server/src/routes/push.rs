use std::io;

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, request_context},
    router::{
        RouterBuilder,
        content::Json,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    Note, Sub,
    http::{Paired, hub, plain},
};

#[route(GET "/push/key")]
async fn push_key(cx: &Cx) -> Result<Response> {
    let hub = hub(cx);
    match tokio::task::spawn_blocking(move || hub.push().public_key()).await? {
        Ok(key) => Json(serde_json::json!({ "key": key })).into_response(cx),
        Err(e) => plain(500, &e.to_string()),
    }
}

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

#[derive(Deserialize)]
struct ActivityToken {
    token: String,
    #[serde(default)]
    activity: Option<String>,
    #[serde(default)]
    env: Option<cooee_core::ApnsEnv>,
}

#[route(POST "/push/activity")]
async fn activity_add(cx: &Cx, Json(t): Json<ActivityToken>) -> Result<Response> {
    let (hub, me) = (hub(cx), request_context::<Paired>(cx).0.id.clone());
    let push = move || {
        hub.push()
            .register_activity(&me, &t.token, t.activity, t.env)
    };
    match tokio::task::spawn_blocking(push).await? {
        Ok(()) => plain(200, "live activity on"),
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => plain(400, &e.to_string()),
        Err(e) => plain(500, &e.to_string()),
    }
}

#[route(GET "/glance")]
async fn glance(cx: &Cx) -> Result<Response> {
    let hub = hub(cx);
    Json(tokio::task::spawn_blocking(move || hub.overview()).await?).into_response(cx)
}

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

#[route(GET "/push/endpoints")]
async fn push_endpoints(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0.id;
    Json(hub(cx).push().endpoints(me)).into_response(cx)
}

#[route(GET "/push/reset")]
async fn push_reset(cx: &Cx) -> Result<Response> {
    let me = &request_context::<Paired>(cx).0.id;
    Json(hub(cx).push().reset(me)).into_response(cx)
}

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(push_key)
        .route(push_subscribe)
        .route(push_unsubscribe)
        .route(apns_add)
        .route(apns_remove)
        .route(activity_add)
        .route(glance)
        .route(push_endpoints)
        .route(push_reset)
        .route(push_test)
}
