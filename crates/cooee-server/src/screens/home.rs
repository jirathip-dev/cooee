//! `GET /screens/home` (#311): the app's bundled home, its blocks filled with the threads and personas they draw.

use blocks::Block;
use cooee_core::agents;
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::route,
};

use crate::http::{Answer, Data, blocking};

const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/home.json");

/// `{type: screen, source: core, title, blocks, threads, personas}`: `threads` as `GET /threads`, `personas` as
/// `GET /personas` with each one's open `topics` and their summed `unread`.
#[route(GET "/screens/home")]
pub async fn home(cx: &Cx) -> Result<Answer<Block>> {
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let screen = blocking(cx, move |hub| {
        let mut screen = super::bundled(BUNDLED)?;
        let threads = hub.list();
        let mut personas: Vec<_> = agents::list(config.as_deref(), &data)
            .iter()
            .map(super::fields)
            .collect();
        for p in &mut personas {
            let open = threads
                .iter()
                .filter(|t| !t.archived && p["id"] == t.persona.as_str());
            let (topics, unread) = open.fold((0, 0), |(n, u), t| (n + 1, u + t.unseen));
            p.insert("topics".to_owned(), Value::from(topics));
            p.insert("unread".to_owned(), Value::from(unread));
        }
        screen.threads = Some(threads.iter().map(super::fields).collect());
        screen.personas = Some(personas);
        Ok(super::core(screen))
    });
    Ok(Answer(screen.await))
}
