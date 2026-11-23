use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::http::{AgentAsk, Answer, bearer, blocking, refused};

#[route(POST "/mcp/agent/mail")]
async fn mcp_mail(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let m = serde_json::from_value(a.server).map_err(refused)?;
            hub.mail(&a.thread, &token, m)
        })
        .await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_mail)
}
