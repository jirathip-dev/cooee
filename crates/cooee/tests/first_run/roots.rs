use std::fs;

use serde_json::json;

use crate::support::{Cooee, FakeShed, scratch, version};

#[test]
fn with_one_root_and_nothing_in_it_a_thread_starts_in_the_root() {
    let dir = scratch("first-run-root");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let root = dir.join("only");
    fs::create_dir_all(&root).unwrap();
    let cooee = Cooee::start_with(
        &dir,
        &shed.dir,
        &format!("roots = [{:?}]\n", root.to_string_lossy()),
    );

    assert_eq!(cooee.json("/folders")["roots"], json!([root]));
    let new = json!({ "agent": "fake-acp", "folder": root, "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let cwd = format!("--cwd {} ", root.canonicalize().unwrap().display());
    assert!(shed.cli_log().contains(&cwd), "{}", shed.cli_log());
}

#[test]
fn with_no_roots_a_chat_still_starts_in_scratch_and_use_a_folder_is_refused() {
    let dir = scratch("first-run-noroots");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, "roots = []\n");
    assert_eq!(cooee.json("/folders")["roots"], json!([]));
    let new = json!({ "agent": "fake-acp", "text": "hi", "random_id": "r0" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"].clone();
    let own = dir.join("data/scratch").join(id.as_str().unwrap());
    let cwd = format!("--cwd {} ", own.display());
    assert!(shed.cli_log().contains(&cwd), "{}", shed.cli_log());
    let attach = json!({ "folder": dir });
    let (status, body) = cooee.post(
        &format!("/threads/{}/folder", id.as_str().unwrap()),
        &attach,
    );
    assert!(
        status == 400 && body.contains("not inside a configured root"),
        "{status} {body}"
    );
}
