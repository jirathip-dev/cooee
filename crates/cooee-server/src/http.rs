//! What the route areas share: the request's device or plugin, the hub and logins, the JSON answer and its errors.

use std::{
    io,
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{Device, Plugin, ShedError, personas};
use cooee_machines::Machines;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body,
        content::Json,
        path_param,
        request::headers,
        response::{IntoResponse, Response},
    },
};

use crate::{Hub, HubError, Logins, Recipe};

/// The pairing cookie.
pub(crate) const COOKIE: &str = "cooee";

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// cooee's own error: one plain sentence, marked so the page shows it (`chat.ts` words any other failure itself).
pub(crate) fn plain(status: u16, text: &str) -> Result<Response> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "text/plain; charset=utf-8")
        .header("Cooee-Error", "1")
        .body(Body::from(text.to_owned()))?)
}

pub(crate) fn header<'a>(cx: &'a Cx, name: &str) -> Option<&'a str> {
    headers(cx).get(name).and_then(|v| v.to_str().ok())
}

/// The request's paired device, put in the request context by the [`Gate`].
pub(crate) struct Paired(pub(crate) Device);

/// The plugin whose token a `/plugin/` request carries, put in the request context by the [`Gate`].
pub(crate) struct Caller(pub(crate) Plugin);

path_param!(pub(crate) id: String);

pub(crate) fn thread_id(cx: &Cx) -> String {
    match path_param::<Id>(cx) {
        Ok(id) => id.clone(),
        Err(never) => match *never {},
    }
}

pub(crate) fn hub(cx: &Cx) -> Arc<Hub> {
    Arc::clone(app_context::<Arc<Hub>>(cx))
}

/// Runs a blocking hub call off the async workers.
pub(crate) async fn blocking<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Hub) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let hub = hub(cx);
    tokio::task::spawn_blocking(move || f(&hub))
        .await
        .map_err(|e| HubError::Refused(e.to_string()))?
}

/// A JSON answer: the value, or the error's status and message.
pub(crate) struct Answer<T>(pub(crate) Result<T, HubError>);

impl<T: Serialize> IntoResponse for Answer<T> {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let (status, body) = match self.0 {
            Ok(value) => return Json(value).into_response(cx),
            Err(e @ HubError::NoThread(_)) => (404, e.to_string()),
            Err(e @ HubError::Refused(_)) => (400, e.to_string()),
            // An answer or message to a session that is gone says so (herdsman#325).
            Err(HubError::Shed(e)) if e.no_such_terminal() => (
                410,
                "Not delivered: this thread's agent is gone.".to_owned(),
            ),
            // shed's own errors go to the log; the page gets a sentence.
            Err(HubError::Shed(
                e @ (ShedError::Rpc(_) | ShedError::Bad(_) | ShedError::Cli { .. }),
            )) => {
                eprintln!("cooee: {e}");
                (
                    502,
                    "shed couldn't do that: run cooee doctor on the Mac.".to_owned(),
                )
            }
            Err(e @ HubError::Shed(_)) => (502, e.to_string()),
            Err(e @ HubError::Store(_)) => (500, e.to_string()),
        };
        plain(status, &body)
    }
}

pub(crate) fn logins(cx: &Cx) -> Arc<Logins> {
    Arc::clone(app_context::<Arc<Logins>>(cx))
}

/// `POST /logins/{id}/send`: a code from the sign-in page.
#[derive(Deserialize)]
pub(crate) struct Code {
    pub(crate) text: String,
}

/// A new thread's id.
#[derive(Serialize)]
pub(crate) struct Created {
    pub(crate) id: String,
}

/// A provider's name for ACP agent `agent` (`acme` for `acme-acp`): never the agent's id on screen.
pub(crate) fn provider_name(agent: &str, providers: &[cooee_core::Sheet]) -> String {
    providers
        .iter()
        .find(|s| s.provider.agent == agent)
        .map_or_else(
            || agent.strip_suffix("-acp").unwrap_or(agent).to_owned(),
            |s| s.provider.name.clone(),
        )
}

/// How a thread of `agent` starts in `folder`: Plain with `account`, or as agent profile `profile` (non-empty: its
/// agent, and its folder if it has one; one of `list`, not hidden), on the model and effort `picked`. Returns its
/// agent and folder too.
pub(crate) fn recipe(
    logins: &Logins,
    (data, config, list): (
        &std::path::Path,
        Option<&std::path::Path>,
        &[cooee_core::Profile],
    ),
    (agent, account, profile): (&str, Option<&str>, &str),
    folder: PathBuf,
    mut picked: (String, String),
) -> Result<(String, PathBuf, Recipe), HubError> {
    // A persona (#126): its agent, as the profile it runs as or Plain, in its folder, on its model and effort.
    let persona = personas::find(config, data, profile).ok();
    if let Some(p) = &persona {
        for (pick, default) in [(&mut picked.0, &p.model), (&mut picked.1, &p.effort)] {
            if pick.is_empty() {
                pick.clone_from(default);
            }
        }
    }
    let (agent, profile, folder) = match &persona {
        Some(p) if !["", "scratch"].contains(&p.folder.as_str()) => (
            p.agent.as_str(),
            p.profile.as_deref().unwrap_or(""),
            PathBuf::from(&p.folder),
        ),
        Some(p) => (p.agent.as_str(), p.profile.as_deref().unwrap_or(""), folder),
        None => (agent, profile, folder),
    };
    let (agent, folder, mut recipe) = if profile.is_empty() {
        let (account, launch) = logins.home(agent, account)?;
        let recipe = Recipe {
            account,
            launch,
            ..Recipe::default()
        };
        (agent.to_owned(), folder, recipe)
    } else {
        let p = list.iter().find(|p| p.id() == profile).cloned();
        let p = p.ok_or_else(|| refused(format!("no profile {profile}")))?;
        if p.hidden {
            return Err(refused(format!("profile {profile} is hidden: {}", p.why)));
        }
        let (account, launch, instructions) = logins.profile(&p, data)?;
        let recipe = Recipe {
            account,
            profile: p.id(),
            launch,
            instructions,
            mcp: p.mcp.iter().map(|k| k.server.clone()).collect(),
            ..Recipe::default()
        };
        (p.agent, p.folder.unwrap_or(folder), recipe)
    };
    recipe.launch.pick(logins.picks(&agent, &picked.1)?);
    (recipe.model, recipe.effort) = picked;
    if let Some(p) = persona {
        recipe.instructions = p.brief(data);
        recipe.profile = p.id;
        recipe.machine = p.machine;
    }
    Ok((agent, folder, recipe))
}

/// The data folder, and the config's folder (its `profiles/`).
pub(crate) struct Data(pub(crate) PathBuf, pub(crate) Option<PathBuf>);

/// The agent profiles (Plain aside), hidden ones too; a plugin's from its backend when it serves them, a `/path`
/// avatar through its proxy.
pub(crate) fn profiles(
    hub: &Hub,
    (data, config): (&std::path::Path, Option<&std::path::Path>),
) -> Vec<cooee_core::Profile> {
    let mut list = cooee_core::profiles::list(config, data, |name| hub.served(name));
    for p in &mut list {
        let via = p.plugin.as_deref().and_then(|n| hub.backend(n));
        if let (Some(via), Some(a)) = (via, &mut p.avatar)
            && a.starts_with('/')
        {
            *a = format!("/p/{}/api{a}", via.name);
        }
    }
    list
}

path_param!(pub(crate) name);
path_param!(pub(crate) rest);

pub(crate) fn refused(e: impl std::fmt::Display) -> HubError {
    HubError::Refused(e.to_string())
}

/// A thread's MCP proxy or cooee's own server asking (#56): `{thread, server}`, with the thread's token.
#[derive(Deserialize)]
pub(crate) struct AgentAsk {
    pub(crate) thread: String,
    #[serde(default)]
    pub(crate) server: serde_json::Value,
}

/// The `Authorization: Bearer` token.
pub(crate) fn bearer(cx: &Cx) -> String {
    let token = header(cx, "authorization").and_then(|a| a.strip_prefix("Bearer "));
    token.unwrap_or_default().to_owned()
}

/// A persona call (#126; docs/api.md) off the async workers, given the data and config folders.
pub(crate) async fn on_personas<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&std::path::Path, &std::path::Path) -> io::Result<T> + Send + 'static,
) -> Result<Answer<T>> {
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let done = blocking(cx, move |_| {
        let config = config.ok_or_else(|| refused("there is no config folder for personas"))?;
        f(&data, &config).map_err(refused)
    });
    Ok(Answer(done.await))
}

/// Whether `machine` may run a thread (#148): this host (empty), or one added to shed (`shed machine ls`).
pub(crate) fn allowed(machines: &Machines, machine: &str) -> std::result::Result<(), String> {
    if machine.is_empty() || machines.added().iter().any(|m| m == machine) {
        Ok(())
    } else {
        Err(format!(
            "no machine {machine}: add it with shed machine add"
        ))
    }
}
