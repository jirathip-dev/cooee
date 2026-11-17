use cooee_core::{Body, Item};

use super::{Hub, Live, lock};

const EXPIRED: &str = "expired";

impl Hub {
    pub(super) fn expire(&self, now: u64) {
        self.drop_frames(now);
        for live in self.all() {
            let due = {
                let l = lock(&live);
                let mut due = l
                    .chat
                    .items
                    .iter()
                    .filter(|i| now >= i.at.saturating_add(self.expiry) && unattended(&l, i));
                due.find_map(|i| deny(i).map(|o| (l.meta.id.clone(), i.id, o)))
            };
            let Some((id, item, option)) = due else {
                continue;
            };
            if let Err(e) = self.approve(&id, item, &option, None, EXPIRED) {
                cooee_core::note!("cooee: {id}: an expired approval's deny: {e}");
            }
        }
    }
}

fn unattended(live: &Live, card: &Item) -> bool {
    let open = matches!(
        &card.body,
        Body::Approval { tapped: None, chosen: None, plugin: None, card, .. } if card.form.is_none()
    );
    if !open {
        return false;
    }
    let sent = live.chat.items.iter().rev().find_map(|i| match &i.body {
        Body::You {
            random_id, routine, ..
        } if i.id < card.id => Some((random_id, routine)),
        _ => None,
    });
    let started = live.meta.routine.is_some();
    sent.map_or(started, |(random_id, routine)| {
        routine.is_some() || (started && *random_id == live.meta.random_id)
    })
}

fn deny(card: &Item) -> Option<String> {
    let Body::Approval { options, .. } = &card.body else {
        return None;
    };
    let reject = |kind: &str| options.iter().find(|o| o.kind == kind);
    reject("reject_once")
        .or_else(|| reject("reject_always"))
        .map(|o| o.id.clone())
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{BufRead, BufReader, Write},
        os::unix::net::UnixListener,
        path::Path,
        sync::{Arc, Mutex},
        thread,
    };

    use cooee_core::shed_proto::{Hello, Request, Response};
    use serde_json::{Value, json};

    use super::super::tests::{hub, scratch};
    use super::*;

    const MIN: u64 = 60_000;
    const EXPIRY: u64 = 30 * MIN;

    fn fake_shed(dir: &Path) -> Arc<Mutex<Vec<Value>>> {
        let sock = dir.join("no-shed-dir/shed.sock");
        fs::create_dir_all(sock.parent().unwrap()).unwrap();
        let listener = UnixListener::bind(sock).unwrap();
        let sent = Arc::new(Mutex::new(Vec::new()));
        let log = Arc::clone(&sent);
        thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                let log = Arc::clone(&log);
                thread::spawn(move || {
                    let mut writer = conn.try_clone().unwrap();
                    for line in BufReader::new(conn).lines().map_while(Result::ok) {
                        let req: Request = serde_json::from_str(&line).unwrap();
                        let result = match req.method.as_str() {
                            "hello" => json!(Hello::ours()),
                            "send" => {
                                log.lock().unwrap().push(req.params.clone());
                                json!({})
                            }
                            _ => json!(null),
                        };
                        let res = Response::ok(req.id.unwrap_or_default(), result);
                        if writeln!(writer, "{}", json!(res)).is_err() {
                            return;
                        }
                    }
                });
            }
        });
        sent
    }

    fn seed(dir: &Path, id: &str, meta: &Value, routine: Option<Value>, at: u64) {
        let mut thread =
            json!({ "id": id, "title": id, "messages": [], "terminal": format!("cooee-{id}") });
        thread
            .as_object_mut()
            .unwrap()
            .extend(meta.as_object().unwrap().clone());
        let threads = dir.join("data/threads");
        let file = json!({ "format": 1, "thread": thread });
        fs::write(threads.join(format!("{id}.json")), file.to_string()).unwrap();
        let mut you = json!({ "id": 1, "at": at, "kind": "you", "text": "go", "random_id": "r1" });
        if let Some(r) = routine {
            you["routine"] = r;
        }
        let option = |id: &str, kind: &str| json!({ "id": id, "name": id, "kind": kind });
        let ask = json!({ "id": 2, "at": at, "seq": 1, "kind": "approval", "tool": "Edit src/lib.rs",
            "options": [option("allow", "allow_once"), option("reject", "reject_once")] });
        fs::write(
            threads.join(format!("{id}.jsonl")),
            format!("{you}\n{ask}\n"),
        )
        .unwrap();
    }

    fn card(hub: &Hub, id: &str) -> Item {
        hub.slice(id, None, 10, 0).unwrap().items.pop().unwrap()
    }

    fn denials(sent: &Mutex<Vec<Value>>, id: &str) -> usize {
        let sent = sent.lock().unwrap();
        let deny = |p: &&Value| {
            p["name"] == format!("cooee-{id}") && p["text"] == "\u{1e}{\"approve\":\"reject\"}\n"
        };
        sent.iter().filter(deny).count()
    }

    #[test]
    fn a_routines_approval_expires_as_deny_and_the_owners_never_does() {
        let dir = scratch("expire");
        let sent = fake_shed(&dir);
        let t0 = super::super::now();
        let mark = json!({ "id": "r-1", "name": "nightly" });
        seed(&dir, "chat", &json!({}), Some(mark.clone()), t0);
        seed(
            &dir,
            "run",
            &json!({ "routine": mark, "random_id": "r1" }),
            None,
            t0,
        );
        seed(&dir, "mine", &json!({}), None, t0);
        seed(
            &dir,
            "early",
            &json!({}),
            Some(json!({ "id": "r-2", "name": "hourly" })),
            t0,
        );
        let hub = hub(&dir);
        hub.approve("early", 2, "allow", None, "phone").unwrap();
        hub.watch(t0 + EXPIRY - 1);
        assert_eq!(denials(&sent, "chat") + denials(&sent, "run"), 0);
        hub.watch(t0 + EXPIRY);
        hub.watch(t0 + 2 * EXPIRY);
        for id in ["chat", "run"] {
            assert_eq!(denials(&sent, id), 1, "{id}");
            let block = card(&hub, id).block().unwrap();
            let line = block["blocks"].as_array().unwrap().last().unwrap()["markdown"].clone();
            assert_eq!(line, "✓ reject · expired", "{id}");
        }
        assert_eq!(denials(&sent, "mine"), 0);
        assert!(matches!(
            card(&hub, "mine").body,
            Body::Approval { tapped: None, .. }
        ));
        assert_eq!(denials(&sent, "early"), 0);
        assert!(matches!(
            card(&hub, "early").body,
            Body::Approval { tapped: Some(t), card, .. } if t == "allow" && card.by.as_deref() == Some("phone")
        ));
    }
}
