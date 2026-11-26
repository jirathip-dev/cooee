use std::path::PathBuf;

use cooee_core::{
    agents::{self, BEAT, CONSOLIDATE, CONSOLIDATION, HEARTBEAT, Heartbeat},
    history::{self, Beat},
    routines::{Change, Mark, Who, day, local},
};
use serde_json::Value;

use super::Runner;
use crate::{
    http::refused,
    hub::{HubError, Recipe},
};

impl Runner {
    pub(super) fn consolidate(&self, now: u64, offset: i32) {
        for p in agents::list(self.config.as_deref(), &self.data) {
            let Some(c) = p.consolidate.as_ref().filter(|c| c.on) else {
                continue;
            };
            let Some(due) = c.schedule().and_then(|s| s.latest(now, offset)) else {
                continue;
            };
            let key = format!("{CONSOLIDATE}-{}", p.id);
            let line = history::watermark(&self.data, &key, |w| {
                let run = (w.due != 0 && due > w.due).then_some(w.line);
                w.due = w.due.max(due);
                run
            });
            let news = match line {
                Ok(Some(line)) => self.hub.news(&p.id, (line, false)),
                Ok(None) => continue,
                Err(e) => Err(refused(e)),
            };
            let ran = news.and_then(|news| {
                let Some(_) = news else { return Ok(()) };
                let mark = (CONSOLIDATE, "Consolidation");
                let text = (CONSOLIDATION, format!("routine-{key}-{due}"));
                self.as_persona(&p, (&c.model, &c.effort), mark, text, None)
            });
            if let Err(e) = ran {
                cooee_core::note!("cooee: routines: {key}: {e}");
            }
        }
    }

    pub(super) fn heartbeat(&self, now: u64, offset: i32) {
        for p in agents::list(self.config.as_deref(), &self.data) {
            let Some(h) = p.heartbeat.as_ref().filter(|h| h.on) else {
                continue;
            };
            let Some(due) = h.schedule().and_then(|s| s.latest(now, offset)) else {
                continue;
            };
            let key = format!("{HEARTBEAT}-{}", p.id);
            let (cap, today) = (h.per_day.unwrap_or(Heartbeat::PER_DAY), day(now, offset));
            let picked = history::watermark(&self.data, &key, |w| {
                let seen = w.due != 0 && due > w.due;
                w.due = w.due.max(due);
                if !seen {
                    return Ok::<_, HubError>(None);
                }
                let news = self.hub.news(&p.id, (w.line, true))?;
                if let Err(why) = w.check(today, cap, news.as_ref().map(|_| Change::Said)) {
                    w.held(today, why);
                    return Ok(None);
                }
                let Some((text, newest)) = news else {
                    return Ok(None);
                };
                w.ran(today, newest);
                let read = format!(
                    "{} new lines, {} characters",
                    text.lines().count(),
                    text.chars().count()
                );
                let run = format!("routine-{key}-{due}-{newest}");
                w.last = Some(Beat {
                    at: now,
                    run: run.clone(),
                    read,
                    why: String::new(),
                    pushed: false,
                });
                Ok(Some((text, run)))
            });
            let ran = picked.map_err(refused).and_then(|news| {
                let Some((news, run)) = news? else {
                    return Ok(());
                };
                self.run_beat(&p, h, &news, run)
            });
            if let Err(e) = ran {
                cooee_core::note!("cooee: routines: {key}: {e}");
            }
        }
    }

    fn run_beat(
        &self,
        p: &agents::Agent,
        h: &Heartbeat,
        news: &str,
        run: String,
    ) -> Result<(), HubError> {
        let home = self
            .config
            .as_deref()
            .and_then(|c| agents::home(c, &p.id).ok());
        let rule = home.as_ref().map(agents::notify).unwrap_or_default();
        let rule = if rule.is_empty() {
            "none, so stay silent"
        } else {
            &rule
        };
        let text =
            format!("{BEAT}\n\nThe owner's notify rule: {rule}\n\nNew since the last run:\n{news}");
        let mark = (HEARTBEAT, "Heartbeat");
        let text = (text.as_str(), run);
        self.as_persona(p, (&h.model, &h.effort), mark, text, Some("ask"))
    }

    fn as_persona(
        &self,
        p: &agents::Agent,
        (model, effort): (&str, &str),
        (id, name): (&str, &str),
        (text, random): (&str, String),
        policy: Option<&str>,
    ) -> Result<(), HubError> {
        let (cheap_model, cheap_effort) = (self.cheapest)(&p.agent);
        let pick = |own: &str, cheap: String| {
            if own.is_empty() {
                cheap
            } else {
                own.to_owned()
            }
        };
        let who = Who {
            agent: p.agent.clone(),
            account: p.account.clone().unwrap_or_default(),
            profile: p.id.clone(),
            folder: PathBuf::new(),
            model: pick(model, cheap_model),
            effort: pick(effort, cheap_effort),
            machine: p.machine.clone(),
        };
        let (agent, agent_name, folder, mut recipe) = (self.start)(&self.hub, &who)?;
        if let Some(policy) = policy {
            recipe.launch.policy = Some(policy.to_owned());
        }
        let recipe = Recipe {
            routine: Some(Mark {
                id: id.to_owned(),
                name: name.to_owned(),
            }),
            ..recipe
        };
        self.hub
            .create_later((&agent, &agent_name), &folder, (text, &random), recipe)?;
        Ok(())
    }

    /// Persona `persona`'s heartbeat (#498) as a row of its panel's routines: its schedule, and its last run's
    /// time, what it read and why it stayed silent; none when it has no heartbeat.
    #[must_use]
    pub fn beat(&self, persona: &str, now: u64) -> Option<Value> {
        let p = agents::find(self.config.as_deref(), &self.data, persona).ok()?;
        let h = p.heartbeat?;
        let key = format!("{HEARTBEAT}-{}", p.id);
        let last = history::watermark(&self.data, &key, |w| w.last.clone()).ok()?;
        let offset = self.offset(now);
        let next = last.map_or_else(
            || "no run yet".to_owned(),
            |b| {
                let why = match (b.pushed, b.why.as_str()) {
                    (true, _) => "pushed".to_owned(),
                    (false, "") => "running".to_owned(),
                    (false, why) => format!("silent: {why}"),
                };
                format!("last run {}: read {} · {why}", local(b.at, offset), b.read)
            },
        );
        let when = h.schedule().map(|s| s.describe()).unwrap_or_default();
        Some(
            serde_json::json!({ "id": HEARTBEAT, "name": "Heartbeat", "when": when, "on": h.on, "next": next }),
        )
    }
}
