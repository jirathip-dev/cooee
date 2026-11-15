//! Folders: browsing them, the roots, a new folder, and a thread's own folder.

use std::path::PathBuf;

use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{RouterBuilder, content::Json, parse_query_params, route},
};

use crate::{
    HubError, Listing,
    http::{Answer, Data, blocking, hub, logins, profiles, recipe, thread_id},
};

/// `POST /threads/{id}/folder`: "Use a folder…" (#86).
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
    b.route(attach).route(browse).route(root).route(new_folder)
}
