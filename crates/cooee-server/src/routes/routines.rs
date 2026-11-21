use cooee_core::routines::{Edit, Routine};
use topcoat::{
    Result,
    context::Cx,
    router::{
        Body as Bytes, RouterBuilder,
        content::{Form, Json},
        response::Response,
        route,
    },
};

use crate::{
    http::{AgentAsk, Answer, bearer, plain, refused, thread_id},
    hub::{HubError, now},
    routines::on,
};

#[route(POST "/mcp/agent/routine")]
async fn mcp_routine(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        on(cx, move |r| {
            let ask = serde_json::from_value(a.server).map_err(refused)?;
            r.create((&a.thread, &token), ask, now())
        })
        .await,
    ))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Stop {
    id: String,
}

#[route(POST "/mcp/agent/routine/stop")]
async fn mcp_routine_stop(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        on(cx, move |r| {
            let ask: Stop = serde_json::from_value(a.server).map_err(refused)?;
            r.stop((&a.thread, &token), &ask.id, now())
        })
        .await,
    ))
}

#[route(GET "/routines.json")]
async fn routine_list(cx: &Cx) -> Result<Answer<Vec<Routine>>> {
    Ok(Answer(on(cx, |r| r.list()).await))
}

fn back<T>(done: Result<T, HubError>) -> Result<Response> {
    match done {
        Ok(_) => Ok(Response::builder().status(204).body(Bytes::from(()))?),
        Err(e) => plain(400, &e.to_string()),
    }
}

#[route(POST "/routines/{id}")]
async fn routine_edit(cx: &Cx, Form(e): Form<Edit>) -> Result<Response> {
    let id = thread_id(cx);
    back(on(cx, move |r| r.edit(&id, e, now())).await)
}

#[route(POST "/routines/{id}/delete")]
async fn routine_delete(cx: &Cx) -> Result<Response> {
    let id = thread_id(cx);
    back(on(cx, move |r| r.delete(&id)).await)
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_routine)
        .route(mcp_routine_stop)
        .route(routine_list)
        .route(routine_edit)
        .route(routine_delete)
}
