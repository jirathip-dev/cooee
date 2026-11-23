use std::time::Duration;

use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::http::{AgentAsk, Answer, bearer, blocking};

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_ask)
}
