//! `GET /workflow` (#127): the Workflow sessions (#33) folded into one row per issue, by their labels `source`,
//! `group` and `item`, their rounds oldest first. What labels can't say comes from the source's plugin as it gives it
//! (`extra`, and `done` for an issue that is finished); cooee reads nothing into it.

use std::{collections::BTreeMap, io, time::Duration};

use cooee_core::{Plugin, Summary};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Hub, hub::now, plugin};

/// The row states, in the order rows are listed.
const ORDER: [&str; 4] = ["needs_you", "failed", "working", "ready"];

/// One round of an issue: one session.
#[derive(Debug, Serialize)]
pub struct Step {
    pub role: String,
    pub round: String,
    pub state: &'static str,
    /// Its last item, short.
    pub summary: String,
    /// Seconds since it last moved.
    pub age: u64,
    pub thread: String,
}

/// One issue: its newest round's stage, round and state, every round, and its plugin's `extra`.
#[derive(Debug, Serialize)]
pub struct Row {
    pub source: String,
    pub group: String,
    pub item: String,
    pub title: String,
    pub url: String,
    pub stage: String,
    pub round: String,
    pub state: &'static str,
    pub age: u64,
    pub rounds: Vec<Step>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

/// What a source's plugin says of one of its items.
#[derive(Debug, Default, Deserialize)]
pub struct Given {
    pub group: String,
    pub item: String,
    /// Finished (merged or closed): its row leaves.
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub extra: Option<Value>,
}

/// A session's thread state as a row state.
fn state(thread: &str) -> &'static str {
    match thread {
        "blocked" => "needs_you",
        "error" | "unconnected" => "failed",
        "working" | "connecting" => "working",
        _ => "ready",
    }
}

/// Folds `sessions` (each with when it started) into rows at `now`, with what each source's plugin `given` says.
#[must_use]
pub fn rows(
    sessions: Vec<(Summary, u64)>,
    given: &BTreeMap<String, Vec<Given>>,
    now: u64,
) -> Vec<Row> {
    let mut by = BTreeMap::<_, Vec<_>>::new();
    for (s, start) in sessions {
        if let Some(w) = s.workflow.clone() {
            let key = (w.source.clone(), w.group.clone(), w.item.clone());
            by.entry(key).or_default().push((w, s, start));
        }
    }
    let mut rows: Vec<Row> = by
        .into_iter()
        .filter_map(|((source, group, item), mut s)| {
            let mine = |g: &&Given| g.group == group && g.item == item;
            let g = given.get(&source).into_iter().flatten().find(mine);
            if g.is_some_and(|g| g.done) {
                return None;
            }
            s.sort_by_key(|(_, _, start)| *start);
            let rounds: Vec<Step> = s
                .iter()
                .map(|(w, s, start)| Step {
                    role: w.role.clone(),
                    round: w.round.clone(),
                    state: state(&s.state),
                    summary: s.last.clone(),
                    age: now.saturating_sub(s.at.max(*start)) / 1000,
                    thread: s.id.clone(),
                })
                .collect();
            let ((w, s, _), last) = (s.last()?, rounds.last()?);
            Some(Row {
                title: s.title.clone(),
                url: w.url.clone(),
                stage: w.role.clone(),
                round: w.round.clone(),
                state: last.state,
                age: last.age,
                extra: g.and_then(|g| g.extra.clone()),
                source,
                group,
                item,
                rounds,
            })
        })
        .collect();
    rows.sort_by_key(|r| (ORDER.iter().position(|s| *s == r.state), r.age));
    rows
}

/// What the plugin named `source` says of its items: `GET /cooee/workflow` answers `{"items": [Given]}`. None without
/// such a plugin, or when it answers 404 or fails.
fn given(plugins: &[Plugin], source: &str) -> Vec<Given> {
    #[derive(Deserialize)]
    struct Items {
        items: Vec<Given>,
    }
    let Some(p) = plugins.iter().find(|p| p.name == source) else {
        return Vec::new();
    };
    let wait = Some(Duration::from_secs(5));
    let got =
        plugin::call(p, "GET", "/cooee/workflow", &[], &[], wait).and_then(|r| match r.status {
            200 => Ok(serde_json::from_slice::<Items>(&r.bytes()?)?.items),
            404 => Ok(Vec::new()),
            code => Err(io::Error::other(format!("the backend answered {code}"))),
        });
    got.unwrap_or_else(|e| {
        eprintln!("cooee: plugin {source}: its workflow: {e}");
        Vec::new()
    })
}

/// `GET /workflow`'s rows now.
pub fn read(hub: &Hub) -> Vec<Row> {
    let sessions = hub.sessions();
    let plugins = hub.plugins();
    let mut by = BTreeMap::new();
    for (s, _) in &sessions {
        if let Some(w) = &s.workflow {
            by.entry(w.source.clone())
                .or_insert_with(|| given(&plugins, &w.source));
        }
    }
    rows(sessions, &by, now())
}

#[cfg(test)]
mod tests {
    use cooee_core::Workflow;
    use serde_json::json;

    use super::*;

    /// Session `id` of `item`, as `role` in `round`, in thread state `state`, last moved at `at`.
    fn session(id: &str, item: &str, role: &str, round: &str, state: &str, at: u64) -> Summary {
        Summary {
            id: id.to_owned(),
            title: format!("cooee {item}"),
            agent: "claude".to_owned(),
            account: String::new(),
            profile: String::new(),
            folder: "/".into(),
            state: state.to_owned(),
            last: format!("{id} said"),
            at,
            unread: false,
            unseen: 0,
            archived: false,
            workflow: Some(Workflow {
                source: "herdsman".to_owned(),
                group: "cooee".to_owned(),
                item: item.to_owned(),
                role: role.to_owned(),
                round: round.to_owned(),
                url: format!("https://x/{item}"),
            }),
        }
    }

    #[test]
    fn sessions_of_one_item_fold_into_one_row_with_their_rounds_oldest_first() {
        let sessions = vec![
            (session("w-rev-1", "#9", "rev", "1", "done", 5_000), 3_000),
            (session("w-impl", "#9", "impl", "1", "done", 2_000), 1_000),
            (
                session("w-fix-1", "#9", "fix", "1", "working", 9_000),
                6_000,
            ),
        ];
        let rows = rows(sessions, &BTreeMap::new(), 10_000);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(
            (
                r.item.as_str(),
                r.title.as_str(),
                r.url.as_str(),
                r.stage.as_str(),
                r.round.as_str()
            ),
            ("#9", "cooee #9", "https://x/#9", "fix", "1")
        );
        assert_eq!((r.state, r.age, r.extra.as_ref()), ("working", 1, None));
        let rounds: Vec<_> = r
            .rounds
            .iter()
            .map(|r| {
                (
                    r.role.as_str(),
                    r.state,
                    r.age,
                    r.thread.as_str(),
                    r.summary.as_str(),
                )
            })
            .collect();
        assert_eq!(
            rounds,
            [
                ("impl", "ready", 8, "w-impl", "w-impl said"),
                ("rev", "ready", 5, "w-rev-1", "w-rev-1 said"),
                ("fix", "working", 1, "w-fix-1", "w-fix-1 said"),
            ]
        );
    }

    #[test]
    fn rows_list_needs_you_then_failed_then_working_then_ready() {
        let sessions = vec![
            (session("a", "#1", "rev", "1", "done", 1_000), 0),
            (session("b", "#2", "impl", "1", "working", 1_000), 0),
            (session("c", "#3", "impl", "1", "error", 1_000), 0),
            (session("d", "#4", "rev", "2", "blocked", 1_000), 0),
            (session("e", "#5", "impl", "1", "connecting", 1_000), 0),
        ];
        let got: Vec<_> = rows(sessions, &BTreeMap::new(), 1_000)
            .into_iter()
            .map(|r| (r.item, r.state))
            .collect();
        let want = [
            ("#4", "needs_you"),
            ("#3", "failed"),
            ("#2", "working"),
            ("#5", "working"),
            ("#1", "ready"),
        ];
        assert_eq!(got, want.map(|(i, s)| (i.to_owned(), s)));
    }

    #[test]
    fn a_finished_issue_leaves_and_the_plugin_extra_rides_as_given() {
        let sessions = || {
            vec![
                (session("a", "#1", "rev", "2", "done", 1_000), 0),
                (session("b", "#2", "impl", "1", "done", 1_000), 0),
            ]
        };
        assert_eq!(rows(sessions(), &BTreeMap::new(), 1_000).len(), 2);
        let given = |item: &str, done, extra| Given {
            group: "cooee".to_owned(),
            item: item.to_owned(),
            done,
            extra,
        };
        let pr = json!({ "pr": 60, "verdict": "PASS" });
        let plugin = BTreeMap::from([(
            "herdsman".to_owned(),
            vec![
                given("#1", true, None),
                given("#2", false, Some(pr.clone())),
            ],
        )]);
        let left = rows(sessions(), &plugin, 1_000);
        assert_eq!(left.len(), 1);
        assert_eq!(
            (left[0].item.as_str(), left[0].extra.as_ref()),
            ("#2", Some(&pr))
        );
    }
}
