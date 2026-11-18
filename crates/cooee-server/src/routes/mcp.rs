use std::time::Duration;

use cooee_core::memory;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    HubError,
    http::{AgentAsk, Answer, bearer, blocking, refused},
    hub::Called,
    mcp,
};

#[route(POST "/mcp/agent/remember")]
async fn mcp_remember(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, e) = (bearer(cx), serde_json::from_value(a.server));
    let e: memory::Edit = e.map_err(|e| refused(format!("remember: {e}")))?;
    Ok(Answer(
        blocking(cx, move |hub| hub.remember(&a.thread, &token, &e)).await,
    ))
}

#[route(POST "/mcp/agent/persona")]
async fn mcp_persona(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let u = serde_json::from_value(a.server).map_err(refused)?;
            hub.update_persona(&a.thread, &token, u)
        })
        .await,
    ))
}

#[route(POST "/mcp/agent/skill")]
async fn mcp_skill(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let me = (a.thread.as_str(), token.as_str());
            if let Some(name) = a.server.get("read").and_then(serde_json::Value::as_str) {
                return hub.read_skill(me, name);
            }
            let p = serde_json::from_value(a.server.get("propose").cloned().unwrap_or_default())
                .map_err(|_| HubError::Refused("give read or propose".to_owned()))?;
            hub.propose_skill(me, p).map(|_| {
                "Proposed. The owner sees a card; nothing is saved until they approve it."
                    .to_owned()
            })
        })
        .await,
    ))
}

#[route(POST "/mcp/agent/grant")]
async fn mcp_grant(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<mcp::Grant>> {
    let (token, server) = (bearer(cx), a.server.as_str().unwrap_or_default().to_owned());
    Ok(Answer(
        blocking(cx, move |hub| hub.grant(&a.thread, &token, &server)).await,
    ))
}

#[route(POST "/mcp/agent/call")]
async fn mcp_call(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<Called>> {
    let token = bearer(cx);
    let text = |k: &str| {
        a.server
            .get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let (server, tool) = (text("server"), text("tool"));
    let args = a.server.get("arguments").cloned().unwrap_or_default();
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.call((&a.thread, &token), (&server, &tool), &args)
        })
        .await,
    ))
}

#[route(POST "/mcp/agent/frame")]
async fn mcp_frame(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<()>> {
    let token = bearer(cx);
    let text = |k: &str| {
        a.server
            .get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let (caption, image) = (text("caption"), text("image"));
    let asked = a.server.get("asked").and_then(serde_json::Value::as_bool) == Some(true);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.keep_frame((&a.thread, &token), &caption, &image, asked)
        })
        .await,
    ))
}

#[route(POST "/mcp/agent/ask")]
async fn mcp_ask(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, s) = (bearer(cx), a.server);
    let text = |k: &str| {
        s.get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let (persona, message) = (text("persona"), text("message"));
    let number = |k: &str| s.get(k).and_then(serde_json::Value::as_u64);
    let wait = Duration::from_secs(number("timeout").unwrap_or(300).min(600));
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.ask_persona((&a.thread, &token), &persona, &message, wait)
        })
        .await,
    ))
}

#[route(POST "/mcp/agent/handoff")]
async fn mcp_handoff(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    let what = a.server.get("what").and_then(serde_json::Value::as_str);
    let what = what.unwrap_or_default().to_owned();
    Ok(Answer(
        blocking(cx, move |hub| hub.handoff((&a.thread, &token), &what)).await,
    ))
}

#[route(POST "/mcp/agent/history")]
async fn mcp_history(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, s) = (bearer(cx), a.server);
    let text = |k: &str| {
        s.get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let number = |k: &str| s.get(k).and_then(serde_json::Value::as_u64);
    let (query, thread, before) = (text("query"), text("thread"), number("before"));
    let limit = usize::try_from(number("limit").unwrap_or(5)).unwrap_or(5);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.history((&a.thread, &token), (&query, limit), &thread, before)
        })
        .await,
    ))
}

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

#[route(POST "/mcp/agent/secret")]
async fn mcp_secret(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let s =
                serde_json::from_value(a.server).map_err(|e| HubError::Refused(e.to_string()))?;
            hub.secret(&a.thread, &token, s)
        })
        .await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_grant)
        .route(mcp_secret)
        .route(mcp_call)
        .route(mcp_frame)
        .route(mcp_propose)
        .route(mcp_ask)
        .route(mcp_handoff)
        .route(mcp_remember)
        .route(mcp_persona)
        .route(mcp_history)
        .route(mcp_skill)
}
