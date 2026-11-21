use std::sync::{Arc, PoisonError};

use cooee_core::{
    routines::{Delivery, Fields, Hooks, Outcome, Paused, Place, Routine, Run, Trigger, local},
    secrets,
};
use serde_json::Value;

use super::Runner;
use crate::{hub::Recipe, push::Note};

const HOUR: u64 = 3_600_000;

impl Runner {
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

    pub(super) fn record(&self, r: &mut Routine, key: String, outcome: Outcome, now: u64) {
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

    pub(super) fn run(&self, r: &Routine, text: &str, key: &str) -> Outcome {
        match &r.script {
            Some(s) => self.script(r, s, key),
            None => self.prompt(r, text, key),
        }
    }

    pub(super) fn prompt(&self, r: &Routine, text: &str, key: &str) -> Outcome {
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

    pub(super) fn offset(&self, now: u64) -> i32 {
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
