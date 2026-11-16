use std::{path::PathBuf, sync::Arc};

use cooee_core::{
    Config,
    personas::{self, Persona},
};
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, route},
};

use crate::{
    HubError,
    http::{Answer, Data, hub, logins, profiles, recipe, thread_id},
    machine,
};

#[derive(Deserialize)]
struct Attach {
    folder: PathBuf,
}

#[route(POST "/threads/{id}/folder")]
async fn attach(cx: &Cx, Json(a): Json<Attach>) -> Result<Answer<cooee_core::Item>> {
    let (id, hub, logins) = (thread_id(cx), hub(cx), logins(cx));
    let data = app_context::<Data>(cx);
    let (data, config) = (data.0.clone(), data.1.clone());
    let done = tokio::task::spawn_blocking(move || {
        let (t, _) = hub.thread(&id)?;
        let list = profiles(&hub, (&data, config.as_deref()));
        let (_, _, recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list),
            (&t.agent, Some(&t.account), &t.profile),
            PathBuf::new(),
            (t.model, t.effort),
        )?;
        hub.attach(&id, &a.folder, &recipe)
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

#[derive(Deserialize)]
struct Move {
    machine: String,
}

#[route(POST "/threads/{id}/move")]
async fn move_thread(cx: &Cx, Json(m): Json<Move>) -> Result<Answer<cooee_core::Item>> {
    let (id, hub, logins) = (thread_id(cx), hub(cx), logins(cx));
    let config = Arc::clone(app_context::<Arc<Config>>(cx));
    let data = app_context::<Data>(cx);
    let (data, dir) = (data.0.clone(), data.1.clone());
    let done = tokio::task::spawn_blocking(move || {
        let (t, _) = hub.thread(&id)?;
        if !m.machine.is_empty() {
            let p = personas::find(dir.as_deref(), &data, &t.profile).unwrap_or_else(|_| Persona {
                id: id.clone(),
                agent: t.agent.clone(),
                profile: Some(t.profile.clone()).filter(|p| !p.is_empty()),
                folder: t
                    .folder
                    .as_ref()
                    .map(|f| f.display().to_string())
                    .unwrap_or_default(),
                ..Persona::default()
            });
            machine::ready(&config, &m.machine, &p).map_err(HubError::Refused)?;
        }
        let list = profiles(&hub, (&data, dir.as_deref()));
        let (_, _, recipe) = recipe(
            &logins,
            (&data, dir.as_deref(), &list),
            (&t.agent, Some(&t.account), &t.profile),
            PathBuf::new(),
            (t.model, t.effort),
        )?;
        hub.move_to(&id, &m.machine, &recipe)
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(attach).route(move_thread)
}
