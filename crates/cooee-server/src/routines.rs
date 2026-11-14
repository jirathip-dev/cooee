//! Routines' runner (#75, #261): every 30 seconds it runs each routine whose due time passed, once, and records
//! it; a signed GitHub delivery on `[hooks] bind` (or one the 15-minute catch-up finds in GitHub's log) runs each
//! routine it matches. `create_routine` and Settings › routines change them. The run's key is the hub's
//! `random_id`, so a crash between a run and its record runs into the same thread or message again, never a second.

use std::{
    io::Write as _,
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use cooee_core::{
    Body, Hooks,
    routines::{
        Ask, Book, Delivery, Edit, Fields, Outcome, Paused, Place, Routine, Routines, Run, Trigger,
        Who, local,
    },
    secrets,
};
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body as Bytes, Router, RouterBuilder,
        content::{Form, Json},
        page,
        request::Bytes as Raw,
        response::Response,
        route,
    },
    view::View,
};

use crate::{
    AgentAsk, Answer, bearer,
    hub::{Hub, HubError, Recipe, now},
    plain,
    push::Note,
    refused, thread_id,
};

/// How a `new_thread` run starts as `who`: its agent, the agent's name, its folder and its recipe.
pub type Start =
    Box<dyn Fn(&Hub, &Who) -> Result<(String, String, PathBuf, Recipe), HubError> + Send + Sync>;

/// The routines and what runs them.
pub struct Runner {
    hub: Arc<Hub>,
    data: PathBuf,
    hooks: Option<Hooks>,
    file: Routines,
    /// Held over each read-change-write of the file.
    lock: Mutex<()>,
    start: Start,
    /// The Mac's UTC offset in minutes, and when it was read (ms).
    offset: Mutex<(i32, u64)>,
}

/// An hour, in milliseconds.
const HOUR: u64 = 3_600_000;

impl Runner {
    /// The routines in `data`, run on `hub`; `new_thread` runs start with `start`; GitHub events come on `hooks`.
    #[must_use]
    pub fn new(hub: Arc<Hub>, data: &std::path::Path, start: Start, hooks: Option<Hooks>) -> Self {
        Self {
            hub,
            data: data.to_owned(),
            hooks,
            file: Routines::new(data),
            lock: Mutex::new(()),
            start,
            offset: Mutex::new((0, 0)),
        }
    }

    /// Runs [`Runner::tick`] every `beat`, from now on, and with `[hooks]` [`Runner::catch_up`] every 15 minutes.
    pub fn every(runner: Arc<Self>, beat: Duration) {
        if runner.hooks.is_some() {
            let poll = Arc::clone(&runner);
            std::thread::spawn(move || {
                loop {
                    poll.catch_up(now());
                    std::thread::sleep(Duration::from_mins(15));
                }
            });
        }
        std::thread::spawn(move || {
            loop {
                runner.tick(now());
                std::thread::sleep(beat);
            }
        });
    }

    /// Every routine.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read.
    pub fn list(&self) -> Result<Vec<Routine>, HubError> {
        self.file.load().map(|b| b.routines).map_err(refused)
    }

    /// Changes the routines with `f`, under the lock, and saves them.
    fn change<T>(&self, f: impl FnOnce(&mut Book) -> Result<T, HubError>) -> Result<T, HubError> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut book = self.file.load().map_err(refused)?;
        let done = f(&mut book)?;
        self.file.save(&mut book).map_err(refused)?;
        Ok(done)
    }

    /// `create_routine` by thread `id`'s agent (`token`: its MCP token): a schedule is stored, then its chat gets
    /// the `Created routine` line; a GitHub one only raises a card, and is stored when the owner approves it with
    /// Face ID. A Workflow session or a routine's own run can't make one. What the agent is told.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, such a thread, no `[hooks]` for a GitHub one, or a routine `ask`
    /// doesn't make.
    pub fn create(
        self: &Arc<Self>,
        (id, token): (&str, &str),
        ask: Ask,
        now: u64,
    ) -> Result<String, HubError> {
        let from = self.hub.token_of(id, token)?;
        if from.workflow.is_some() || from.routine.is_some() {
            return Err(refused(
                "NOT_HERE: a Workflow session or a routine's own run can't make routines",
            ));
        }
        if ask.github.is_some() {
            if self.hooks.is_none() {
                return Err(refused("NO_HOOKS: [hooks] isn't set up on this Mac"));
            }
            let list = self.list()?;
            let r = Routine::make(&list, ask.clone(), &from, now).map_err(refused)?;
            let tool = format!(
                "save routine {}: {}, {}",
                r.name,
                r.when(),
                placed(&r.place)
            );
            let runner = Arc::downgrade(self);
            let save = Box::new(move || {
                let runner = runner.upgrade().ok_or("cooee is stopping")?;
                let saved = runner.change(|b| {
                    let r = Routine::make(&b.routines, ask, &from, crate::hub::now())
                        .map_err(refused)?;
                    b.routines.push(r.clone());
                    Ok(r)
                });
                saved
                    .map(|r| format!("saved routine {}", r.name))
                    .map_err(|e| e.to_string())
            });
            self.hub.propose_later(id, (tool, r.prompt), save)?;
            return Ok(
                "Proposed. Nothing is saved until the owner approves the card in this chat."
                    .to_owned(),
            );
        }
        let r = self.change(|b| {
            let r = Routine::make(&b.routines, ask, &from, now).map_err(refused)?;
            b.routines.push(r.clone());
            Ok(r)
        })?;
        let body = Body::Routine {
            routine: r.id.clone(),
            name: r.name.clone(),
            when: r.when(),
            place: placed(&r.place).to_owned(),
        };
        self.hub.say(id, body)?;
        Ok(
            "Made. The owner sees it in this chat and can change it in Settings › routines."
                .to_owned(),
        )
    }

    /// Settings › routines' edit of routine `id`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for no such routine or an edit it refuses.
    pub fn edit(&self, id: &str, e: Edit, now: u64) -> Result<Routine, HubError> {
        self.change(|b| {
            let r = b.routines.iter_mut().find(|r| r.id == id);
            let r = r.ok_or_else(|| refused(format!("no routine {id}")))?;
            r.edit(e, now).map_err(refused)?;
            Ok(r.clone())
        })
    }

    /// Deletes routine `id`; the threads it ran in stay.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read or written.
    pub fn delete(&self, id: &str) -> Result<(), HubError> {
        self.change(|b| {
            b.routines.retain(|r| r.id != id);
            Ok(())
        })
    }

    /// Runs each routine whose due time passed by `now`, once, and records it; a paused one only moves on (no
    /// burst when it resumes).
    pub fn tick(&self, now: u64) {
        let offset = self.offset(now);
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut book = match self.file.load() {
            Ok(book) => book,
            Err(e) => return eprintln!("cooee: routines: {e}"),
        };
        for at in 0..book.routines.len() {
            let Some(due) = book.routines[at].due(now, offset) else {
                continue;
            };
            let r = &mut book.routines[at];
            if r.paused.is_none() {
                let outcome = self.run(r, &r.text(&Fields::Due(due, offset)), &r.key(due));
                self.record(r, r.key(due), outcome, now);
            }
            r.last_due = due;
            if let Err(e) = self.file.save(&mut book) {
                eprintln!("cooee: routines: {e}");
            }
        }
    }

    /// Records `r`'s run `key` at `now`; a `chat` run whose chat is closed or gone pauses it, with a push.
    fn record(&self, r: &mut Routine, key: String, outcome: Outcome, now: u64) {
        if let (Outcome::Failed { .. }, Place::Chat { thread }) = (&outcome, &r.place)
            && !self.hub.thread(thread).is_ok_and(|(t, _)| !t.closed)
        {
            r.paused = Some(Paused::ChatGone {
                thread: thread.clone(),
            });
            self.hub.push().send(&Note {
                title: format!("Routine {} paused", r.name),
                body: "its chat was closed or deleted".to_owned(),
                tag: r.id.clone(),
                url: "/routines".to_owned(),
                item: 0,
            });
        }
        r.last_run = Some(Run {
            at: now,
            key,
            outcome,
        });
    }

    /// `POST /hooks/github`'s answer for a delivery with headers `X-GitHub-Event`, `X-GitHub-Delivery` and
    /// `X-Hub-Signature-256`: 503 without `secret:GITHUB_WEBHOOK_SECRET` (GitHub marks it failed; the catch-up takes
    /// it), 400 or 401 when [`Delivery::signed`] refuses it, else [`Runner::deliver`]'s.
    #[must_use]
    pub fn hook(
        self: &Arc<Self>,
        headers: (&str, &str, &str),
        body: &[u8],
        now: u64,
    ) -> (u16, String) {
        let secret = match secrets::get(&self.data, "GITHUB_WEBHOOK_SECRET") {
            Ok(Some(s)) => s,
            Ok(None) => return (503, "no secret:GITHUB_WEBHOOK_SECRET".to_owned()),
            Err(e) => return (503, e.to_string()),
        };
        match Delivery::signed(headers, body, secret.as_bytes()) {
            Ok(d) => self.deliver(&d, now),
            Err(e) => (e.status(), e.to_string()),
        }
    }

    /// Delivery `d`, once per guid: 200 when seen before or a ping; else 202, and each routine it matches runs on a
    /// worker (paused, or over the rate: skipped), so the answer comes before any run starts.
    pub fn deliver(self: &Arc<Self>, d: &Delivery, now: u64) -> (u16, String) {
        if d.event() == "ping" {
            return (200, "pong".to_owned());
        }
        let picked = self.change(|b| {
            if !b.see(d.guid(), now) {
                return Ok(None);
            }
            let mut runs = Vec::new();
            for r in b.routines.iter_mut().filter(|r| r.matches(d)) {
                match r.admit(now) {
                    Ok(()) => runs.push(r.clone()),
                    Err(why) => self.record(r, r.key(d.guid()), Outcome::Skipped { why }, now),
                }
            }
            Ok(Some(runs))
        });
        let runs = match picked {
            Ok(Some(runs)) => runs,
            Ok(None) => return (200, "seen".to_owned()),
            Err(e) => return (500, e.to_string()),
        };
        let n = runs.len();
        let (runner, d) = (Arc::clone(self), d.clone());
        std::thread::spawn(move || {
            for r in runs {
                let key = r.key(d.guid());
                let outcome = runner.run(&r, &r.text(&Fields::Event(&d)), &key);
                let saved = runner.change(|b| {
                    if let Some(o) = b.routines.iter_mut().find(|o| o.id == r.id) {
                        runner.record(o, key, outcome, crate::hub::now());
                    }
                    Ok(())
                });
                if let Err(e) = saved {
                    eprintln!("cooee: routines: {e}");
                }
            }
        });
        (202, format!("runs: {n}"))
    }

    /// The deliveries GitHub logged in the last 24 hours (and since a routine named the repo) for `[hooks] url`, on
    /// each repo a routine names, read with `secret:GITHUB_TOKEN`: each guid not seen goes through
    /// [`Runner::deliver`].
    pub fn catch_up(self: &Arc<Self>, now: u64) {
        let Some(hooks) = &self.hooks else { return };
        let mut repos: Vec<(String, u64)> = Vec::new();
        for r in self.list().unwrap_or_default() {
            if let Trigger::Github(g) = &r.trigger {
                let repo = g.repo.to_lowercase();
                match repos.iter_mut().find(|(o, _)| *o == repo) {
                    Some((_, since)) => *since = (*since).min(r.made_at),
                    None => repos.push((repo, r.made_at)),
                }
            }
        }
        if repos.is_empty() {
            return;
        }
        let token = match secrets::get(&self.data, "GITHUB_TOKEN") {
            Ok(Some(t)) => t,
            Ok(None) => {
                return eprintln!("cooee: routines: no secret:GITHUB_TOKEN for the catch-up");
            }
            Err(e) => return eprintln!("cooee: routines: GITHUB_TOKEN: {e}"),
        };
        for (repo, since) in repos {
            let since = local(since.max(now.saturating_sub(24 * HOUR)), 0).replace(' ', "T");
            if let Err(e) = self.missed(hooks, &token, &repo, &since, now) {
                eprintln!("cooee: routines: catch-up {repo}: {e}");
            }
        }
    }

    /// Repo `repo`'s hook for `hooks.url`: its deliveries since `since` (`YYYY-MM-DDTHH:MM`, UTC) not seen, delivered.
    fn missed(
        self: &Arc<Self>,
        hooks: &Hooks,
        token: &str,
        repo: &str,
        since: &str,
        now: u64,
    ) -> Result<(), String> {
        let api = format!("{}/repos/{repo}/hooks", hooks.api.trim_end_matches('/'));
        let list = github(&api, token)?;
        let hook = list
            .as_array()
            .into_iter()
            .flatten()
            .find(|h| h["config"]["url"] == hooks.url.as_str());
        let Some(hook) = hook.and_then(|h| h["id"].as_u64()) else {
            return Err(format!("no hook for {}", hooks.url));
        };
        let log = github(&format!("{api}/{hook}/deliveries?per_page=100"), token)?;
        for d in log.as_array().into_iter().flatten() {
            let (guid, at) = (
                d["guid"].as_str().unwrap_or_default(),
                d["delivered_at"].as_str(),
            );
            let seen = self.file.load().map_err(|e| e.to_string())?.seen;
            if at.is_none_or(|at| at < since) || seen.iter().any(|(g, _)| g == guid) {
                continue;
            }
            let full = github(&format!("{api}/{hook}/deliveries/{}", d["id"]), token)?;
            let event = full["event"].as_str().unwrap_or_default().to_owned();
            let payload = full["request"]["payload"].clone();
            self.deliver(&Delivery::fetched(guid.to_owned(), event, payload), now);
        }
        Ok(())
    }

    /// Runs `r` with prompt `text` and key `key`: into its chat, or as a new thread.
    fn run(&self, r: &Routine, text: &str, key: &str) -> Outcome {
        let mark = Some(r.mark());
        let ran = match &r.place {
            Place::Chat { thread } => self
                .hub
                .queue_as(thread, (text, key), &[], mark)
                .map(|_| thread.clone()),
            Place::NewThread { who } => {
                (self.start)(&self.hub, who).and_then(|(agent, name, folder, recipe)| {
                    let recipe = Recipe {
                        routine: mark,
                        ..recipe
                    };
                    self.hub
                        .create_later((&agent, &name), &folder, (text, key), recipe)
                })
            }
        };
        ran.map_or_else(
            |e| Outcome::Failed { why: e.to_string() },
            |thread| Outcome::Ran { thread },
        )
    }

    /// The Mac's UTC offset in minutes (`date +%z`), read again after an hour.
    fn offset(&self, now: u64) -> i32 {
        let mut o = self.offset.lock().unwrap_or_else(PoisonError::into_inner);
        if o.1 == 0 || now.saturating_sub(o.1) >= HOUR {
            let out = Command::new("date").arg("+%z").output();
            let text = out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
            match text.ok().as_deref().and_then(minutes) {
                Some(m) => *o = (m, now),
                None => eprintln!("cooee: routines: can't read the UTC offset; using {}", o.0),
            }
        }
        o.0
    }
}

fn runner(cx: &Cx) -> Arc<Runner> {
    Arc::clone(app_context::<Arc<Runner>>(cx))
}

/// Runs `f` on the runner off the async workers.
async fn on<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Arc<Runner>) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let r = runner(cx);
    tokio::task::spawn_blocking(move || f(&r))
        .await
        .map_err(refused)?
}

/// `POST /mcp/agent/routine` `{thread, server: {name, when, prompt, place?}}`: `create_routine`'s line.
#[route(POST "/mcp/agent/routine")]
async fn mcp_routine(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<String>> {
    let token = bearer(cx);
    Ok(Answer(
        on(cx, move |r| {
            let ask = serde_json::from_value(a.server).map_err(refused)?;
            r.create((&a.thread, &token), ask, now())
        })
        .await,
    ))
}

/// `GET /routines.json`: every routine.
#[route(GET "/routines.json")]
async fn routine_list(cx: &Cx) -> Result<Answer<Vec<Routine>>> {
    Ok(Answer(on(cx, |r| r.list()).await))
}

/// `GET /routines`: Settings › routines.
#[page("/routines")]
async fn routine_page(cx: &Cx) -> Result<impl View> {
    let list = on(cx, |r| r.list()).await.map_err(std::io::Error::other)?;
    Ok(cooee_web::routines_page(cx, list))
}

/// Back to Settings › routines, or the reason.
fn back<T>(done: Result<T, HubError>) -> Result<Response> {
    match done {
        Ok(_) => Ok(Response::builder()
            .status(303)
            .header("Location", "/routines")
            .body(Bytes::from(()))?),
        Err(e) => plain(400, &e.to_string()),
    }
}

/// `POST /routines/{id}` `name`, `when`, `prompt`, `paused` (each optional): its edit.
#[route(POST "/routines/{id}")]
async fn routine_edit(cx: &Cx, Form(e): Form<Edit>) -> Result<Response> {
    let id = thread_id(cx);
    back(on(cx, move |r| r.edit(&id, e, now())).await)
}

/// `POST /routines/{id}/delete`.
#[route(POST "/routines/{id}/delete")]
async fn routine_delete(cx: &Cx) -> Result<Response> {
    let id = thread_id(cx);
    back(on(cx, move |r| r.delete(&id)).await)
}

/// The routines' routes, on `runner`.
pub fn routes(runner: Arc<Runner>, b: RouterBuilder) -> RouterBuilder {
    b.app_context(runner)
        .route(mcp_routine)
        .route(routine_list)
        .page(routine_page)
        .route(routine_edit)
        .route(routine_delete)
}

/// `POST /hooks/github`: the only route on `[hooks] bind` (its body at most topcoat's default 2 MiB, else 413).
#[route(POST "/hooks/github")]
async fn github_hook(cx: &Cx, body: Raw) -> Result<Response> {
    let h = |n: &str| crate::header(cx, n).unwrap_or_default().to_owned();
    let headers = (
        h("x-github-event"),
        h("x-github-delivery"),
        h("x-hub-signature-256"),
    );
    let r = runner(cx);
    let (status, text) = tokio::task::spawn_blocking(move || {
        r.hook((&headers.0, &headers.1, &headers.2), &body, now())
    })
    .await
    .unwrap_or_else(|e| (500, e.to_string()));
    Ok(Response::builder().status(status).body(Bytes::from(text))?)
}

/// `[hooks] bind`'s router: `POST /hooks/github` and nothing else; no Gate, no devices, no other route.
#[must_use]
pub fn hooks(runner: Arc<Runner>) -> Router {
    Router::builder()
        .app_context(runner)
        .route(github_hook)
        .build()
}

/// GitHub API `url`'s JSON, read with `token` (on curl's stdin, never its arguments).
fn github(url: &str, token: &str) -> Result<Value, String> {
    let mut c = Command::new("curl")
        .args(["-sSf", "--max-time", "20", "-K", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("curl: {e}"))?;
    let config = format!(
        "url = {url:?}\nheader = \"Authorization: Bearer {token}\"\nheader = \"Accept: application/vnd.github+json\"\n"
    );
    if let Some(mut stdin) = c.stdin.take() {
        stdin
            .write_all(config.as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let out = c.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "{url}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("{url}: {e}"))
}

/// Where a routine's runs post, in words.
const fn placed(p: &Place) -> &'static str {
    match p {
        Place::Chat { .. } => "in this chat",
        Place::NewThread { .. } => "each run a new thread",
    }
}

/// `+0700` as 420, `-0330` as -210.
fn minutes(z: &str) -> Option<i32> {
    let (sign, hhmm) = z.split_at_checked(1)?;
    let n: i32 = hhmm.parse().ok().filter(|_| hhmm.len() == 4)?;
    let m = n / 100 * 60 + n % 100;
    match sign {
        "+" => Some(m),
        "-" => Some(-m),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_offset_reads_as_minutes() {
        assert_eq!(super::minutes("+0700"), Some(420));
        assert_eq!(super::minutes("-0330"), Some(-210));
        assert_eq!(super::minutes("0700"), None);
    }
}
