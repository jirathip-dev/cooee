//! Personas (#126), their memory, and what a persona picks from: the machines, agents and models.

use std::{collections::BTreeMap, io, sync::Arc};

use cooee_core::personas::{self, Persona};
use cooee_machines::Machines;
use serde::Serialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, path_param, route},
};

use crate::http::{
    Answer, Code, Name, allowed, blocking, hub, logins, now, on_personas, refused, thread_id,
};

/// `GET /models` (#193): each agent and account's models as last seen, each short name with what it resolves to now.
#[derive(Serialize)]
struct ModelList {
    agent: String,
    account: String,
    seen: u64,
    models: Vec<[String; 2]>,
    short: BTreeMap<String, String>,
}

/// `GET /agents` (#230): each installed agent with the efforts its provider offers.
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

/// Saves `p`, its plugins (#129) each an installed one and its machine (#148) an allowed one.
async fn persona_save(cx: &Cx, p: Persona) -> Result<Answer<Persona>> {
    let installed = hub(cx).installed();
    let machines = Arc::clone(app_context::<Arc<Machines>>(cx));
    on_personas(cx, move |d, c| {
        let why = match p.plugins.iter().find(|n| !installed.contains(n)) {
            Some(n) => Err(format!("no installed plugin {n}")),
            None => allowed(&machines, &p.machine),
        };
        why.map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        personas::save(c, d, p)
    })
    .await
}

/// One machine a thread may run on (#148): its name (empty: this host, `host` its host name), whether shed reaches
/// it, when it last answered and its health now (the machines plugin's card).
#[derive(Serialize)]
struct MachineRow {
    name: String,
    host: String,
    online: bool,
    last_seen: Option<u64>,
    health: Option<cooee_core::shed_proto::Sample>,
    error: Option<String>,
}

/// `GET /machines`: the allowed machines, this host first.
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

/// `GET /personas/{id}/places`: its header strip, its attached plugins' `header-strip` chips (#125, q984).
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
