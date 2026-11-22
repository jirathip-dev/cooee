use std::{io, path::PathBuf, sync::Arc, time::UNIX_EPOCH};

use cooee_core::{
    Device, Plugin, ShedError,
    agents::{self, Agent},
};
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

use crate::{Hub, HubError, Logins, Manager, Recipe};

pub(crate) const COOKIE: &str = "cooee";

#[derive(Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct Tag(pub(crate) String);

impl TryFrom<String> for Tag {
    type Error = &'static str;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.is_empty() {
            Err("random_id is empty")
        } else {
            Ok(Self(s))
        }
    }
}

pub(crate) fn now() -> u64 {
    cooee_core::clock::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

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

pub(crate) struct Paired(pub(crate) Device);

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

pub(crate) async fn blocking<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Hub) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let hub = hub(cx);
    tokio::task::spawn_blocking(move || f(&hub))
        .await
        .map_err(|e| HubError::Refused(e.to_string()))?
}

pub(crate) struct Answer<T>(pub(crate) Result<T, HubError>);

impl<T: Serialize> IntoResponse for Answer<T> {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let (status, body) = match self.0 {
            Ok(value) => return Json(value).into_response(cx),
            Err(e @ HubError::NoThread(_)) => (404, e.to_string()),
            Err(e @ HubError::Refused(_)) => (400, e.to_string()),
            Err(e @ HubError::Stale(_)) => (409, e.to_string()),
            Err(HubError::Shed(e)) if e.no_such_terminal() => (
                410,
                "Not delivered: this thread's agent is gone.".to_owned(),
            ),
            Err(HubError::Shed(
                e @ (ShedError::Rpc(_) | ShedError::Bad(_) | ShedError::Cli { .. }),
            )) => {
                cooee_core::note!("cooee: {e}");
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

#[derive(Deserialize)]
pub(crate) struct Code {
    pub(crate) text: String,
}

#[derive(Serialize)]
pub(crate) struct Created {
    pub(crate) id: String,
}

pub(crate) fn recipe(
    logins: &Logins,
    (data, config, list, registry): (
        &std::path::Path,
        Option<&std::path::Path>,
        &[Agent],
        &[cooee_core::mcp::Server],
    ),
    (agent, account, id): (&str, Option<&str>, &str),
    folder: PathBuf,
    (mut picked, setup): ((String, String), bool),
) -> Result<(String, PathBuf, Recipe), HubError> {
    let (agent, folder, mut recipe) = if id.is_empty() {
        let (account, launch) = logins.home(agent, account);
        let recipe = Recipe {
            account,
            launch,
            ..Recipe::default()
        };
        (agent.to_owned(), folder, recipe)
    } else {
        let a = list.iter().find(|a| a.id == id);
        let a = a.ok_or_else(|| refused(format!("no profile {id}")))?;
        if let Some(why) = a.lent.as_ref().and_then(|l| l.hidden.as_ref()) {
            return Err(refused(format!("profile {id} is hidden: {why}")));
        }
        for (pick, default) in [(&mut picked.0, &a.model), (&mut picked.1, &a.effort)] {
            if pick.is_empty() {
                pick.clone_from(default);
            }
        }
        let recipe = Recipe {
            mcp: a.mcp.iter().map(|k| k.server.clone()).collect(),
            machine: a.machine.clone(),
            ..crate::hub::session::persona(logins, a, account, (data, config, registry))?
        };
        (a.agent.clone(), a.work_folder().unwrap_or(folder), recipe)
    };
    recipe.launch.pick(logins.picks(&agent, &picked.1)?);
    (recipe.model, recipe.effort) = picked;
    if setup {
        if id.is_empty() || list.iter().any(|a| a.id == id && a.lent.is_some()) {
            return Err(refused("a setup chat needs one of your own personas"));
        }
        if !recipe.instructions.is_empty() {
            recipe.instructions.push_str(
                "

",
            );
        }
        recipe.instructions.push_str(agents::SETUP);
        recipe.setup = true;
    }
    Ok((agent, folder, recipe))
}

pub(crate) fn seated(
    hub: &Hub,
    logins: &Logins,
    (data, config): (&std::path::Path, Option<&std::path::Path>),
    id: &str,
    seat: &agents::Seat,
) -> Result<Recipe, HubError> {
    let (t, _) = hub.thread(id)?;
    let mut list = profiles(hub, (data, config));
    for p in list.iter_mut().filter(|p| p.id == t.profile) {
        seat.agent.clone_into(&mut p.agent);
        (p.account, p.model, p.effort) = (seat.account.clone(), String::new(), String::new());
    }
    let (_, _, recipe) = recipe(
        logins,
        (data, config, &list, &hub.mcp()?),
        (&seat.agent, seat.account.as_deref(), &t.profile),
        PathBuf::new(),
        (Default::default(), t.setup),
    )?;
    Ok(recipe)
}

pub(crate) struct Data(pub(crate) PathBuf, pub(crate) Option<PathBuf>);

pub(crate) fn profiles(
    hub: &Hub,
    (data, config): (&std::path::Path, Option<&std::path::Path>),
) -> Vec<Agent> {
    let mut list = agents::all(config, data, |name| hub.served(name));
    for p in &mut list {
        let via = p.lent.as_ref().and_then(|l| hub.backend(&l.plugin));
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

#[derive(Deserialize)]
pub(crate) struct AgentAsk {
    pub(crate) thread: String,
    #[serde(default)]
    pub(crate) server: serde_json::Value,
}

pub(crate) fn bearer(cx: &Cx) -> String {
    let token = header(cx, "authorization").and_then(|a| a.strip_prefix("Bearer "));
    token.unwrap_or_default().to_owned()
}

pub(crate) async fn on_personas<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&std::path::Path, &std::path::Path) -> io::Result<T> + Send + 'static,
) -> Result<Answer<T>> {
    let d = app_context::<Data>(cx);
    let (data, config) = (d.0.clone(), d.1.clone());
    let done = blocking(cx, move |_| {
        let config = config.ok_or_else(|| refused("there is no config folder for agents"))?;
        f(&data, &config).map_err(|e| match e.get_ref() {
            Some(s) if s.is::<cooee_core::disk::Stale>() => HubError::Stale(e.to_string()),
            _ => refused(e),
        })
    });
    Ok(Answer(done.await))
}

pub(crate) fn allowed(machines: &Machines, machine: &str) -> std::result::Result<(), String> {
    if machine.is_empty() || machines.added().iter().any(|m| m == machine) {
        Ok(())
    } else {
        Err(format!(
            "no machine {machine}: add it with shed machine add"
        ))
    }
}

pub(crate) fn manager(cx: &Cx) -> Arc<Manager> {
    Arc::clone(app_context::<Arc<Manager>>(cx))
}

pub(crate) async fn on_logins<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Arc<Logins>) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let logins = logins(cx);
    tokio::task::spawn_blocking(move || f(&logins))
        .await
        .map_err(|e| HubError::Refused(e.to_string()))?
}

pub(crate) async fn persona_save(cx: &Cx, p: Agent) -> Result<Answer<Agent>> {
    let installed = hub(cx).installed();
    let machines = Arc::clone(app_context::<Arc<Machines>>(cx));
    on_personas(cx, move |d, c| {
        let why = match p.plugins.iter().find(|n| !installed.contains(n)) {
            Some(n) => Err(format!("no installed plugin {n}")),
            None => allowed(&machines, &p.machine),
        };
        why.map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        agents::save(c, d, p)
    })
    .await
}

pub(crate) fn core(screen: blocks::Screen) -> blocks::Block {
    blocks::Block::Screen(Box::new(screen.source("core")))
}

pub(crate) fn bundled(text: &str) -> Result<blocks::Screen, HubError> {
    match serde_json::from_str(text).map_err(refused)? {
        blocks::Block::Screen(screen) => Ok(*screen),
        _ => Err(refused("a bundled screen is no screen")),
    }
}

pub(crate) fn fields(value: impl serde::Serialize) -> blocks::Fields {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::Object(fields)) => fields,
        _ => blocks::Fields::new(),
    }
}

pub(crate) fn image(name: &std::path::Path) -> Option<&'static str> {
    let ext = name
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "heic" => Some("image/heic"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}
