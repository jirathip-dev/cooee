use std::time::Duration;

use cooee_core::personas;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    HubError,
    http::{AgentAsk, Answer, bearer, blocking, refused},
    mcp,
};

#[route(POST "/mcp/agent/remember")]
async fn mcp_remember(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, e) = (bearer(cx), serde_json::from_value(a.server));
    let e: personas::Edit = e.map_err(|e| refused(format!("remember: {e}")))?;
    Ok(Answer(
        blocking(cx, move |hub| hub.remember(&a.thread, &token, &e)).await,
    ))
}

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

#[route(POST "/mcp/agent/grant")]
async fn mcp_grant(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<mcp::Grant>> {
    let (token, server) = (bearer(cx), a.server.as_str().unwrap_or_default().to_owned());
    Ok(Answer(
        blocking(cx, move |hub| hub.grant(&a.thread, &token, &server)).await,
    ))
}

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_grant)
        .route(mcp_propose)
        .route(mcp_ask)
        .route(mcp_remember)
        .route(mcp_history)
        .route(mcp_skill)
}
