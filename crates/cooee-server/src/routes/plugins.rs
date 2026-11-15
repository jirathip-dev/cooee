//! Plugins: their proxy and screens, Settings › Plugins, and what a plugin's backend posts.

use std::{io, sync::Arc, time::Duration};

use cooee_core::Plugin;
use futures_util::stream;
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
        Answer, Caller, Data, Name, Paired, blocking, header, hub, logins, now, plain, refused,
        thread_id,
    },
    passkey::{self, Unlocks},
    plugin,
};

/// The plugins whose tab shows a dot (`POST /plugin/dot`), by name; kept in memory only.
#[derive(Default)]
struct Dots(std::sync::Mutex<std::collections::BTreeSet<String>>);

/// The plugin tab the path names (a plugin with screens only has none).
fn named(cx: &Cx) -> Result<Plugin> {
    let name = path_param::<Name>(cx);
    let hub = app_context::<Arc<Hub>>(cx);
    let found = (hub.plugins().into_iter()).find(|p| p.name == name && !p.module.is_empty());
    Ok(found.ok_or_else(not_found)?)
}

/// A backend's event as one `{event, data}` message (a page's `EventSource` hears only those), its id kept.
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

/// `/p/<name>/api/<path>`: `<path>` on the plugin's backend with its token. Only the content type, `Accept` and
/// `Last-Event-ID` go along (never the cookie); an event stream comes back as `{event, data}` messages.
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
    // The gated tab: only an unlocked device's calls get through, and each counts as use; the backend learns which
    // device (it refuses calls without one).
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
        let timeout = Some(Duration::from_secs(30));
        plugin::call(&p, &method, &target, &headers, body.as_bytes(), timeout)
    })
    .await?;
    let reply = match reply {
        Ok(r) => r,
        Err(e) => return plain(502, &format!("{prefix}: {e}")),
    };
    // A locked device's stream ends at its next event.
    let open = move || !gated || unlocks.open(&me, now());
    pass_on(cx, reply, relay, open).await
}

/// A backend's answer passed on: an event stream event by event (each through `relay` while `open`), else its
/// status, content type and body.
async fn pass_on(
    cx: &Cx,
    reply: plugin::Reply,
    relay: fn(plugin::Sse) -> Event,
    open: impl Fn() -> bool + Send + 'static,
) -> Result<Response> {
    if reply
        .header("content-type")
        .is_some_and(|t| t.starts_with("text/event-stream"))
    {
        reply.body.get_ref().set_read_timeout(None)?;
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        std::thread::spawn(move || plugin::events(reply, |e| open() && tx.send(e).is_ok()));
        let stream = stream::unfold(rx, move |mut rx| async move {
            let e = rx.recv().await?;
            Some((Result::<Event>::Ok(relay(e)), rx))
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

/// Installed plugin `{name}`'s backend and manifest, when it has places: only those have screens (#125).
fn with_screens(cx: &Cx) -> Option<(Plugin, cooee_core::plugins::Manifest)> {
    let name = path_param::<Name>(cx);
    let data = &app_context::<Data>(cx).0;
    let lock = cooee_core::plugins::read_lock(data).ok()?;
    let m = cooee_core::plugins::manifest(data, lock.iter().find(|l| l.name == name)?).ok()?;
    let p = app_context::<Arc<Hub>>(cx).backend(name)?;
    (!m.places.is_empty()).then_some((p, m))
}

/// A screen stream's event as the backend sent it: its `{seq, ops}` data, its name and id.
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

/// `GET /p/<plugin>/screens/<path>` (#125): a screen from a plugin with places, off its backend with its token
/// (`Accept` and `Last-Event-ID` go along, never the cookie); a stream of block ops comes back event by event.
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
    let reply = tokio::task::spawn_blocking(move || {
        let headers: Vec<(&str, &str)> = pass.iter().map(|(k, v)| (*k, v.as_str())).collect();
        plugin::call(
            &p,
            "GET",
            &target,
            &headers,
            &[],
            Some(Duration::from_secs(5)),
        )
    })
    .await?;
    match reply {
        Ok(r) => pass_on(cx, r, ops, || true).await,
        Err(e) => plain(502, &format!("{}: {e}", path_param::<Name>(cx))),
    }
}

/// One action from a plugin's screen or card (#125): the block it came from and its values.
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

/// `POST /p/<plugin>/action` (#125): one action to its backend's `POST /action`, with the device that sent it as
/// `by`; one not in its manifest's `[permissions]` actions is refused (403) and never sent.
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
        let timeout = Some(Duration::from_secs(10));
        plugin::call(
            &p,
            "POST",
            "/action",
            &json,
            body.to_string().as_bytes(),
            timeout,
        )
    })
    .await?;
    match reply {
        Ok(r) => pass_on(cx, r, ops, || true).await,
        Err(e) => plain(502, &format!("{}: {e}", m.name)),
    }
}

fn manager(cx: &Cx) -> Arc<Manager> {
    Arc::clone(app_context::<Arc<Manager>>(cx))
}

/// `GET /plugins`: Settings › Plugins: the installed plugins with their state, the index, the jobs in progress.
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

/// `?name=`: whose log.
#[derive(Deserialize)]
struct LogQuery {
    name: String,
}

/// `GET /plugins/log?name=`: the end of a plugin's log (its builds' and backend's output).
#[route(GET "/plugins/log")]
async fn plugin_log(cx: &Cx) -> Result<Response> {
    let q: LogQuery =
        parse_query_params(cx).map_err(|e| topcoat::router::error::bad_request(e.to_string()))?;
    let installed = manager(cx).installed()?;
    if !installed.iter().any(|l| l.name == q.name) {
        return plain(404, &format!("{} isn't installed.", q.name));
    }
    let text = std::fs::read(manager(cx).log(&q.name)).unwrap_or_default();
    let tail = String::from_utf8_lossy(&text[text.len().saturating_sub(16 << 10)..]).into_owned();
    plain(
        200,
        if tail.is_empty() {
            "(no log yet)"
        } else {
            &tail
        },
    )
}

/// `POST /plugins/<action>`: Settings › Plugins' buttons.
#[derive(Deserialize, Default)]
#[serde(default)]
struct Act {
    name: String,
    source: String,
    rev: Option<String>,
    /// The commit the owner confirmed (add, update).
    commit: String,
    purge: bool,
    on: bool,
    text: String,
}

/// `[improve]`: a thread with the first agent (its default account) in the plugin's checkout, on its
/// `cooee-improve` branch, with the owner's `text`.
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
         cooee-plugin.toml says how it builds (`{}`) and runs; docs/plugins.md in cooee's repo has the plugin API. \
         Make the change here and build it to check it; don't push. When I approve in Settings › Plugins, cooee \
         commits the branch, builds it and swaps it in.\n\n{ask}",
        manifest.build
    );
    let rid = format!("improve-{name}-{}", now());
    let thread = agent_thread(hub, logins, &repo, &prompt, &rid)?;
    m.improving(name, Some(thread.clone())).map_err(refused)?;
    Ok(thread)
}

/// A thread with the first agent in shed (its default account) in `folder`, starting with `prompt`.
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
    let (account, launch) = logins.home(&agent.id, None)?;
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
        // A confirm holds for the exact commit it showed.
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
        // The tabs follow at once (the supervisor would within half a second).
        hub.set_plugins(m.tabs());
        done
    });
    Ok(Answer(
        done.await.map_err(|e| HubError::Refused(e.to_string()))?,
    ))
}

/// `POST /plugin/push`: a line from a plugin; a tap opens its tab (at `#<path>`, when given). A newer push of the same
/// `tag` (default `plugin-<name>`) replaces the older one.
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
    };
    hub(cx).push().send(&note);
    Json(note).into_response(cx)
}

/// `POST /plugin/dot` `{"on": bool}`: shows or clears the dot on the plugin's tab.
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

/// `POST /plugin/card`: a plugin's card in a thread ([`CardSpec`]).
#[route(POST "/plugin/card")]
async fn plugin_card(cx: &Cx, Json(c): Json<CardSpec>) -> Result<Answer<cooee_core::Item>> {
    let p = request_context::<Caller>(cx).0.clone();
    Ok(Answer(blocking(cx, move |hub| hub.card(&p, c)).await))
}

/// `POST /plugin/handoff` `{from, to, text}`: `from` shows `continued in` `to`, which gets `text` `from` it; `POST
/// /plugin/line` `{thread, from, text}`: a line flowing back into `thread` from thread `from` (#553 row 35).
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
