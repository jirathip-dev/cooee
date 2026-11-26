use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    http::{AgentAsk, Answer, bearer, blocking},
    hub::Called,
    mcp,
};

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_grant).route(mcp_call)
}
