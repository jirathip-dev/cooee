use cooee_core::agents;
use serde_json::{Value, json};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, route},
};

use crate::http::{Answer, Data, blocking, core, refused};

const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/home.json");

/// `{type: screen, source: core, title, blocks, threads, personas}`: `threads` as `GET /threads`, `personas` as
/// `GET /personas` with each one's open `topics` and their summed `unread`.
#[route(GET "/screens/home")]
pub async fn home(cx: &Cx) -> Result<Answer<Value>> {
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let screen = blocking(cx, move |hub| {
        let mut screen: Value = serde_json::from_str(BUNDLED).map_err(refused)?;
        let threads = hub.list();
        let mut personas = json!(agents::list(config.as_deref(), &data));
        for p in personas.as_array_mut().into_iter().flatten() {
            let open = threads
                .iter()
                .filter(|t| !t.archived && p["id"] == t.persona.as_str());
            let (topics, unread) = open.fold((0, 0), |(n, u), t| (n + 1, u + t.unseen));
            if let Some(p) = p.as_object_mut() {
                p.insert("topics".to_owned(), json!(topics));
                p.insert("unread".to_owned(), json!(unread));
            }
        }
        if let Some(o) = screen.as_object_mut() {
            o.insert("threads".to_owned(), json!(threads));
            o.insert("personas".to_owned(), personas);
        }
        Ok(core(screen))
    });
    Ok(Answer(screen.await))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(home)
}
