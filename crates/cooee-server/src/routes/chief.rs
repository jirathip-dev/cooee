use std::sync::Arc;

use cooee_core::Config;
use cooee_machines::Machines;
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    chief::{self, Desk},
    http::{AgentAsk, Answer, Data, bearer, blocking, hub, refused},
    routines::Runner,
};

#[route(POST "/mcp/agent/chief/tools")]
async fn chief_tools(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<Vec<Value>>> {
    let token = bearer(cx);
    Ok(Answer(
        blocking(cx, move |hub| Ok(chief::list(hub, &a.thread, &token))).await,
    ))
}

#[route(POST "/mcp/agent/chief")]
async fn chief_call(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    let data = app_context::<Data>(cx);
    let desk = Desk {
        hub: hub(cx),
        runner: Arc::clone(app_context::<Arc<Runner>>(cx)),
        machines: Arc::clone(app_context::<Arc<Machines>>(cx)),
        config: Arc::clone(app_context::<Arc<Config>>(cx)),
        data: data.0.clone(),
        dir: data.1.clone().unwrap_or_default(),
    };
    let done =
        tokio::task::spawn_blocking(move || chief::call(&desk, (&a.thread, &token), &a.server));
    Ok(Answer(done.await.map_err(refused).and_then(|r| r)))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(chief_tools).route(chief_call)
}
