//! Routines' API (#75): `create_routine`'s line, the list, an edit, a delete.

use std::sync::Arc;

use cooee_core::{
    Item,
    routines::{Edit, Routine},
};
use topcoat::{
    Result,
    context::{Cx, app_context},
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
    routines::Runner,
};

fn runner(cx: &Cx) -> Arc<Runner> {
    Arc::clone(app_context::<Arc<Runner>>(cx))
}

/// Runs `f` on the runner off the async workers.
async fn on<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Runner) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let r = runner(cx);
    tokio::task::spawn_blocking(move || f(&r))
        .await
        .map_err(refused)?
}

/// `POST /mcp/agent/routine` `{thread, server: {name, when, prompt, place?}}`: `create_routine`'s line.
#[route(POST "/mcp/agent/routine")]
async fn mcp_routine(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<Item>> {
    let token = bearer(cx);
    Ok(Answer(
        on(cx, move |r| {
            let ask = serde_json::from_value(a.server).map_err(refused)?;
            r.create((&a.thread, &token), ask, now())
        })
        .await,
    ))
}

/// `GET /routines.json`: every routine.
#[route(GET "/routines.json")]
async fn routine_list(cx: &Cx) -> Result<Answer<Vec<Routine>>> {
    Ok(Answer(on(cx, Runner::list).await))
}

/// A 204, or the reason.
fn back<T>(done: Result<T, HubError>) -> Result<Response> {
    match done {
        Ok(_) => Ok(Response::builder().status(204).body(Bytes::from(()))?),
        Err(e) => plain(400, &e.to_string()),
    }
}

/// `POST /routines/{id}` `name`, `when`, `prompt`, `paused` (each optional): its edit.
#[route(POST "/routines/{id}")]
async fn routine_edit(cx: &Cx, Form(e): Form<Edit>) -> Result<Response> {
    let id = thread_id(cx);
    back(on(cx, move |r| r.edit(&id, e, now())).await)
}

/// `POST /routines/{id}/delete`.
#[route(POST "/routines/{id}/delete")]
async fn routine_delete(cx: &Cx) -> Result<Response> {
    let id = thread_id(cx);
    back(on(cx, move |r| r.delete(&id)).await)
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(mcp_routine)
        .route(routine_list)
        .route(routine_edit)
        .route(routine_delete)
}
