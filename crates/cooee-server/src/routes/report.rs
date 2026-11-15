//! A problem reported from the page.

use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{RouterBuilder, content::Json, response::Response, route},
};

use crate::http::{Data, Paired, now, plain};

/// `POST /report`: a problem reported from the page (report mode): the note and the element it is about.
#[derive(Deserialize, Serialize)]
struct Report {
    note: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    html: String,
    #[serde(default)]
    look: String,
    #[serde(default)]
    viewport: String,
}

/// Appends the report to `<data>/reports.jsonl`, with when and which device; `cooee reports` lists them.
#[route(POST "/report")]
async fn report(cx: &Cx, Json(r): Json<Report>) -> Result<Response> {
    if r.note.trim().is_empty() {
        return plain(400, "A report needs a note.");
    }
    let cut = |s: &str, n| s.chars().take(n).collect::<String>();
    let line = serde_json::json!({
        "at": now(), "device": request_context::<Paired>(cx).0.id, "note": cut(&r.note, 2000),
        "path": cut(&r.path, 300), "target": cut(&r.target, 300), "html": cut(&r.html, 600),
        "look": cut(&r.look, 200), "viewport": cut(&r.viewport, 40),
    });
    let file = app_context::<Data>(cx).0.join("reports.jsonl");
    tokio::task::spawn_blocking(move || {
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file)?;
        writeln!(f, "{line}")
    })
    .await??;
    plain(200, "report sent")
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(report)
}
