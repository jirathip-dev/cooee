use std::{path::PathBuf, sync::Arc};

use cooee_core::{Item, Via};
use cooee_machines::Machines;
use serde::Deserialize;
use topcoat::context::{Cx, app_context, request_context};

use crate::{
    Hub, HubError, Logins, Upload,
    http::{
        Data, Paired, Tag, allowed, blocking, hub, logins, profiles, provider_name, recipe, seated,
    },
    pairing, plugin,
};

#[derive(Deserialize)]
pub(crate) struct Start {
    pub(crate) agent: String,
    #[serde(default)]
    pub(crate) folder: PathBuf,
    pub(crate) text: String,
    pub(crate) random_id: Tag,
    #[serde(default)]
    pub(crate) account: Option<String>,
    #[serde(default)]
    pub(crate) background: bool,
    #[serde(default)]
    pub(crate) profile: Option<String>,
    #[serde(default)]
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) effort: String,
    #[serde(default)]
    pub(crate) machine: Option<String>,
    #[serde(default)]
    pub(crate) setup: bool,
}

pub(crate) async fn start(cx: &Cx, new: Start) -> Result<String, HubError> {
    let (logins, machines) = (logins(cx), Arc::clone(app_context::<Arc<Machines>>(cx)));
    let data = app_context::<Data>(cx);
    let (data, config, hub) = (data.0.clone(), data.1.clone(), hub(cx));
    let id = tokio::task::spawn_blocking(move || {
        let profile = new.profile.as_deref().unwrap_or_default();
        let list = profiles(&hub, (&data, config.as_deref()));
        let picked = (new.model, new.effort);
        let asked = new.folder.clone();
        let (agent, folder, mut recipe) = recipe(
            &logins,
            (&data, config.as_deref(), &list, &hub.mcp()?),
            (&new.agent, new.account.as_deref(), profile),
            new.folder,
            (picked, new.setup),
        )?;
        let folder = topic_folder(asked, folder);
        if let Some(m) = new.machine {
            recipe.machine = m;
        }
        allowed(&machines, &recipe.machine).map_err(HubError::Refused)?;
        let id = if new.background {
            let name = provider_name(&agent, &logins.statuses());
            hub.create_later(
                (&agent, &name),
                &folder,
                (&new.text, &new.random_id.0),
                recipe,
            )?
        } else {
            hub.create(&agent, &folder, &new.text, &new.random_id.0, &recipe)?
        };
        let ours = list.into_iter().find(|a| a.id == profile).and_then(|a| {
            let name = a.id.split_once('/')?.1.to_owned();
            Some((hub.backend(&a.lent?.plugin)?, name))
        });
        if let Some((b, name)) = ours
            && let Err(e) = plugin::started(&b, &id, &name)
        {
            cooee_core::note!("cooee: plugin {}: thread-started: {e}", b.name);
        }
        Ok(id)
    });
    id.await.map_err(|e| HubError::Refused(e.to_string()))?
}

fn topic_folder(asked: PathBuf, default: PathBuf) -> PathBuf {
    if asked.as_os_str() == "scratch" {
        PathBuf::new()
    } else if asked.as_os_str().is_empty() {
        default
    } else {
        asked
    }
}

pub(crate) struct Again {
    pub(crate) id: String,
    pub(crate) hub: Arc<Hub>,
    logins: Arc<Logins>,
    data: PathBuf,
    config: Option<PathBuf>,
}

impl Again {
    pub(crate) fn of(cx: &Cx, id: String) -> Self {
        let data = app_context::<Data>(cx);
        let (hub, logins) = (hub(cx), logins(cx));
        let (data, config) = (data.0.clone(), data.1.clone());
        Self {
            id,
            hub,
            logins,
            data,
            config,
        }
    }

    fn recipe(&self) -> Result<(crate::hub::Recipe, String), HubError> {
        let (t, _) = self.hub.thread(&self.id)?;
        let config = self.config.as_deref();
        let list = profiles(&self.hub, (&self.data, config));
        let (_, _, recipe) = recipe(
            &self.logins,
            (&self.data, config, &list, &self.hub.mcp()?),
            (&t.agent, Some(&t.account), &t.profile),
            PathBuf::new(),
            ((t.model, t.effort), t.setup),
        )?;
        Ok((recipe, provider_name(&t.agent, &self.logins.statuses())))
    }

    pub(crate) fn restart(&self) -> Result<(), HubError> {
        self.hub.restart(&self.id, &self.recipe()?.0)
    }

    pub(crate) fn retry(&self) -> Result<(), HubError> {
        let (recipe, name) = self.recipe()?;
        self.hub.retry(&self.id, recipe, &name)
    }

    pub(crate) fn send(
        &self,
        (text, random_id): (&str, &str),
        files: &[Upload],
        via: Option<Via>,
    ) -> Result<Item, HubError> {
        let send = || self.hub.send(&self.id, text, random_id, files, via);
        match send() {
            Err(HubError::Shed(e)) if e.no_such_terminal() => self.restart().and_then(|()| send()),
            done => done,
        }
    }
}

pub(crate) async fn off<T: Send + 'static>(
    f: impl FnOnce() -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let done = tokio::task::spawn_blocking(f).await;
    done.map_err(|e| HubError::Refused(e.to_string()))?
}

#[derive(Deserialize)]
pub(crate) struct Approve {
    pub(crate) item: u64,
    pub(crate) option: String,
    #[serde(default)]
    pub(crate) value: Option<String>,
    #[serde(default)]
    pub(crate) passkey: Option<pairing::Assertion>,
}

pub(crate) type Answered = Result<Result<Option<Item>, HubError>, &'static str>;

pub(crate) async fn answer(cx: &Cx, id: String, a: Approve) -> topcoat::Result<Answered> {
    if a.option == "approve" && hub(cx).is_proposal(&id, a.item) {
        let Some(p) = a.passkey else {
            return Ok(Err("Approving this needs Face ID."));
        };
        if let Err(why) = pairing::assert(cx, p).await? {
            return Ok(Err(why));
        }
    }
    let by = request_context::<Paired>(cx).0.label.clone();
    let (logins, data) = (logins(cx), app_context::<Data>(cx));
    let (data, dir) = (data.0.clone(), data.1.clone());
    Ok(Ok(blocking(cx, move |hub| {
        if let Some(seat) = hub.fallback_seat(&id, a.item, &a.option) {
            let recipe = seated(hub, &logins, (&data, dir.as_deref()), &id, &seat)?;
            return hub.fall_back((&id, a.item), &a.option, &recipe, &by);
        }
        hub.approve(&id, a.item, &a.option, a.value.as_deref(), &by)
    })
    .await))
}
