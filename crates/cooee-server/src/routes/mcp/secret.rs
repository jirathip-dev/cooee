use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    HubError,
    http::{AgentAsk, Answer, bearer, blocking},
    hub::SecretAsk,
};

#[route(POST "/mcp/agent/secret")]
async fn mcp_secret(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let s: SecretAsk =
                serde_json::from_value(a.server).map_err(|e| HubError::Refused(e.to_string()))?;
            hub.secret(&a.thread, &token, s)
        })
        .await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_secret)
}
