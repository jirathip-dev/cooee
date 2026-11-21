use std::sync::Arc;

use cooee_core::{Config, agents, library, skills};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, path_param, route},
};

use crate::http::{Answer, Data, Name, blocking, on_personas, thread_id};

#[route(GET "/personas/{id}/skills")]
async fn skill_list(cx: &Cx) -> Result<Answer<Vec<skills::Skill>>> {
    let id = thread_id(cx);
    on_personas(cx, move |_, c| Ok(skills::list(&agents::home(c, &id)?))).await
}

#[route(POST "/personas/{id}/skills")]
async fn skill_add(cx: &Cx, Json(s): Json<skills::Skill>) -> Result<Answer<skills::Skill>> {
    let id = thread_id(cx);
    on_personas(cx, move |_, c| {
        skills::save(&agents::home(c, &id)?, s, None)
    })
    .await
}

#[route(GET "/personas/{id}/skills/{name}")]
async fn skill_get(cx: &Cx) -> Result<Answer<skills::Skill>> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |_, c| skills::read(&agents::home(c, &id)?, &name)).await
}

#[route(PUT "/personas/{id}/skills/{name}")]
async fn skill_put(cx: &Cx, Json(s): Json<skills::Skill>) -> Result<Answer<skills::Skill>> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |_, c| {
        skills::save(&agents::home(c, &id)?, s, Some(&name))
    })
    .await
}

#[route(DELETE "/personas/{id}/skills/{name}")]
async fn skill_delete(cx: &Cx) -> Result<Answer<()>> {
    let (id, name) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |_, c| {
        skills::remove(&agents::home(c, &id)?, &name)
    })
    .await
}

#[route(POST "/threads/{id}/skills/update")]
async fn skill_update(cx: &Cx) -> Result<Answer<Vec<String>>> {
    let id = thread_id(cx);
    let index = app_context::<Arc<Config>>(cx).skill_index.clone();
    let work = app_context::<Data>(cx).0.join("skills-fetch");
    let done = blocking(cx, move |hub| {
        let sources = library::index(index.as_deref()).map_err(crate::http::refused)?;
        hub.update_skills(&id, &sources, &work)
    });
    Ok(Answer(done.await))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(skill_list)
        .route(skill_add)
        .route(skill_get)
        .route(skill_put)
        .route(skill_delete)
        .route(skill_update)
}
