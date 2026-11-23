use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::http::{AgentAsk, Answer, bearer, blocking};

#[route(POST "/mcp/agent/handoff")]
async fn mcp_handoff(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    let what = a.server.get("what").and_then(serde_json::Value::as_str);
    let what = what.unwrap_or_default().to_owned();
    Ok(Answer(
        blocking(cx, move |hub| hub.handoff((&a.thread, &token), &what)).await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_handoff)
}
