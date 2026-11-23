use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    HubError,
    http::{AgentAsk, Answer, bearer, blocking, refused},
};

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_persona).route(mcp_skill)
}
