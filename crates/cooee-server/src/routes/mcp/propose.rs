use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    HubError,
    http::{AgentAsk, Answer, bearer, blocking},
};

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
    b.route(mcp_propose)
}
