use cooee_core::memory;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::http::{AgentAsk, Answer, bearer, blocking, refused};

#[route(POST "/mcp/agent/remember")]
async fn mcp_remember(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let (token, e) = (bearer(cx), serde_json::from_value(a.server));
    let e: memory::Edit = e.map_err(|e| refused(format!("remember: {e}")))?;
    Ok(Answer(
        blocking(cx, move |hub| hub.remember(&a.thread, &token, &e)).await,
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
    let whose = text("persona");
    let limit = usize::try_from(number("limit").unwrap_or(5)).unwrap_or(5);
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.history(
                (&a.thread, &token, &whose),
                (&query, limit),
                &thread,
                before,
            )
        })
        .await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_remember).route(mcp_history)
}
