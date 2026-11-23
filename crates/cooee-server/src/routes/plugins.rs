use std::{io, sync::Arc, time::Duration};

use cooee_core::Plugin;
use futures_util::StreamExt;
use serde::Deserialize;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{
        Body, RouterBuilder,
        content::{
            Json,
            sse::{Event, KeepAlive, Sse},
        },
        error::not_found,
        parse_query_params, path_param,
        request::{method, uri},
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    CardSpec, Hub, HubError, ISSUES, Logins, Manager, Note, Plan, Recipe,
    http::{
        Answer, Caller, Data, Name, Paired, blocking, header, hub, logins, manager, now, plain,
        refused, thread_id,
    },
    passkey::{self, Unlocks},
    plugin,
};

#[derive(Default)]
struct Dots(std::sync::Mutex<std::collections::BTreeSet<String>>);

fn named(cx: &Cx) -> Result<Plugin> {
    let name = path_param::<Name>(cx);
    let hub = app_context::<Arc<Hub>>(cx);
    let found = (hub.plugins().into_iter()).find(|p| p.name == name && !p.module.is_empty());
    Ok(found.ok_or_else(not_found)?)
}

fn relay(e: plugin::Sse) -> Event {
    let event = if e.event.is_empty() {
        "message"
    } else {
        &e.event
    };
    let out = Event::new().data(serde_json::json!({ "event": event, "data": e.data }).to_string());
    match e.id {
        Some(id) => out.id(id),
        None => out,
    }
}

#[route([GET, POST] "/p/{name}/api/{*rest}")]
async fn proxy(cx: &Cx, body: String) -> Result<Response> {
    let p = named(cx)?;
    let prefix = format!("/p/{}/api", p.name);
    let uri = uri(cx);
    let target = uri.path_and_query().map_or("/", |t| t.as_str());
    let target = target.strip_prefix(&prefix).unwrap_or(target).to_owned();
    let method = method(cx).to_string();
    let mut pass: Vec<(&str, String)> = ["content-type", "accept", "last-event-id"]
        .into_iter()
        .filter_map(|h| Some((h, header(cx, h)?.to_owned())))
        .collect();
    let me = request_context::<Paired>(cx).0.id.clone();
    let unlocks = Arc::clone(app_context::<Arc<Unlocks>>(cx));
    let gated = p.name == passkey::GATED;
    if gated {
        if !unlocks.touch(&me, now()) {
            return Ok(Response::builder()
                .status(403)
                .header("Cooee-Locked", "1")
                .body(Body::from("locked"))?);
        }
        pass.push(("cooee-device", me.clone()));
    }
    let reply = tokio::task::spawn_blocking(move || {
        let headers: Vec<(&str, &str)> = pass.iter().map(|(k, v)| (*k, v.as_str())).collect();
        let deadline = Duration::from_secs(30);
        plugin::call(&p, &method, &target, &headers, body.as_bytes(), deadline)
    })
    .await?;
    let reply = match reply {
        Ok(r) => r,
        Err(e) => return plain(502, &format!("{prefix}: {e}")),
    };
    let open = move || !gated || unlocks.open(&me, now());
    pass_on(cx, reply, relay, open).await
}

async fn pass_on(
    cx: &Cx,
    reply: plugin::Reply,
    relay: fn(plugin::Sse) -> Event,
    open: impl Fn() -> bool + Send + 'static,
) -> Result<Response> {
    if reply.streams() {
        let stream = plugin::events(reply)?
            .take_while(move |_| std::future::ready(open()))
            .map(move |heard| match heard {
                plugin::Heard::Event(e) => Result::<Event>::Ok(relay(e)),
                plugin::Heard::Beat => Ok(Event::new().comment("")),
            });
        return Sse::new(stream)
            .keep_alive(KeepAlive::new())
            .into_response(cx);
    }
    let status = reply.status;
    let kind = reply
        .header("content-type")
        .unwrap_or("application/octet-stream")
        .to_owned();
    let bytes = tokio::task::spawn_blocking(move || reply.bytes()).await??;
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", kind)
        .header("Cache-Control", "no-store")
        .body(Body::from(bytes))?)
}

fn with_screens(cx: &Cx) -> Option<(Plugin, cooee_core::plugins::Manifest)> {
    let name = path_param::<Name>(cx);
    let data = &app_context::<Data>(cx).0;
    let installed = cooee_core::plugins::installed(data).ok()?;
    let (_, m) = installed.iter().find(|(l, _)| l.name == name)?;
    let m = m.clone();
    let p = app_context::<Arc<Hub>>(cx).backend(name)?;
    (!m.places.is_empty()).then_some((p, m))
}

fn ops(e: plugin::Sse) -> Event {
    let out = Event::new().data(e.data);
    let out = if e.event.is_empty() {
        out
    } else {
        out.event(e.event)
    };
    match e.id {
        Some(id) => out.id(id),
        None => out,
    }
}

#[route(GET "/p/{name}/screens/{*rest}")]
async fn screen(cx: &Cx) -> Result<Response> {
    let Some((p, m)) = with_screens(cx) else {
        return plain(404, "no installed plugin with screens has this name");
    };
    let uri = uri(cx);
    let target = uri.path_and_query().map_or("/", |t| t.as_str());
    let target = target
        .strip_prefix(&format!("/p/{}", path_param::<Name>(cx)))
        .unwrap_or(target)
        .to_owned();
    let path = target.split('?').next().unwrap_or_default();
    let declared = m.places.iter().any(|pl| {
        !pl.screen.is_empty()
            && path
                .strip_prefix(pl.screen.as_str())
                .is_some_and(|r| r.is_empty() || r.starts_with('/'))
    });
    if !declared || path.contains("..") || path.to_ascii_lowercase().contains("%2e") {
        return plain(
            403,
            &format!("{path} isn't under one of {}'s places' screens", m.name),
        );
    }
    let pass: Vec<(&str, String)> = ["accept", "last-event-id"]
        .into_iter()
        .filter_map(|h| Some((h, header(cx, h)?.to_owned())))
        .collect();
    if let Some(feed) = plugin::follow(&p, &target) {
        return shared(cx, feed);
    }
    let (backend, at) = (p.clone(), target.clone());
    let reply = tokio::task::spawn_blocking(move || {
        let headers: Vec<(&str, &str)> = pass.iter().map(|(k, v)| (*k, v.as_str())).collect();
        plugin::call(&backend, "GET", &at, &headers, &[], Duration::from_secs(5))
    })
    .await?;
    match reply {
        Ok(r) if r.streams() => shared(cx, plugin::share(p, target, r)?),
        Ok(r) => pass_on(cx, r, ops, || true).await,
        Err(e) => plain(502, &format!("{}: {e}", path_param::<Name>(cx))),
    }
}

fn shared(
    cx: &Cx,
    feed: impl futures_util::Stream<Item = plugin::Sse> + Send + 'static,
) -> Result<Response> {
    Sse::new(feed.map(|e| Result::<Event>::Ok(ops(e))))
        .keep_alive(KeepAlive::new())
        .into_response(cx)
}

#[derive(Deserialize)]
struct BlockAct {
    #[serde(default)]
    screen: String,
    #[serde(default)]
    block: String,
    action: String,
    #[serde(default)]
    values: serde_json::Map<String, serde_json::Value>,
}

#[route(POST "/p/{name}/action")]
async fn screen_action(cx: &Cx, Json(a): Json<BlockAct>) -> Result<Response> {
    let Some((p, m)) = with_screens(cx) else {
        return plain(404, "no installed plugin with screens has this name");
    };
    if !m.permissions.actions.contains(&a.action) {
        return plain(
            403,
            &format!("{} isn't in {}'s [permissions] actions", a.action, m.name),
        );
    }
    let by = request_context::<Paired>(cx).0.label.clone();
    let body = serde_json::json!({
        "screen": a.screen, "block": a.block, "action": a.action, "values": a.values, "by": by,
    });
    let reply = tokio::task::spawn_blocking(move || {
        let json = [("Content-Type", "application/json")];
        let deadline = Duration::from_secs(10);
        plugin::call(
            &p,
            "POST",
            "/action",
            &json,
            body.to_string().as_bytes(),
            deadline,
        )
    })
    .await?;
    match reply {
        Ok(r) => pass_on(cx, r, ops, || true).await,
        Err(e) => plain(502, &format!("{}: {e}", m.name)),
    }
}

#[route(GET "/plugins")]
async fn plugin_list(cx: &Cx) -> Result<Answer<serde_json::Value>> {
    let m = manager(cx);
    let list = tokio::task::spawn_blocking(move || {
        let rows = m.list().map_err(refused)?;
        let index = m.index();
        Ok(
            serde_json::json!({ "installed": rows, "index": index, "jobs": m.jobs(), "issues": ISSUES }),
        )
    });
    Ok(Answer(
        list.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

#[derive(Deserialize)]
struct LogQuery {
    name: String,
}

#[route(GET "/plugins/log")]
async fn plugin_log(cx: &Cx) -> Result<Response> {
    let q: LogQuery =
        parse_query_params(cx).map_err(|e| topcoat::router::error::bad_request(e.to_string()))?;
    let installed = manager(cx).installed()?;
    if !installed.iter().any(|l| l.name == q.name) {
        return plain(404, &format!("{} isn't installed.", q.name));
    }
    let text = std::fs::read(manager(cx).log(&q.name)).unwrap_or_default();
    let tail = text
        .get(text.len().saturating_sub(16 << 10)..)
        .unwrap_or_default();
    let tail = String::from_utf8_lossy(tail).into_owned();
    plain(
        200,
        if tail.is_empty() {
            "(no log yet)"
        } else {
            &tail
        },
    )
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Act {
    name: String,
    source: String,
    rev: Option<String>,
    commit: String,
    purge: bool,
    on: bool,
    text: String,
}

fn improve(
    m: &Manager,
    hub: &Hub,
    logins: &Logins,
    name: &str,
    text: &str,
) -> Result<String, HubError> {
    let (repo, manifest) = m.improve(name).map_err(refused)?;
    let ask = if text.trim().is_empty() {
        "Ask me what to change."
    } else {
        text.trim()
    };
    let prompt = format!(
        "You're improving the cooee plugin {name}: this folder is its checkout, on the branch cooee-improve. \
         cooee-plugin.toml says how it builds (`{}`) and runs; docs/plugins/ in cooee's repo has the plugin API. \
         Make the change here and build it to check it; don't push. When I approve in Settings › Plugins, cooee \
         commits the branch, builds it and swaps it in.\n\n{ask}",
        manifest.build
    );
    let rid = format!("improve-{name}-{}", now());
    let thread = agent_thread(hub, logins, &repo, &prompt, &rid)?;
    m.improving(name, Some(thread.clone())).map_err(refused)?;
    Ok(thread)
}

fn agent_thread(
    hub: &Hub,
    logins: &Logins,
    folder: &std::path::Path,
    prompt: &str,
    rid: &str,
) -> Result<String, HubError> {
    let agents = hub.agents().map_err(refused)?;
    let agent = agents.first().ok_or_else(|| {
        HubError::Refused("No agents in shed: run cooee doctor on the Mac.".to_owned())
    })?;
    let (account, launch) = logins.home(&agent.id, None);
    let recipe = Recipe {
        account,
        launch,
        ..Recipe::default()
    };
    hub.start(&agent.id, folder, prompt, rid, &recipe)
}

#[route(POST "/plugins/{id}")]
async fn plugin_act(cx: &Cx, Json(a): Json<Act>) -> Result<Answer<serde_json::Value>> {
    let (act, m, hub, logins) = (thread_id(cx), manager(cx), hub(cx), logins(cx));
    let done = tokio::task::spawn_blocking(move || {
        let said = |what: &str| Ok(serde_json::json!(what));
        let confirmed = |p: &Plan| {
            let again = "It changed since you confirmed: confirm again.";
            (p.commit == a.commit)
                .then_some(())
                .ok_or_else(|| HubError::Refused(again.to_owned()))
        };
        let done = match act.as_str() {
            "plan" if a.name.is_empty() => Ok(serde_json::json!(
                m.plan(&a.source, a.rev.as_deref()).map_err(refused)?
            )),
            "plan" => Ok(serde_json::json!(m.plan_update(&a.name).map_err(refused)?)),
            "add" => {
                let p = m.plan(&a.source, a.rev.as_deref()).map_err(refused)?;
                confirmed(&p)?;
                let name = p.manifest.name.clone();
                m.job(&name, "installing", move |m| m.add(&p))
                    .map_err(refused)?;
                said("installing")
            }
            "update" => {
                let p = m.plan_update(&a.name).map_err(refused)?;
                confirmed(&p)?;
                m.job(&a.name, "updating", move |m| m.update(&p))
                    .map_err(refused)?;
                said("updating")
            }
            "approve" => {
                let name = a.name.clone();
                let build = move |m: &Manager| m.approve(&name);
                m.job(&a.name, "building the improvement", build)
                    .map_err(refused)?;
                said("building the improvement")
            }
            "remove" => m
                .remove(&a.name, a.purge)
                .map_err(refused)
                .and_then(|()| said("removed")),
            "enable" => m
                .enable(&a.name, a.on)
                .map_err(refused)
                .and_then(|()| said(if a.on { "on" } else { "off" })),
            "rollback" => m
                .rollback(&a.name)
                .map_err(refused)
                .and_then(|()| said("rolled back")),
            "improve" => improve(&m, &hub, &logins, &a.name, &a.text)
                .map(|t| serde_json::json!({ "thread": t })),
            _ => Err(HubError::Refused(format!("no plugin action {act}"))),
        };
        hub.set_plugins(m.tabs());
        done
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

#[derive(Deserialize)]
struct PluginPush {
    text: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    tag: String,
}

#[route(POST "/plugin/push")]
async fn plugin_push(cx: &Cx, Json(b): Json<PluginPush>) -> Result<Response> {
    let p = &request_context::<Caller>(cx).0;
    if b.text.trim().is_empty() {
        return plain(400, "a push needs text");
    }
    let hash = if b.path.is_empty() {
        String::new()
    } else {
        format!("#{}", b.path)
    };
    let note = Note {
        title: p.label.clone(),
        body: b.text.chars().take(120).collect(),
        tag: if b.tag.is_empty() {
            format!("plugin-{}", p.name)
        } else {
            b.tag
        },
        url: format!("/p/{}{hash}", p.name),
        item: 0,
        say: false,
    };
    hub(cx).push().send(&note);
    Json(note).into_response(cx)
}

#[derive(Deserialize)]
struct Dot {
    on: bool,
}

#[route(POST "/plugin/dot")]
async fn plugin_dot(cx: &Cx, Json(d): Json<Dot>) -> Result<Response> {
    let name = request_context::<Caller>(cx).0.name.clone();
    let dots = app_context::<Arc<Dots>>(cx);
    let mut dots = dots.0.lock().map_err(|e| io::Error::other(e.to_string()))?;
    if d.on {
        dots.insert(name);
    } else {
        dots.remove(&name);
    }
    Json(d.on).into_response(cx)
}

#[route(POST "/plugin/card")]
async fn plugin_card(cx: &Cx, Json(c): Json<CardSpec>) -> Result<Answer<cooee_core::Item>> {
    let p = request_context::<Caller>(cx).0.clone();
    Ok(Answer(blocking(cx, move |hub| hub.card(&p, c)).await))
}

#[derive(Deserialize)]
struct Handoff {
    #[serde(alias = "thread")]
    to: String,
    from: String,
    text: String,
}

#[route(POST "/plugin/handoff")]
async fn plugin_handoff(cx: &Cx, Json(h): Json<Handoff>) -> Result<Answer<cooee_core::Item>> {
    Ok(Answer(
        blocking(cx, move |hub| {
            hub.link(&h.from, &h.to, "", true)?;
            hub.link(&h.to, &h.from, &h.text, false)
        })
        .await,
    ))
}

#[route(POST "/plugin/line")]
async fn plugin_line(cx: &Cx, Json(h): Json<Handoff>) -> Result<Answer<cooee_core::Item>> {
    if h.text.trim().is_empty() {
        return Ok(Answer(Err(HubError::Refused(
            "a line needs text".to_owned(),
        ))));
    }
    Ok(Answer(
        blocking(cx, move |hub| hub.link(&h.to, &h.from, &h.text, false)).await,
    ))
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(proxy)
        .route(plugin_push)
        .route(plugin_card)
        .route(plugin_handoff)
        .route(plugin_line)
        .route(plugin_list)
        .route(plugin_log)
        .route(plugin_act)
        .route(plugin_dot)
        .route(screen)
        .route(screen_action)
        .app_context(Arc::new(Dots::default()))
}
