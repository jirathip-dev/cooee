use cooee_core::{agents, skills};
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, path_param, route},
};

use crate::http::{Answer, Name, on_personas, thread_id};

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

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(skill_list)
        .route(skill_add)
        .route(skill_get)
        .route(skill_put)
        .route(skill_delete)
}
