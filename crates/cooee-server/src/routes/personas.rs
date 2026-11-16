use std::{collections::BTreeMap, sync::Arc};

use cooee_core::personas::{self, Persona};
use cooee_machines::Machines;
use serde::Serialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, path_param, route},
};

use crate::http::{
    Answer, Code, Name, blocking, hub, logins, now, on_personas, persona_save, refused, thread_id,
};

#[derive(Serialize)]
struct ModelList {
    agent: String,
    account: String,
    seen: u64,
    models: Vec<[String; 2]>,
    short: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct AgentRow {
    id: String,
    efforts: Vec<String>,
}

#[route(GET "/agents")]
async fn agent_list(cx: &Cx) -> Result<Answer<Vec<AgentRow>>> {
    let logins = logins(cx);
    let rows = hub(cx).agents().map_err(refused).map(|agents| {
        let rows = agents.into_iter().map(|a| AgentRow {
            efforts: logins.efforts(&a.id),
            id: a.id,
        });
        rows.collect()
    });
    Ok(Answer(rows))
}

#[route(GET "/models")]
async fn model_list(cx: &Cx) -> Result<Answer<Vec<ModelList>>> {
    let lists = hub(cx).models().into_iter().map(|o| ModelList {
        short: o.short_names().into_iter().collect(),
        agent: o.agent,
        account: o.account,
        seen: o.seen,
        models: o.models,
    });
    Ok(Answer(Ok(lists.collect())))
}

#[route(GET "/personas")]
async fn persona_list(cx: &Cx) -> Result<Answer<Vec<Persona>>> {
    on_personas(cx, |d, c| Ok(personas::list(Some(c), d))).await
}

#[route(POST "/personas")]
async fn persona_new(cx: &Cx, Json(p): Json<Persona>) -> Result<Answer<Persona>> {
    let p = Persona {
        id: String::new(),
        ..p
    };
    persona_save(cx, p).await
}

#[derive(Serialize)]
struct MachineRow {
    name: String,
    host: String,
    online: bool,
    last_seen: Option<u64>,
    health: Option<cooee_core::shed_proto::Sample>,
    error: Option<String>,
}

#[route(GET "/machines")]
async fn machine_list(cx: &Cx) -> Result<Answer<Vec<MachineRow>>> {
    let machines = Arc::clone(app_context::<Arc<Machines>>(cx));
    Ok(Answer(
        blocking(cx, move |_| {
            let cards = machines.poll(cooee_machines::unix_now());
            Ok(cards
                .into_iter()
                .enumerate()
                .map(|(i, c)| MachineRow {
                    name: if i == 0 {
                        String::new()
                    } else {
                        c.name.clone()
                    },
                    host: c.name,
                    online: c.online,
                    last_seen: c.last_seen,
                    health: c.now,
                    error: c.error,
                })
                .collect())
        })
        .await,
    ))
}

#[route(GET "/personas/{id}")]
async fn persona_get(cx: &Cx) -> Result<Answer<Persona>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| personas::find(Some(c), d, &id)).await
}

#[route(PUT "/personas/{id}")]
async fn persona_put(cx: &Cx, Json(p): Json<Persona>) -> Result<Answer<Persona>> {
    let p = Persona {
        id: thread_id(cx),
        ..p
    };
    persona_save(cx, p).await
}

#[route(GET "/personas/{id}/places")]
async fn places(cx: &Cx) -> Result<Answer<Vec<personas::Place>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| personas::places(Some(c), d, &id)).await
}

#[route(DELETE "/personas/{id}")]
async fn persona_delete(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| personas::remove(c, d, &id)).await
}

#[route(GET "/personas/{id}/memory")]
async fn memory(cx: &Cx) -> Result<Answer<Vec<personas::Note>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        Ok(personas::notes(d, &personas::find(Some(c), d, &id)?.id))
    })
    .await
}

async fn owner_edit(cx: &Cx, e: personas::Edit) -> Result<Answer<Option<personas::Note>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        let id = personas::find(Some(c), d, &id)?.id;
        personas::remember(d, &id, "owner", &e, now())
    })
    .await
}

#[route(POST "/personas/{id}/memory")]
async fn memory_add(cx: &Cx, Json(n): Json<Code>) -> Result<Answer<Option<personas::Note>>> {
    let e = personas::Edit {
        text: Some(n.text),
        ..personas::Edit::default()
    };
    owner_edit(cx, e).await
}

#[route(PUT "/personas/{id}/memory/{name}")]
async fn memory_put(cx: &Cx, Json(n): Json<Code>) -> Result<Answer<Option<personas::Note>>> {
    let e = personas::Edit {
        text: Some(n.text),
        replaces: Some(path_param::<Name>(cx).to_owned()),
        ..personas::Edit::default()
    };
    owner_edit(cx, e).await
}

#[route(DELETE "/personas/{id}/memory/{name}")]
async fn memory_drop(cx: &Cx) -> Result<Answer<Option<personas::Note>>> {
    let e = personas::Edit {
        forget: vec![path_param::<Name>(cx).to_owned()],
        ..personas::Edit::default()
    };
    owner_edit(cx, e).await
}

#[route(DELETE "/personas/{id}/memory")]
async fn memory_clear(cx: &Cx) -> Result<Answer<()>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        personas::forget(d, &personas::find(Some(c), d, &id)?.id)
    })
    .await
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(persona_list)
        .route(machine_list)
        .route(model_list)
        .route(agent_list)
        .route(persona_new)
        .route(persona_get)
        .route(persona_put)
        .route(persona_delete)
        .route(places)
        .route(memory)
        .route(memory_add)
        .route(memory_put)
        .route(memory_drop)
        .route(memory_clear)
}
