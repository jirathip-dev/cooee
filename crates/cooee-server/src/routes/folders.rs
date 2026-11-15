//! Folders: browsing them, the roots and a new folder.

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

/// `?path=<folder>&hidden=1`: Settings › Folders.
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

/// `POST /roots`: a folder to add as a root, or to remove.
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

/// `POST /folders/new`: a folder to make inside `parent`.
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
