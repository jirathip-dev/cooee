use std::path::PathBuf;

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, parse_query_params, route},
};

use crate::{
    Listing,
    http::{Answer, blocking},
};

#[derive(Deserialize)]
struct Browse {
    path: Option<PathBuf>,
    hidden: Option<String>,
}

#[route(GET "/folders")]
async fn browse(cx: &Cx) -> Result<Answer<Listing>> {
    let q: Option<Browse> = parse_query_params(cx).ok();
    let (path, hidden) = q.map_or((None, false), |q| (q.path, q.hidden.is_some()));
    Ok(Answer(
        blocking(cx, move |hub| hub.browse(path.as_deref(), hidden)).await,
    ))
}

#[derive(Deserialize)]
struct Root {
    path: PathBuf,
    #[serde(default)]
    remove: bool,
}

#[route(POST "/roots")]
async fn root(cx: &Cx, Json(r): Json<Root>) -> Result<Answer<Vec<PathBuf>>> {
    Ok(Answer(
        blocking(cx, move |hub| hub.set_root(&r.path, !r.remove)).await,
    ))
}

#[derive(Deserialize)]
struct Folder {
    parent: PathBuf,
    name: String,
}

#[route(POST "/folders/new")]
async fn new_folder(cx: &Cx, Json(f): Json<Folder>) -> Result<Answer<PathBuf>> {
    Ok(Answer(
        blocking(cx, move |hub| hub.new_folder(&f.parent, &f.name)).await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(browse).route(root).route(new_folder)
}
