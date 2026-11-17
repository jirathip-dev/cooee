use super::*;
use std::{env, process::Stdio, thread};

fn scratch(name: &str) -> PathBuf {
    let dir = crate::scratch::owned(
        env::temp_dir().join(format!("cooee-store-{name}-{}", std::process::id())),
    );
    fs::remove_dir_all(&dir).ok();
    dir
}

fn sample(id: &str, title: &str) -> Thread {
    let text = "สวัสดีครับ ".repeat(200);
    Thread {
        id: id.into(),
        title: title.into(),
        messages: vec![Message {
            role: Role::User,
            text,
        }],
        ..Thread::default()
    }
}

#[test]
fn save_then_load_round_trips_and_leaves_no_temp() {
    let dir = scratch("round-trip");
    let store = Store::open(&dir).unwrap();
    store.save(&sample("a", "first")).unwrap();
    store.save(&sample("a", "second")).unwrap();
    assert_eq!(store.load("a").unwrap(), sample("a", "second"));
    assert_eq!(store.ids().unwrap(), ["a"]);
    assert!(!dir.join("threads/a.json.tmp").exists());
}

fn item(id: u64, kind: &str, text: &str) -> Item {
    let body = match kind {
        "you" => serde_json::json!({ "text": text, "random_id": "r" }),
        "agent" => serde_json::json!({ "text": text, "stop_reason": "end_turn" }),
        _ => serde_json::json!({ "call": "c", "title": text, "status": "completed" }),
    };
    let mut v = serde_json::json!({ "id": id, "at": id, "kind": kind });
    v.as_object_mut()
        .unwrap()
        .extend(body.as_object().unwrap().clone());
    serde_json::from_value(v).unwrap()
}

fn ids(items: &[Item]) -> Vec<u64> {
    items.iter().map(|i| i.id).collect()
}

#[test]
fn items_replace_by_id_and_page_from_either_end() {
    let dir = scratch("items");
    let store = Store::open(&dir).unwrap();
    assert_eq!(ids(&store.items("a").unwrap()), [0; 0]);
    for id in [1, 3, 2, 4, 5] {
        store.append("a", &item(id, "tool", "pending")).unwrap();
    }
    store.append("a", &item(2, "tool", "done")).unwrap();
    store.append("b", &item(1, "you", "other")).unwrap();
    let all = store.items("a").unwrap();
    assert_eq!(ids(&all), [1, 2, 3, 4, 5]);
    assert_eq!(all[1], item(2, "tool", "done"));
    assert_eq!(ids(&store.page("a", Page::Last, 2).unwrap()), [4, 5]);
    assert_eq!(ids(&store.page("a", Page::Before(4), 2).unwrap()), [2, 3]);
    assert_eq!(ids(&store.page("a", Page::After(1), 2).unwrap()), [2, 3]);
    assert_eq!(ids(&store.page("a", Page::After(5), 2).unwrap()), [0; 0]);
    store.remove("a").unwrap();
    assert_eq!(ids(&store.items("a").unwrap()), [0; 0]);
    assert_eq!(store.items("b").unwrap().len(), 1);
    assert!(store.append("../x", &item(1, "you", "x")).is_err());
}

#[test]
fn the_index_holds_message_text_only_and_finds_thai() {
    let dir = scratch("index");
    let store = Store::open(&dir).unwrap();
    store
        .append("a", &item(1, "you", "ช่วยหาสูตรต้มยำกุ้ง"))
        .unwrap();
    store
        .append("a", &item(2, "tool", "cat ต้มยำ.txt secret"))
        .unwrap();
    store
        .append("a", &item(3, "agent", "สูตรต้มยำกุ้งน้ำข้น"))
        .unwrap();
    store
        .append("a", &item(3, "agent", "สูตรต้มยำกุ้งน้ำใส"))
        .unwrap();
    let found = |words: &[&str]| {
        let words: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
        let mut got = Vec::new();
        store
            .said(&words, |t, i| {
                got.push((t.to_owned(), i.id));
                true
            })
            .unwrap();
        got
    };
    assert_eq!(found(&[]), [("a".to_owned(), 3), ("a".to_owned(), 1)]);
    assert_eq!(found(&["ต้มยำ"]), [("a".to_owned(), 3), ("a".to_owned(), 1)]);
    assert_eq!(found(&["น้ำใส"]), [("a".to_owned(), 3)]);
    assert!(
        found(&["น้ำข้น"]).is_empty(),
        "the replaced version left the index"
    );
    assert!(found(&["secret"]).is_empty(), "tool output is not indexed");
}

#[test]
fn an_items_file_imports_once_and_skips_a_torn_line() {
    let dir = scratch("import");
    fs::create_dir_all(dir.join("threads")).unwrap();
    let lines: String = [
        item(1, "you", "hi"),
        item(2, "tool", "a"),
        item(2, "tool", "b"),
    ]
    .iter()
    .map(|i| serde_json::to_string(i).unwrap() + "\n")
    .collect();
    fs::write(dir.join("threads/a.jsonl"), lines + "{\"torn").unwrap();
    let want = [item(1, "you", "hi"), item(2, "tool", "b")];
    assert_eq!(Store::open(&dir).unwrap().items("a").unwrap(), want);
    assert!(!dir.join("threads/a.jsonl").exists());
    assert!(dir.join("threads/a.jsonl.imported").exists());
    let store = Store::open(&dir).unwrap();
    assert_eq!(store.items("a").unwrap(), want);
    store.remove("a").unwrap();
    assert!(!dir.join("threads/a.jsonl.imported").exists());
}

fn crash(child: &str, dir: &Path) {
    let status = crate::runner::command(env::current_exe().unwrap())
        .args(["--exact", child, "--ignored", "--test-threads=1"])
        .env("COOEE_CRASH_DIR", dir)
        .stdout(Stdio::null())
        .status()
        .unwrap();
    assert!(!status.success(), "{child} must die mid-write");
}

fn crash_dir() -> Option<PathBuf> {
    crate::env_var("COOEE_CRASH_DIR").map(PathBuf::from)
}

#[test]
fn a_kill_mid_import_keeps_the_files_and_imports_again() {
    let dir = scratch("crash-import");
    fs::create_dir_all(dir.join("threads")).unwrap();
    for t in ["a", "b"] {
        let lines: String = (1..=50)
            .map(|n| serde_json::to_string(&item(n, "you", t)).unwrap() + "\n")
            .collect();
        fs::write(dir.join(format!("threads/{t}.jsonl")), lines).unwrap();
    }
    crash("store::tests::crash_import_child", &dir);
    assert!(dir.join("threads/a.jsonl").exists() && dir.join("threads/b.jsonl").exists());
    let store = Store::open(&dir).unwrap();
    for t in ["a", "b"] {
        let want: Vec<Item> = (1..=50).map(|n| item(n, "you", t)).collect();
        assert_eq!(store.items(t).unwrap(), want, "{t}");
    }
}

#[test]
#[ignore = "the child process of a_kill_mid_import_keeps_the_files_and_imports_again"]
fn crash_import_child() {
    let Some(dir) = crash_dir() else { return };
    let threads = dir.join("threads");
    let (a, b) = (threads.join("a.jsonl"), threads.join("b.jsonl"));
    let (aside_a, aside_b) = (dir.join("a.jsonl"), dir.join("b.jsonl"));
    fs::rename(&a, &aside_a).unwrap();
    fs::rename(&b, &aside_b).unwrap();
    let store = Store::open(&dir).unwrap();
    fs::rename(&aside_a, &a).unwrap();
    fs::rename(&aside_b, &b).unwrap();
    let mut db = store.db();
    let tx = db.transaction().unwrap();
    for n in 1..=50 {
        store.put(&tx, "a", &item(n, "you", "a")).unwrap();
    }
    tx.commit().unwrap();
    let tx = db.transaction().unwrap();
    for n in 1..=25 {
        store.put(&tx, "b", &item(n, "you", "b")).unwrap();
    }
    std::process::abort();
}

#[test]
fn a_kill_mid_append_keeps_every_acknowledged_item() {
    let dir = scratch("crash-append");
    crash("store::tests::crash_append_child", &dir);
    let store = Store::open(&dir).unwrap();
    let want: Vec<Item> = (1..=100).map(|n| item(n, "agent", "kept")).collect();
    assert_eq!(store.items("a").unwrap(), want);
}

#[test]
#[ignore = "the child process of a_kill_mid_append_keeps_every_acknowledged_item"]
fn crash_append_child() {
    let Some(dir) = crash_dir() else { return };
    let store = Store::open(&dir).unwrap();
    for n in 1..=100 {
        store.append("a", &item(n, "agent", "kept")).unwrap();
    }
    let mut db = store.db();
    let tx = db.transaction().unwrap();
    store.put(&tx, "a", &item(101, "agent", "torn")).unwrap();
    std::process::abort();
}

#[test]
fn bad_ids_and_newer_formats_are_refused() {
    let dir = scratch("refused");
    let store = Store::open(&dir).unwrap();
    for id in ["", "../x", "a/b", "a.json", &"x".repeat(65)] {
        assert!(
            matches!(store.save(&sample(id, "t")), Err(StoreError::InvalidId(_))),
            "{id:?}"
        );
    }
    fs::write(
        dir.join("threads/n.json"),
        r#"{"format":3,"thread":{"id":"n","future":true}}"#,
    )
    .unwrap();
    assert!(matches!(
        store.load("n"),
        Err(StoreError::Newer { found: 3, .. })
    ));
}

#[test]
fn concurrent_saves_of_one_thread_never_tear() {
    let dir = scratch("concurrent");
    let store = Store::open(&dir).unwrap();
    thread::scope(|s| {
        for i in 0..8 {
            let store = &store;
            s.spawn(move || {
                for j in 0..25 {
                    store.save(&sample("c", &format!("{i}-{j}"))).unwrap();
                }
            });
        }
    });
    assert_eq!(store.load("c").unwrap().messages, sample("c", "").messages);
}

#[test]
fn crash_between_temp_write_and_rename_keeps_old_file() {
    let dir = scratch("crash");
    let store = Store::open(&dir).unwrap();
    store.save(&sample("a", "old")).unwrap();
    crash("store::tests::crash_child", &dir);
    assert!(
        dir.join("threads/a.json.tmp").exists(),
        "the child got as far as the temp file"
    );
    assert_eq!(store.load("a").unwrap(), sample("a", "old"));
    assert_eq!(store.ids().unwrap(), ["a"]);
    store.save(&sample("a", "new")).unwrap();
    assert_eq!(store.load("a").unwrap(), sample("a", "new"));
}

#[test]
#[ignore = "the child process of crash_between_temp_write_and_rename_keeps_old_file"]
fn crash_child() {
    let Some(dir) = crash_dir() else { return };
    let store = Store::open(&dir).unwrap();
    store.stage(&sample("a", "new")).unwrap();
    std::process::abort();
}

const DAY: u64 = 86_400_000;

fn at(mut item: Item, at: u64) -> Item {
    item.at = at;
    item
}

fn history(toml: &str) -> crate::config::History {
    toml::from_str::<crate::Config>(toml).unwrap().history
}

fn hot_rows(store: &Store, id: &str) -> i64 {
    store
        .db()
        .query_row("SELECT COUNT(*) FROM items WHERE thread = ?1", [id], |r| {
            r.get(0)
        })
        .unwrap()
}

fn seed_ended(store: &Store, id: &str, closed: bool, last: u64) -> Vec<Item> {
    store
        .save(&Thread {
            closed,
            ..sample(id, id)
        })
        .unwrap();
    let items = vec![
        at(item(1, "you", &format!("archived question {id}")), last - 2),
        at(item(2, "tool", "ls"), last - 1),
        at(item(3, "agent", "the archived answer"), last),
    ];
    for i in &items {
        store.append(id, i).unwrap();
    }
    items
}

#[test]
fn a_thread_past_hot_days_is_archived_and_reopens_with_every_item() {
    let dir = scratch("retain-move");
    let store = Store::open(&dir).unwrap();
    let old = seed_ended(&store, "old", true, 40 * DAY);
    let open = seed_ended(&store, "open", false, 40 * DAY);
    let recent = seed_ended(&store, "recent", true, 60 * DAY);
    let now = 75 * DAY;
    assert_eq!(store.retain(now, &history("")).unwrap(), 3);
    assert_eq!(store.retain(now, &history("")).unwrap(), 0);
    let zst = fs::read(dir.join("archive/threads-1970-02.jsonl.zst")).unwrap();
    assert_eq!(zst.get(..4), Some(&[0x28, 0xb5, 0x2f, 0xfd][..]));
    assert_eq!(
        (
            hot_rows(&store, "old"),
            hot_rows(&store, "open"),
            hot_rows(&store, "recent")
        ),
        (0, 3, 3)
    );
    drop(store);
    let store = Store::open(&dir).unwrap();
    assert_eq!(store.items("old").unwrap(), old);
    assert_eq!(store.items("open").unwrap(), open);
    assert_eq!(store.items("recent").unwrap(), recent);
    assert_eq!(ids(&store.page("old", Page::Last, 2).unwrap()), [2, 3]);
    assert_eq!(ids(&store.page("old", Page::Before(3), 1).unwrap()), [2]);
    assert_eq!(ids(&store.page("old", Page::After(1), 5).unwrap()), [2, 3]);
    let read = history::read(&store, "old", None).unwrap();
    assert!(read.contains("owner: archived question old"), "{read}");
    let found = history::search(&store, &["old".to_owned()], "archived question", 5).unwrap();
    assert!(found.starts_with("old 1970-02-"), "{found}");
    let more = at(item(4, "you", "back again"), now);
    store.append("old", &more).unwrap();
    let mut all = old;
    all.push(more);
    assert_eq!(store.items("old").unwrap(), all);
}

#[test]
fn a_trimmed_tool_item_shows_its_head_and_tail_and_its_whole_text_comes_back() {
    let dir = scratch("retain-trim");
    let store = Store::open(&dir).unwrap();
    let long = format!("HEAD{}TAIL", "ก".repeat(5000));
    let tool = at(item(1, "tool", &long), DAY);
    let short = at(item(2, "tool", "ls"), DAY);
    let said = at(item(3, "agent", &long), DAY);
    for i in [&tool, &short, &said] {
        store.append("t", i).unwrap();
    }
    assert_eq!(store.retain(9 * DAY, &history("")).unwrap(), 1);
    assert_eq!(store.retain(9 * DAY, &history("")).unwrap(), 0);
    let items = store.items("t").unwrap();
    let Some(Item {
        body: crate::Body::Tool { title, .. },
        ..
    }) = items.first()
    else {
        panic!("{items:?}");
    };
    assert!(
        title.starts_with("HEADก") && title.ends_with("กTAIL"),
        "{title}"
    );
    assert!(
        title.contains("trimmed") && title.len() < 4096,
        "{}",
        title.len()
    );
    assert_eq!(items.get(1..), Some(&[short.clone(), said.clone()][..]));
    assert_eq!(store.whole("t").unwrap(), [tool, short, said]);
    assert!(dir.join("archive/threads-1970-01.jsonl.zst").is_file());
}

#[test]
fn changing_a_key_changes_the_behaviour() {
    let dir = scratch("retain-keys");
    let store = Store::open(&dir).unwrap();
    seed_ended(&store, "a", true, 40 * DAY);
    let long = at(item(1, "tool", &"x".repeat(3000)), 40 * DAY);
    store.append("b", &long).unwrap();
    let now = 75 * DAY;
    let keep = "[history]\nhot_days = 40\ntrim_after_days = 40\ntrim_over_kb = 3\n";
    assert_eq!(store.retain(now, &history(keep)).unwrap(), 0);
    assert_eq!(
        store
            .retain(now, &history("[history]\nhot_days = 30"))
            .unwrap(),
        3
    );
    assert_eq!(
        store
            .retain(now, &history("[history]\ntrim_over_kb = 2"))
            .unwrap(),
        1
    );
    assert_eq!(hot_rows(&store, "a"), 0);
    assert_eq!(store.whole("b").unwrap(), [long]);
    assert!(toml::from_str::<crate::Config>("[history]\nhot_dayz = 1").is_err());
}
