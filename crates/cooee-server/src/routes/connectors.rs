use std::sync::Arc;

use cooee_core::mcp::Server;
use serde::Deserialize;
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Body, RouterBuilder, content::Json, parse_query_params, response::Response, route},
};

use crate::{
    Hub, HubError,
    connectors::Connectors,
    http::{Answer, Data, bearer, header, hub, now, plain, profiles, thread_id},
};

async fn on<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Hub, &Connectors) -> Result<T, String> + Send + 'static,
) -> Answer<T> {
    let (hub, c) = (hub(cx), Arc::clone(app_context::<Arc<Connectors>>(cx)));
    let done = tokio::task::spawn_blocking(move || f(&hub, &c)).await;
    Answer(
        done.map_err(|e| e.to_string())
            .and_then(|r| r)
            .map_err(HubError::Refused),
    )
}

fn server(hub: &Hub, name: &str) -> Result<Server, String> {
    let list = hub.mcp().map_err(|e| e.to_string())?;
    list.into_iter()
        .find(|s| s.name == name)
        .ok_or_else(|| format!("there is no connector {name}"))
}

#[route(GET "/connectors")]
async fn connectors_list(cx: &Cx) -> Result<Answer<Value>> {
    let Data(data, config) = app_context::<Data>(cx);
    let (data, config) = (data.clone(), config.clone());
    Ok(on(cx, move |hub, c| {
        let personas = profiles(hub, (&data, config.as_deref()));
        c.list(&hub.mcp().map_err(|e| e.to_string())?, &personas)
    })
    .await)
}

#[derive(Deserialize)]
struct Add {
    catalogue: String,
}

#[route(POST "/connectors")]
async fn connectors_add(cx: &Cx, Json(a): Json<Add>) -> Result<Answer<Value>> {
    Ok(on(cx, move |hub, _| {
        let name = a.catalogue;
        cooee_core::connectors::entry(&name)
            .ok_or_else(|| format!("the catalogue has no {name}"))?;
        hub.edit_mcp(|list| match list.iter().find(|s| s.name == name) {
            Some(s) if s.catalogue => Ok(String::new()),
            Some(_) => Err(format!("there is a server {name} already")),
            None => {
                list.push(Server {
                    name: name.clone(),
                    kind: "stdio".to_owned(),
                    enabled: true,
                    catalogue: true,
                    ..Server::default()
                });
                Ok(String::new())
            }
        })
        .map_err(|e| e.to_string())?;
        Ok(json!({ "name": name, "reviewed": true }))
    })
    .await)
}

#[route(DELETE "/connectors/{id}")]
async fn connectors_remove(cx: &Cx) -> Result<Answer<String>> {
    let name = thread_id(cx);
    Ok(on(cx, move |hub, c| {
        server(hub, &name)?;
        c.forget(&name)?;
        hub.edit_mcp(|list| {
            list.retain(|s| s.name != name);
            Ok(format!("Removed {name}; its accounts are signed out."))
        })
        .map_err(|e| e.to_string())
    })
    .await)
}

#[derive(Deserialize)]
struct SignIn {
    account: String,
    origin: String,
}

#[route(POST "/connectors/{id}/signin")]
async fn connectors_signin(cx: &Cx, Json(b): Json<SignIn>) -> Result<Answer<Value>> {
    let (name, host) = (
        thread_id(cx),
        header(cx, "host").unwrap_or_default().to_owned(),
    );
    Ok(on(cx, move |hub, c| {
        c.signin(&server(hub, &name)?, &b.account, (&b.origin, &host), now())
    })
    .await)
}

#[derive(Deserialize)]
struct Paste {
    flow: String,
    value: String,
}

#[route(POST "/connectors/{id}/token")]
async fn connectors_token(cx: &Cx, Json(b): Json<Paste>) -> Result<Answer<String>> {
    let name = thread_id(cx);
    Ok(on(cx, move |_, c| c.paste(&name, &b.flow, &b.value, now())).await)
}

#[derive(Deserialize)]
struct OwnerClient {
    id: String,
    secret: Option<String>,
}

#[route(POST "/connectors/{id}/client")]
async fn connectors_client(cx: &Cx, Json(b): Json<OwnerClient>) -> Result<Answer<String>> {
    let name = thread_id(cx);
    Ok(on(cx, move |hub, c| {
        c.add_client(&server(hub, &name)?, &b.id, b.secret.as_deref())
    })
    .await)
}

#[route(GET "/connectors/flows/{id}")]
async fn connectors_flow(cx: &Cx) -> Result<Response> {
    let id = thread_id(cx);
    match app_context::<Arc<Connectors>>(cx).flow(&id, now()) {
        Some(v) => Ok(Response::builder()
            .header("Content-Type", "application/json")
            .body(Body::from(v.to_string()))?),
        None => plain(404, "no such sign-in"),
    }
}

#[derive(Deserialize)]
struct Who {
    account: String,
}

#[route(POST "/connectors/{id}/signout")]
async fn connectors_signout(cx: &Cx, Json(b): Json<Who>) -> Result<Answer<String>> {
    let name = thread_id(cx);
    Ok(on(cx, move |hub, c| {
        server(hub, &name)?;
        c.signout(&name, &b.account)
    })
    .await)
}

#[derive(Default, Deserialize)]
struct Back {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

#[route(GET "/connectors/callback")]
async fn connectors_callback(cx: &Cx) -> Result<Response> {
    let q: Back = parse_query_params(cx).unwrap_or_default();
    let c = Arc::clone(app_context::<Arc<Connectors>>(cx));
    let ended = tokio::task::spawn_blocking(move || {
        let outcome = match (&q.code, &q.error) {
            (_, Some(e)) => Err(e.as_str()),
            (Some(code), None) => Ok(code.as_str()),
            (None, None) => return None,
        };
        c.callback(q.state.as_deref()?, outcome, now())
    })
    .await
    .ok()
    .flatten();
    match ended {
        Some(id) => Ok(Response::builder()
            .status(302)
            .header("Location", format!("cooee://connector?flow={id}"))
            .body(Body::from(()))?),
        None => plain(
            400,
            "This sign-in is unknown, used or over 10 minutes old: start it again from the app.",
        ),
    }
}

#[derive(Deserialize)]
struct Piped {
    thread: String,
    server: String,
    message: Value,
}

#[route(POST "/mcp/agent/rpc")]
async fn mcp_rpc(cx: &Cx, Json(p): Json<Piped>) -> Result<Response> {
    let token = bearer(cx);
    let got = on(cx, move |hub, c| {
        c.rpc(hub, (&p.thread, &token), &p.server, &p.message, now())
    })
    .await;
    match got.0 {
        Ok(all) => Ok(Response::builder()
            .header("Content-Type", "application/x-ndjson")
            .body(Body::from(
                all.iter().map(|m| m.to_string() + "\n").collect::<String>(),
            ))?),
        Err(e) => plain(400, &e.to_string()),
    }
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(connectors_list)
        .route(connectors_add)
        .route(connectors_remove)
        .route(connectors_signin)
        .route(connectors_token)
        .route(connectors_client)
        .route(connectors_flow)
        .route(connectors_signout)
        .route(connectors_callback)
        .route(mcp_rpc)
}
