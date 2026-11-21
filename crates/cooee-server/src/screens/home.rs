use blocks::Block;
use serde_json::Value;
use topcoat::{Result, context::Cx, router::route};

use crate::http::{Answer, blocking};

const BUNDLED: &str = include_str!("../../../../apple/Sources/CooeeUI/Default/screens/home.json");

/// `{type: screen, source: core, title, blocks, threads, personas}`: `threads` as `GET /threads`, `personas` as
/// `GET /personas` with each one's open `topics` and their summed `unread`.
#[route(GET "/screens/home")]
pub async fn home(cx: &Cx) -> Result<Answer<Block>> {
    let screen = blocking(cx, move |hub| {
        let mut screen = crate::http::bundled(BUNDLED)?;
        let threads = hub.list();
        let mut personas: Vec<_> = hub.personas().iter().map(crate::http::fields).collect();
        for p in &mut personas {
            let open = threads
                .iter()
                .filter(|t| !t.archived && p["id"] == t.persona.as_str());
            let (topics, unread) = open.fold((0, 0), |(n, u), t| (n + 1, u + t.unseen));
            p.insert("topics".to_owned(), Value::from(topics));
            p.insert("unread".to_owned(), Value::from(unread));
        }
        screen.threads = Some(threads.iter().map(crate::http::fields).collect());
        screen.personas = Some(personas);
        Ok(crate::http::core(screen))
    });
    Ok(Answer(screen.await))
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(home)
}
