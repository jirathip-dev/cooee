//! Routines' runner (#75): every 30 seconds it runs each routine whose due time passed, once, and records it;
//! `create_routine` and the routines' API change them. The run's key is the hub's `random_id`, so a crash between
//! a run and its record runs into the same thread or message again, never a second one.

use std::{
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex, PoisonError},
};

use cooee_core::{
    Body, Item,
    routines::{Ask, Edit, Outcome, Paused, Place, Routine, Routines, Run, Who},
};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body as Bytes, RouterBuilder,
        content::{Form, Json},
        response::Response,
        route,
    },
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
    /// The routines in `data`, run on `hub`; `new_thread` runs start with `start`.
    #[must_use]
    pub fn new(hub: Arc<Hub>, data: &std::path::Path, start: Start) -> Self {
        Self {
            hub,
            file: Routines::new(data),
            lock: Mutex::new(()),
            start,
            offset: Mutex::new((0, 0)),
        }
    }

    /// Runs [`Runner::tick`] every `beat`, from now on.
    pub fn every(runner: Arc<Self>, beat: std::time::Duration) {
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
        self.file.load().map_err(refused)
    }

    /// Changes the routines with `f`, under the lock, and saves them.
    fn change<T>(
        &self,
        f: impl FnOnce(&mut Vec<Routine>) -> Result<T, HubError>,
    ) -> Result<T, HubError> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut list = self.list()?;
        let done = f(&mut list)?;
        self.file.save(&list).map_err(refused)?;
        Ok(done)
    }

    /// `create_routine` by thread `id`'s agent (`token`: its MCP token): the routine is stored, then its chat gets
    /// the `Created routine` line. A Workflow session or a routine's own run can't make one.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, such a thread, or a routine `ask` doesn't make.
    pub fn create(&self, (id, token): (&str, &str), ask: Ask, now: u64) -> Result<Item, HubError> {
        let from = self.hub.token_of(id, token)?;
        if from.workflow.is_some() || from.routine.is_some() {
            return Err(refused(
                "NOT_HERE: a Workflow session or a routine's own run can't make routines",
            ));
        }
        let r = self.change(|list| {
            let r = Routine::make(list, ask, &from, now).map_err(refused)?;
            list.push(r.clone());
            Ok(r)
        })?;
        let place = match r.place {
            Place::Chat { .. } => "in this chat",
            Place::NewThread { .. } => "each run a new thread",
        };
        let body = Body::Routine {
            routine: r.id.clone(),
            name: r.name.clone(),
            when: r.when(),
            place: place.to_owned(),
        };
        self.hub.say(id, body)
    }

    /// Settings › routines' edit of routine `id`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for no such routine or an edit it refuses.
    pub fn edit(&self, id: &str, e: Edit, now: u64) -> Result<Routine, HubError> {
        self.change(|list| {
            let r = list.iter_mut().find(|r| r.id == id);
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
        self.change(|list| {
            list.retain(|r| r.id != id);
            Ok(())
        })
    }

    /// Runs each routine whose due time passed by `now`, once, and records it; a paused one only moves on (no
    /// burst when it resumes).
    pub fn tick(&self, now: u64) {
        let offset = self.offset(now);
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut list = match self.list() {
            Ok(list) => list,
            Err(e) => return eprintln!("cooee: routines: {e}"),
        };
        for at in 0..list.len() {
            let Some(due) = list[at].due(now, offset) else {
                continue;
            };
            let r = &mut list[at];
            if r.paused.is_none() {
                let outcome = self.run(r, due, offset);
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
                    key: r.key(due),
                    outcome,
                });
            }
            r.last_due = due;
            if let Err(e) = self.file.save(&list) {
                eprintln!("cooee: routines: {e}");
            }
        }
    }

    /// Runs `r` for due time `due`: into its chat, or as a new thread.
    fn run(&self, r: &Routine, due: u64, offset: i32) -> Outcome {
        let (text, key, mark) = (r.text(due, offset), r.key(due), Some(r.mark()));
        let ran = match &r.place {
            Place::Chat { thread } => self
                .hub
                .queue_as(thread, (&text, &key), &[], mark)
                .map(|_| thread.clone()),
            Place::NewThread { who } => {
                (self.start)(&self.hub, who).and_then(|(agent, name, folder, recipe)| {
                    let recipe = Recipe {
                        routine: mark,
                        ..recipe
                    };
                    self.hub
                        .create_later((&agent, &name), &folder, (&text, &key), recipe)
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
    f: impl FnOnce(&Runner) -> Result<T, HubError> + Send + 'static,
) -> Result<T, HubError> {
    let r = runner(cx);
    tokio::task::spawn_blocking(move || f(&r))
        .await
        .map_err(refused)?
}

/// `POST /mcp/agent/routine` `{thread, server: {name, when, prompt, place?}}`: `create_routine`'s line.
#[route(POST "/mcp/agent/routine")]
async fn mcp_routine(cx: &Cx, Json(a): Json<AgentAsk>) -> Result<Answer<Item>> {
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
    Ok(Answer(on(cx, Runner::list).await))
}

/// A 204, or the reason.
fn back<T>(done: Result<T, HubError>) -> Result<Response> {
    match done {
        Ok(_) => Ok(Response::builder().status(204).body(Bytes::from(()))?),
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
        .route(routine_edit)
        .route(routine_delete)
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
