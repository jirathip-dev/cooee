use std::{
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use cooee_core::{
    Body,
    routines::{
        Ask, Book, Delivery, Edit, Fields, Hooks, Inbound, Mode, Outcome, Paused, Place, Routine,
        Routines, Run, Trigger, Who, local,
    },
    secrets,
};
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{Body as Bytes, Router, request::Bytes as Raw, response::Response, route},
};

use crate::{
    http::refused,
    hub::{Hub, HubError, Recipe, now},
    push::Note,
};

mod builtin;
mod end;
mod guard;
mod script;

pub use guard::Guarded;
mod webhook;

/// How a `new_thread` run starts as `who`: its agent, the agent's name, its folder and its recipe.
pub type Start =
    Box<dyn Fn(&Hub, &Who) -> Result<(String, String, PathBuf, Recipe), HubError> + Send + Sync>;

/// The model and effort a background run of an agent picks when it names none (#219): its provider's cheapest.
pub type Cheapest = Box<dyn Fn(&str) -> (String, String) + Send + Sync>;

/// The routines and what runs them.
pub struct Runner {
    hub: Arc<Hub>,
    data: PathBuf,
    config: Option<PathBuf>,
    cheapest: Cheapest,
    hooks: Option<Hooks>,
    file: Routines,
    lock: Mutex<()>,
    start: Start,
    offset: Mutex<(i32, u64)>,
}

const HOUR: u64 = 3_600_000;

type Picked = (Routine, String, String);

impl Runner {
    /// The routines of config folder `config` (none: the data folder), their run state in `data`, run on `hub`;
    /// `new_thread` runs start with `start`, a persona's consolidation runs (#219) on `cheapest` unless it names a
    /// model; GitHub events come on `hooks`.
    #[must_use]
    pub fn new(
        hub: Arc<Hub>,
        (config, data): (Option<&std::path::Path>, &std::path::Path),
        (start, cheapest): (Start, Cheapest),
        hooks: Option<Hooks>,
    ) -> Self {
        Self {
            hub,
            data: data.to_owned(),
            config: config.map(ToOwned::to_owned),
            cheapest,
            hooks,
            file: Routines::new(config.unwrap_or(data), data),
            lock: Mutex::new(()),
            start,
            offset: Mutex::new((0, 0)),
        }
    }

    /// Runs [`Runner::tick`] every `beat`, from now on; with `[hooks]`, [`hooks`] on its `bind` and
    /// [`Runner::catch_up`] every 15 minutes. Call it inside the server's runtime.
    ///
    /// # Errors
    /// When `[hooks] bind` can't be listened on.
    pub fn every(runner: Arc<Self>, beat: Duration) -> std::io::Result<()> {
        if let Some(h) = &runner.hooks {
            let listener = std::net::TcpListener::bind(h.bind)?;
            listener.set_nonblocking(true)?;
            let listener = tokio::net::TcpListener::from_std(listener)?;
            tokio::spawn(topcoat::serve(listener, hooks(Arc::clone(&runner))));
            let poll = Arc::clone(&runner);
            std::thread::spawn(move || {
                loop {
                    poll.catch_up(now());
                    cooee_core::clock::sleep(Duration::from_mins(15));
                }
            });
        }
        std::thread::spawn(move || {
            loop {
                runner.tick(now());
                cooee_core::clock::sleep(beat);
            }
        });
        Ok(())
    }

    /// Every routine.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read.
    pub fn list(&self) -> Result<Vec<Routine>, HubError> {
        self.file.load().map(|b| b.routines).map_err(refused)
    }

    /// Routine `r`'s public URL, when it is a webhook one and `[hooks]` is set.
    #[must_use]
    pub fn url(&self, r: &Routine) -> Option<String> {
        let hooks = self
            .hooks
            .as_ref()
            .filter(|_| matches!(r.trigger, Trigger::Webhook(_)))?;
        Some(hooks.inbound(&r.id))
    }

    fn change<T>(&self, f: impl FnOnce(&mut Book) -> Result<T, HubError>) -> Result<T, HubError> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut book = self.file.load().map_err(refused)?;
        let done = f(&mut book)?;
        self.file.save(&book).map_err(refused)?;
        Ok(done)
    }

    /// `create_routine` by thread `id`'s agent (`token`: its MCP token): a schedule is stored, then its chat gets
    /// the `Created routine` line; a GitHub, webhook or script one only raises a card, and is stored when the owner
    /// approves it with Face ID. A Workflow session or a routine's own run can't make one. What the agent is told.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, such a thread, no `[hooks]` for a GitHub or webhook one, or a routine `ask`
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
        let outside = ask.github.is_some() || ask.webhook.is_some() || ask.mail;
        if outside && self.hooks.is_none() {
            return Err(refused("NO_HOOKS: [hooks] isn't set up on this Mac"));
        }
        let inbox = self.hub.persona(&from.profile).mail;
        if ask.mail && !inbox.is_some_and(|m| m.on && !m.hook.is_empty()) {
            return Err(refused(
                "BAD_TRIGGER: mail arrives only at a persona's catch-all inbox, and it is off",
            ));
        }
        if outside || ask.script.is_some() {
            let list = self.list()?;
            let r = Routine::make(&list, ask.clone(), &from, (now, self.offset(now)))
                .map_err(refused)?;
            let kind = match r.script.as_ref().map(|s| s.mode) {
                Some(Mode::Post) => ", a script that posts its output",
                Some(Mode::Wake) => ", a script that wakes the agent when it fails",
                None => "",
            };
            let tool = format!(
                "save routine {}: {}, {}{kind}",
                r.name,
                self.when(&r, now),
                placed(&r.place)
            );
            let runner = Arc::downgrade(self);
            let save = Box::new(move || {
                let runner = runner.upgrade().ok_or("cooee is stopping")?;
                let now = crate::hub::now();
                let offset = runner.offset(now);
                let saved = runner.change(|b| {
                    let r =
                        Routine::make(&b.routines, ask, &from, (now, offset)).map_err(refused)?;
                    b.routines.push(r.clone());
                    Ok(r)
                });
                saved
                    .map(|r| format!("saved routine {}", r.name))
                    .map_err(|e| e.to_string())
            });
            let shown = r.script.map_or(r.prompt, |s| s.run);
            self.hub.propose_later(id, (tool, shown), save)?;
            return Ok(
                "Proposed. Nothing is saved until the owner approves the card in this chat."
                    .to_owned(),
            );
        }
        let r = self.make(id, ask, &from, now)?;
        Ok(format!(
            "Made routine {}. The owner sees it in this chat and can change it in Settings › routines; stop_routine ends it.",
            r.id
        ))
    }

    pub(crate) fn make(
        &self,
        id: &str,
        ask: Ask,
        from: &cooee_core::Thread,
        now: u64,
    ) -> Result<Routine, HubError> {
        let r = self.change(|b| {
            let r =
                Routine::make(&b.routines, ask, from, (now, self.offset(now))).map_err(refused)?;
            b.routines.push(r.clone());
            Ok(r)
        })?;
        let body = Body::Routine {
            routine: r.id.clone(),
            name: r.name.clone(),
            when: self.when(&r, now),
            place: placed(&r.place).to_owned(),
        };
        self.hub.say(id, body)?;
        Ok(r)
    }

    /// Settings › routines' edit of routine `id`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for no such routine or an edit it refuses.
    pub fn edit(&self, id: &str, e: Edit, now: u64) -> Result<Routine, HubError> {
        self.change(|b| {
            let r = b.routines.iter_mut().find(|r| r.id == id);
            let r = r.ok_or_else(|| refused(format!("no routine {id}")))?;
            let was = r.clone();
            r.edit(e, now).map_err(refused)?;
            let resumed = was.paused.is_some() && r.paused.is_none();
            if resumed || was.prompt != r.prompt || was.trigger != r.trigger {
                self.fresh(r);
            }
            Ok(r.clone())
        })
    }

    /// Deletes routine `id`; the threads it ran in stay.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read or written.
    pub fn delete(&self, id: &str) -> Result<(), HubError> {
        self.change(|b| {
            for r in b.routines.iter().filter(|r| r.id == id) {
                self.forget(r);
            }
            b.routines.retain(|r| r.id != id);
            Ok(())
        })
    }

    /// Ends each routine whose end time passed by `now`, then runs each whose due time passed, once, and records it; a
    /// paused or ended one only moves on (no burst when it resumes).
    pub fn tick(&self, now: u64) {
        let offset = self.offset(now);
        self.consolidate(now, offset);
        self.heartbeat(now, offset);
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut book = match self.file.load() {
            Ok(book) => book,
            Err(e) => return cooee_core::note!("cooee: routines: {e}"),
        };
        let made: Vec<()> = book
            .routines
            .iter_mut()
            .filter(|r| r.made_at == 0)
            .map(|r| r.made_at = now)
            .collect();
        let quiet: Vec<(Routine, u64)> = book
            .routines
            .iter_mut()
            .filter_map(|r| r.silence(now).map(|m| (r.clone(), m)))
            .collect();
        if !(made.is_empty() && quiet.is_empty())
            && let Err(e) = self.file.save(&book)
        {
            cooee_core::note!("cooee: routines: {e}");
        }
        for (r, m) in quiet {
            self.hub.push().send(&Note {
                title: format!("Routine {} is silent", r.name),
                body: format!("no signed POST for {m} min"),
                tag: r.id.clone(),
                url: "/routines".to_owned(),
                item: 0,
                say: false,
            });
            self.note(&r, &format!("no signed POST for {m} min"));
        }
        for at in 0..book.routines.len() {
            let Some(r) = book.routines.get_mut(at) else {
                continue;
            };
            let ended = r.end(now);
            let due = r.due(now, offset);
            if let Some(due) = due {
                if r.paused.is_none() && self.admit_due(r, now).is_ok() {
                    let outcome = self.run(r, &r.text(&Fields::Due(due, offset)), &r.key(due));
                    self.count(r, &outcome, now);
                    self.record(r, r.key(due), outcome, now);
                }
                r.last_due = due;
            }
            if (ended || due.is_some())
                && let Err(e) = self.file.save(&book)
            {
                cooee_core::note!("cooee: routines: {e}");
            }
        }
    }

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
                say: false,
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
    /// worker (paused, over the rate or at its cap: skipped), so the answer comes before any run starts.
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
                let key = r.key(d.guid());
                match r.admit(now).and_then(|()| self.admit_event(r, now)) {
                    Ok(()) => runs.push((r.clone(), r.text(&Fields::Event(d)), key)),
                    Err(why) => self.record(r, key, Outcome::Skipped { why }, now),
                }
            }
            Ok(Some(runs))
        });
        self.later(picked)
    }

    /// Signed POST `i` to routine `id`, once per key: 200 when seen before; else 202 and the routine runs on a
    /// worker (paused, over the rate or at its cap: skipped), so the answer comes before the run starts.
    pub fn accept(self: &Arc<Self>, id: &str, i: &Inbound, now: u64) -> (u16, String) {
        let picked = self.change(|b| {
            if !b.see(&format!("{id}:{}", i.key()), now) {
                return Ok(None);
            }
            let mut runs = Vec::new();
            for r in b.routines.iter_mut().filter(|r| r.id == id) {
                let key = r.key(i.key());
                match r.admit(now).and_then(|()| self.admit_event(r, now)) {
                    Ok(()) => runs.push((r.clone(), r.text(&Fields::Inbound(i)), key)),
                    Err(why) => self.record(r, key, Outcome::Skipped { why }, now),
                }
            }
            Ok(Some(runs))
        });
        self.later(picked)
    }

    fn later(self: &Arc<Self>, picked: Result<Option<Vec<Picked>>, HubError>) -> (u16, String) {
        let runs = match picked {
            Ok(Some(runs)) => runs,
            Ok(None) => return (200, "seen".to_owned()),
            Err(e) => return (500, e.to_string()),
        };
        let n = runs.len();
        let runner = Arc::clone(self);
        std::thread::spawn(move || {
            for (r, text, key) in runs {
                let outcome = runner.run(&r, &text, &key);
                let saved = runner.change(|b| {
                    if let Some(o) = b.routines.iter_mut().find(|o| o.id == r.id) {
                        runner.record(o, key, outcome, crate::hub::now());
                    }
                    Ok(())
                });
                if let Err(e) = saved {
                    cooee_core::note!("cooee: routines: {e}");
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
                return cooee_core::note!(
                    "cooee: routines: no secret:GITHUB_TOKEN for the catch-up"
                );
            }
            Err(e) => return cooee_core::note!("cooee: routines: GITHUB_TOKEN: {e}"),
        };
        for (repo, since) in repos {
            let since = local(since.max(now.saturating_sub(24 * HOUR)), 0).replace(' ', "T");
            if let Err(e) = self.missed(hooks, &token, &repo, &since, now) {
                cooee_core::note!("cooee: routines: catch-up {repo}: {e}");
            }
        }
    }

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
            .find(|h| h.pointer("/config/url").and_then(Value::as_str) == Some(&hooks.url));
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
            let event = full
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let payload = full
                .pointer("/request/payload")
                .cloned()
                .unwrap_or_default();
            self.deliver(&Delivery::fetched(guid.to_owned(), event, payload), now);
        }
        Ok(())
    }

    fn run(&self, r: &Routine, text: &str, key: &str) -> Outcome {
        match &r.script {
            Some(s) => self.script(r, s, key),
            None => self.prompt(r, text, key),
        }
    }

    fn prompt(&self, r: &Routine, text: &str, key: &str) -> Outcome {
        let mark = Some(r.mark());
        let ran = match &r.place {
            Place::Chat { thread } => self
                .hub
                .queue_as(thread, (text, key), &[], (mark, None))
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

    fn offset(&self, now: u64) -> i32 {
        let mut o = self.offset.lock().unwrap_or_else(PoisonError::into_inner);
        if o.1 == 0 || now.saturating_sub(o.1) >= HOUR {
            let out = cooee_core::runner::command("date").arg("+%z").output();
            let text = out.map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned());
            match text.ok().as_deref().and_then(minutes) {
                Some(m) => *o = (m, now),
                None => {
                    cooee_core::note!("cooee: routines: can't read the UTC offset; using {}", o.0);
                }
            }
        }
        o.0
    }
}

fn runner(cx: &Cx) -> Arc<Runner> {
    Arc::clone(app_context::<Arc<Runner>>(cx))
}

pub(crate) async fn on<T: Send + 'static>(
    cx: &Cx,
    f: impl FnOnce(&Arc<Runner>) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let r = runner(cx);
    tokio::task::spawn_blocking(move || f(&r))
        .await
        .map_err(refused)?
}

#[route(POST "/hooks/github")]
async fn github_hook(cx: &Cx, body: Raw) -> Result<Response> {
    let h = |n: &str| crate::http::header(cx, n).unwrap_or_default().to_owned();
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

/// `[hooks] bind`'s router: `POST /hooks/github`, `POST /hooks/in/<id>`, `POST /hooks/mail/<persona>` and nothing else; no Gate, no devices, no other route.
#[must_use]
pub fn hooks(runner: Arc<Runner>) -> Router {
    Router::builder()
        .app_context(runner)
        .route(github_hook)
        .route(webhook::inbound_hook)
        .route(webhook::mail_hook)
        .build()
}

fn github(url: &str, token: &str) -> Result<Value, String> {
    let headers = [
        format!("Authorization: Bearer {token}"),
        "Accept: application/vnd.github+json".to_owned(),
    ];
    let r = crate::net::fetch("GET", url, &headers, None)?;
    if !r.ok() {
        return Err(format!("{url}: HTTP {}", r.status));
    }
    serde_json::from_str(&r.body).map_err(|e| format!("{url}: {e}"))
}

const fn placed(p: &Place) -> &'static str {
    match p {
        Place::Chat { .. } => "in this chat",
        Place::NewThread { .. } => "each run a new thread",
    }
}

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
