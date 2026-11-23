use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    http::{AgentAsk, Answer, bearer, blocking, refused},
    hub::DeskSeen,
};

#[route(POST "/mcp/agent/desk")]
async fn mcp_desk(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<DeskSeen>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| {
            let d = serde_json::from_value(a.server).map_err(refused)?;
            hub.desk(&a.thread, &token, &d)
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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_desk).route(mcp_frame)
}
