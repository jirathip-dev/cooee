use std::{io, path::PathBuf, sync::PoisonError};

use cooee_core::{ApnsEnv, Devices, Summary, write_private};
use serde::{Deserialize, Serialize};

use super::{Push, send::now, subs::LOCK};
use crate::net::{Post, post_all};

/// One device's Live Activity token: its push-to-start token, or its running activity's update token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityToken {
    /// The paired device it belongs to.
    pub device: String,
    /// The APNs token, hex.
    pub token: String,
    /// The activity it updates (none: the push-to-start token).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub activity: Option<String>,
    /// Its APNs server.
    pub env: ApnsEnv,
}

/// What a Live Activity push does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Starts the activity with the push-to-start token.
    Start,
    /// Shows a new state.
    Update,
    /// Ends it: it shows `replied` for 15 minutes.
    End,
}

/// The one Live Activity's state: the top thread, ranked asks › failed › running, and the rest as `+N`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Shown {
    /// `ask`, `failed`, `running` or `ended`.
    pub kind: String,
    /// The thread it opens.
    pub thread: String,
    /// The thread's title.
    pub title: String,
    /// Its persona's id (empty: none).
    pub persona: String,
    /// Its persona's name (else its agent).
    pub name: String,
    /// Its machine (empty: this host).
    pub machine: String,
    /// The question, the failure or the last line.
    pub line: String,
    /// Since when it is in this state, in Unix milliseconds.
    pub since: u64,
    /// The rest: `+2 running` (empty: none).
    pub more: String,
    /// The tools its turn ran so far.
    pub tools: usize,
}

/// Each row's kind for the Live Activity: `ask` (blocked, not a lane), `failed`, `running` (or none).
#[must_use]
pub fn kind(row: &Summary) -> Option<&'static str> {
    match row.state.as_str() {
        "blocked" if row.workflow.is_none() => Some("ask"),
        "error" => Some("failed"),
        "working" => Some("running"),
        _ => None,
    }
}

/// The one activity over `rows`: the first ask, else failed, else running; `since` says when each thread was first
/// seen in its kind.
#[must_use]
pub fn rank(rows: &[Summary], since: impl Fn(&str) -> u64) -> Option<Shown> {
    let order = ["ask", "failed", "running"];
    let mut ranked: Vec<(usize, &Summary)> = rows
        .iter()
        .filter(|r| !r.archived)
        .filter_map(|r| Some((order.iter().position(|k| Some(*k) == kind(r))?, r)))
        .collect();
    ranked.sort_by_key(|(at, _)| *at);
    let mut ranked = ranked.into_iter();
    let (at, top) = ranked.next()?;
    let mut rest = [0; 3];
    for (i, _) in ranked {
        if let Some(n) = rest.get_mut(i) {
            *n += 1;
        }
    }
    let words = ["ask", "failed", "running"].map(|w| (w, if w == "ask" { "asks" } else { w }));
    let more = rest.iter().zip(words).filter(|(n, _)| **n > 0);
    let more = more.map(|(n, (one, many))| format!("+{n} {}", if *n == 1 { one } else { many }));
    Some(Shown {
        kind: order.get(at).copied().unwrap_or("running").to_owned(),
        thread: top.id.clone(),
        title: top.title.clone(),
        persona: top.persona.clone(),
        name: who(top),
        machine: top.machine.clone(),
        line: top.last.clone(),
        since: since(&top.id),
        more: more.collect::<Vec<_>>().join(" · "),
        tools: 0,
    })
}

fn who(row: &Summary) -> String {
    if row.persona_name.is_empty() {
        row.agent.clone()
    } else {
        row.persona_name.clone()
    }
}

/// The push due when the activity goes from `sent` (shown at `at`) to `next` at `now` (milliseconds): a start, a
/// state change, an ask or an end at once; a new last line at most once per 15 s.
#[must_use]
pub fn due(sent: Option<&(Shown, u64)>, next: Option<&Shown>, now: u64) -> Option<Event> {
    match (sent, next) {
        (None, None) => None,
        (None, Some(_)) => Some(Event::Start),
        (Some(_), None) => Some(Event::End),
        (Some((was, _)), Some(n))
            if (&was.kind, &was.thread, &was.more) != (&n.kind, &n.thread, &n.more) =>
        {
            Some(Event::Update)
        }
        (Some((was, at)), Some(n)) => {
            (was.line != n.line && now >= at.saturating_add(15_000)).then_some(Event::Update)
        }
    }
}

/// One row of the glance: a thread or a lane that needs you or runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Glanced {
    /// The thread it opens.
    pub thread: String,
    /// Its title.
    pub title: String,
    /// Its persona's id (empty: none).
    pub persona: String,
    /// Its persona's name (else its agent).
    pub name: String,
    /// Its machine (empty: this host).
    pub machine: String,
    /// The question or the last line.
    pub line: String,
    /// Since when it is in this state, in Unix milliseconds.
    pub since: u64,
    /// A lane a tool started, not a chat.
    pub lane: bool,
}

/// What the widgets show (`GET /glance`): the asks, the blocked lanes and what runs, as of `at`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Overview {
    /// When it was read, in Unix milliseconds.
    pub at: u64,
    /// The threads that ask you.
    pub asks: Vec<Glanced>,
    /// The lanes that are blocked.
    pub blocked: Vec<Glanced>,
    /// The turns and lanes that run.
    pub running: Vec<Glanced>,
}

/// The glance over `rows` at `at`; `since` says when each thread was first seen in its state.
#[must_use]
pub fn overview(rows: &[Summary], since: impl Fn(&str) -> u64, at: u64) -> Overview {
    let row = |r: &Summary| Glanced {
        thread: r.id.clone(),
        title: r.title.clone(),
        persona: r.persona.clone(),
        name: who(r),
        machine: r.machine.clone(),
        line: r.last.clone(),
        since: since(&r.id),
        lane: r.workflow.is_some(),
    };
    let pick = |f: &dyn Fn(&Summary) -> bool| {
        rows.iter()
            .filter(|r| !r.archived && f(r))
            .map(row)
            .collect()
    };
    Overview {
        at,
        asks: pick(&|r| kind(r) == Some("ask")),
        blocked: pick(&|r| r.state == "blocked" && r.workflow.is_some()),
        running: pick(&|r| r.state == "working"),
    }
}

impl Push {
    fn activity_path(&self) -> PathBuf {
        self.data.join("activity.json")
    }

    /// Every device's Live Activity tokens.
    #[must_use]
    pub fn activity_tokens(&self) -> Vec<ActivityToken> {
        std::fs::read(self.activity_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    fn store_activity(&self, all: &[ActivityToken]) -> io::Result<()> {
        write_private(&self.activity_path(), &serde_json::to_vec_pretty(all)?)
    }

    /// Keeps `device`'s push-to-start `token` (no `activity`) or its activity's update token, replacing the one before.
    ///
    /// # Errors
    /// [`io::ErrorKind::InvalidInput`] for a token that is not hex; else when it can't be stored.
    pub fn register_activity(
        &self,
        device: &str,
        token: &str,
        activity: Option<String>,
        env: Option<ApnsEnv>,
    ) -> io::Result<()> {
        if !token.bytes().all(|b| b.is_ascii_hexdigit()) || !(32..=200).contains(&token.len()) {
            let why = "the token must be an APNs token, in hex";
            return Err(io::Error::new(io::ErrorKind::InvalidInput, why));
        }
        let env = env.or(self.apns.as_ref().map(|a| a.env));
        let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut all = self.activity_tokens();
        all.retain(|t| t.device != device || t.activity.is_some() != activity.is_some());
        all.push(ActivityToken {
            device: device.to_owned(),
            token: token.to_ascii_lowercase(),
            activity,
            env: env.unwrap_or(ApnsEnv::Production),
        });
        self.store_activity(&all)
    }

    /// Sends `event` with `shown` to each paired device's Live Activity (a start to its push-to-start token); an end
    /// or a gone token drops the update tokens. Its log lines.
    ///
    /// # Errors
    /// When the tokens can't be read or stored.
    pub fn send_activity(&self, event: Event, shown: &Shown) -> io::Result<Vec<String>> {
        let paired: Vec<String> = Devices::new(&self.data)
            .list()?
            .into_iter()
            .map(|d| d.id)
            .collect();
        let to: Vec<ActivityToken> = self
            .activity_tokens()
            .into_iter()
            .filter(|t| {
                paired.contains(&t.device) && t.activity.is_none() == (event == Event::Start)
            })
            .collect();
        let (a, k) = match self.apns_key() {
            Ok(ak) => ak,
            Err(_) if to.is_empty() => return Ok(Vec::new()),
            Err(why) => return Ok(vec![format!(": live activity: {why}")]),
        };
        let at = now();
        let aps = match event {
            Event::Start => serde_json::json!({
                "timestamp": at, "event": "start", "content-state": shown,
                "attributes-type": "CooeeActivity", "attributes": {},
                "alert": {"title": shown.title, "body": shown.line},
            }),
            Event::Update => {
                serde_json::json!({"timestamp": at, "event": "update", "content-state": shown})
            }
            Event::End => serde_json::json!({
                "timestamp": at, "event": "end", "content-state": shown, "dismissal-date": at + 15 * 60,
            }),
        };
        let body = serde_json::json!({ "aps": aps });
        let posts: Vec<Post> = to
            .iter()
            .map(|t| self.apns_request(a, &k, (t.env, &t.token), "liveactivity", &body))
            .collect();
        let (mut gone, mut log) = (Vec::new(), Vec::new());
        for (t, r) in to
            .iter()
            .zip(post_all(&posts, 10).map_err(io::Error::other)?)
        {
            match r.map(|r| (r.status, r.body.trim().to_owned())) {
                Ok((200..=299, _)) => {}
                Ok((410, _)) => gone.push(t.token.clone()),
                Ok((_, why)) if why.contains("BadDeviceToken") => gone.push(t.token.clone()),
                Ok((code, why)) => log.push(format!(" to device {}: HTTP {code} {why}", t.device)),
                Err(e) => log.push(format!(" to device {}: {e}", t.device)),
            }
        }
        if !gone.is_empty() || event == Event::End {
            let _g = LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            let mut all = self.activity_tokens();
            all.retain(|t| {
                !gone.contains(&t.token) && (event != Event::End || t.activity.is_none())
            });
            self.store_activity(&all)?;
        }
        Ok(log)
    }
}

#[cfg(test)]
mod tests {
    use cooee_core::Workflow;

    use super::*;
    use crate::push::{
        apns::tests::{apns, fake_apns, p8, paired},
        crypto::new_key,
        key::tests::dir,
    };

    fn row(id: &str, state: &str) -> Summary {
        Summary {
            id: id.to_owned(),
            title: format!("{id} title"),
            agent: "agent".to_owned(),
            state: state.to_owned(),
            last: format!("{id} line"),
            ..Summary::default()
        }
    }

    fn lane(id: &str, state: &str) -> Summary {
        let w = Workflow {
            source: "drover".to_owned(),
            ..Workflow::default()
        };
        Summary {
            workflow: Some(w),
            ..row(id, state)
        }
    }

    #[test]
    fn asks_rank_over_failed_over_running_and_the_rest_count_as_plus_n() {
        let named = Summary {
            persona_name: "Researcher".to_owned(),
            ..row("ask", "blocked")
        };
        let rows = [
            row("r1", "working"),
            row("f", "error"),
            lane("r2", "working"),
            named,
            row("d", "done"),
        ];
        let top = rank(&rows, |id| if id == "ask" { 7 } else { 0 }).unwrap();
        assert_eq!(
            (
                top.kind.as_str(),
                top.thread.as_str(),
                top.name.as_str(),
                top.since
            ),
            ("ask", "ask", "Researcher", 7)
        );
        assert_eq!(
            (top.line.as_str(), top.more.as_str()),
            ("ask line", "+1 failed · +2 running")
        );

        let top = rank(&rows[..1], |_| 0).unwrap();
        assert_eq!(
            (top.kind.as_str(), top.name.as_str(), top.more.as_str()),
            ("running", "agent", "")
        );
        let failed = rank(
            &[row("r1", "working"), row("f", "error"), row("g", "error")],
            |_| 0,
        )
        .unwrap();
        assert_eq!(
            (failed.thread.as_str(), failed.more.as_str()),
            ("f", "+1 failed · +1 running")
        );
        let asks = rank(
            &[
                row("a", "blocked"),
                row("b", "blocked"),
                row("c", "blocked"),
            ],
            |_| 0,
        )
        .unwrap();
        assert_eq!(asks.more, "+2 asks");
        let blocked_lane = rank(&[lane("l", "blocked"), row("r", "working")], |_| 0).unwrap();
        assert_eq!(
            (blocked_lane.kind.as_str(), blocked_lane.thread.as_str()),
            ("running", "r")
        );
        let archived = Summary {
            archived: true,
            ..row("old", "blocked")
        };
        assert_eq!(rank(&[archived, row("d", "done")], |_| 0), None);
    }

    #[test]
    fn a_new_last_line_waits_15_s_but_a_start_a_change_an_ask_or_an_end_goes_at_once() {
        let running = rank(&[row("r", "working")], |_| 0).unwrap();
        let said = Shown {
            line: "next line".to_owned(),
            ..running.clone()
        };
        let sent = (running.clone(), 1_000);
        assert_eq!(due(None, None, 0), None);
        assert_eq!(due(None, Some(&running), 0), Some(Event::Start));
        assert_eq!(due(Some(&sent), Some(&running), 60_000), None);
        assert_eq!(due(Some(&sent), Some(&said), 15_999), None);
        assert_eq!(due(Some(&sent), Some(&said), 16_000), Some(Event::Update));
        let ask = rank(&[row("a", "blocked"), row("r", "working")], |_| 0).unwrap();
        assert_eq!(due(Some(&sent), Some(&ask), 1_001), Some(Event::Update));
        let two = rank(&[row("r", "working"), row("s", "working")], |_| 0).unwrap();
        assert_eq!(due(Some(&sent), Some(&two), 1_001), Some(Event::Update));
        assert_eq!(due(Some(&sent), None, 1_001), Some(Event::End));
    }

    #[test]
    fn the_glance_lists_asks_blocked_lanes_and_running_turns_and_lanes_as_of_its_time() {
        let rows = [
            row("a", "blocked"),
            lane("l", "blocked"),
            row("r", "working"),
            lane("m", "working"),
            row("d", "done"),
            row("e", "error"),
        ];
        let g = overview(&rows, |id| u64::from(id.as_bytes()[0]), 42);
        let ids = |v: &[Glanced]| {
            v.iter()
                .map(|r| (r.thread.clone(), r.lane))
                .collect::<Vec<_>>()
        };
        assert_eq!(g.at, 42);
        assert_eq!(ids(&g.asks), [("a".to_owned(), false)]);
        assert_eq!(ids(&g.blocked), [("l".to_owned(), true)]);
        assert_eq!(
            ids(&g.running),
            [("r".to_owned(), false), ("m".to_owned(), true)]
        );
        let json = serde_json::to_value(&g).unwrap();
        assert_eq!(
            json["asks"][0],
            serde_json::json!({"thread": "a", "title": "a title", "persona": "", "name": "agent", "machine": "",
                "line": "a line", "since": 97, "lane": false})
        );
    }

    #[test]
    fn activity_tokens_are_kept_one_per_device_and_kind_and_a_bad_one_is_refused() {
        let push = Push::new(&dir("activity-tokens"));
        let tok = |c: &str| c.repeat(64);
        push.register_activity("a", &tok("A"), None, None).unwrap();
        push.register_activity("a", &tok("b"), Some("x".into()), Some(ApnsEnv::Sandbox))
            .unwrap();
        push.register_activity("a", &tok("c"), Some("y".into()), None)
            .unwrap();
        push.register_activity("b", &tok("d"), None, None).unwrap();
        let bad = push
            .register_activity("a", "not hex", None, None)
            .unwrap_err();
        assert_eq!(bad.kind(), io::ErrorKind::InvalidInput);
        let kept: Vec<(String, String, Option<String>)> = Push::new(&push.data)
            .activity_tokens()
            .into_iter()
            .map(|t| (t.device, t.token, t.activity))
            .collect();
        assert_eq!(
            kept,
            [
                ("a".into(), tok("a"), None),
                ("a".into(), tok("c"), Some("y".into())),
                ("b".into(), tok("d"), None)
            ]
        );
        assert!(push.activity_path().ends_with("activity.json"));
    }

    #[test]
    fn a_start_goes_to_the_push_to_start_token_and_an_end_to_the_update_token_which_it_drops() {
        let dir = dir("activity-send");
        let ids = paired(&dir, 1);
        let (url, got) = fake_apns(|_| (200, ""));
        write_private(&dir.join("apns.p8"), p8(&new_key().unwrap()).as_bytes()).unwrap();
        let push = Push::new(&dir).with_apns(Some(apns(&url)));
        let (start, update) = ("a".repeat(64), "b".repeat(64));
        push.register_activity(&ids[0], &start, None, None).unwrap();
        push.register_activity("unpaired", &"c".repeat(64), None, None)
            .unwrap();
        let shown = rank(&[row("r", "working"), row("s", "working")], |_| 5).unwrap();

        assert_eq!(
            push.send_activity(Event::Start, &shown).unwrap(),
            Vec::<String>::new()
        );
        let (head, body) = got.recv().unwrap();
        assert!(
            head.starts_with(&format!("post /3/device/{start} ")),
            "{head}"
        );
        for h in [
            "apns-push-type: liveactivity",
            "apns-topic: dev.cooee.app.push-type.liveactivity",
        ] {
            assert!(head.contains(&format!("{h}\r\n")), "{h} in {head}");
        }
        let aps = &serde_json::from_str::<serde_json::Value>(&body).unwrap()["aps"];
        assert_eq!(
            (&aps["event"], &aps["attributes-type"]),
            (&"start".into(), &"CooeeActivity".into())
        );
        assert_eq!(aps["content-state"]["more"], "+1 running");
        assert_eq!(aps["alert"]["title"], "r title");

        push.register_activity(&ids[0], &update, Some("act".into()), None)
            .unwrap();
        push.send_activity(Event::End, &shown).unwrap();
        let (head, body) = got.recv().unwrap();
        assert!(
            head.starts_with(&format!("post /3/device/{update} ")),
            "{head}"
        );
        let aps = &serde_json::from_str::<serde_json::Value>(&body).unwrap()["aps"];
        assert_eq!(aps["event"], "end");
        assert!(
            aps["dismissal-date"].as_u64().unwrap() >= aps["timestamp"].as_u64().unwrap() + 900
        );
        assert!(got.try_recv().is_err());
        let left: Vec<String> = push
            .activity_tokens()
            .into_iter()
            .map(|t| t.token)
            .collect();
        assert_eq!(left, [start, "c".repeat(64)]);
    }
}
