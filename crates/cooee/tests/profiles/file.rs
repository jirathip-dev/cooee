use std::{fmt::Write as _, fs};

use cooee_core::agents::PLAIN;
use serde_json::json;

use crate::setup::{Setup, claude_switches, setup};
use crate::support::{Cooee, FakeShed, scratch, version};

fn a_plain_thread_adds_nothing(s: &Setup) {
    let (id, line) = s.thread("", "hi");
    assert!(
        !line.contains(" -- ") && !line.contains("--policy") && !line.contains("--meta"),
        "{line}"
    );
    assert_eq!(s.shed.adapter_env(), "", "no adapter command to run");
    let terminal = format!("cooee-{id}");
    assert_eq!(s.shed.sent(&terminal), ["\x1e{\"prompt\":\"hi\"}\n"]);
    let row = s.row(&id);
    assert_eq!(
        (&row["profile"], &row["agent"], &row["account"]),
        (&json!(""), &json!("fake-acp"), &json!("auto")),
        "{row}"
    );
}

fn the_coder_profile_is_applied_at_spawn(s: &Setup) {
    let (id, line) = s.thread("coder", "hi");
    let data = s.dir.join("data");
    assert!(line.contains(" --policy ask --account work "), "{line}");
    let terminal = format!("cooee-{id}");
    let meta = s.shed.meta(&terminal).expect("#320: no personal setup");
    claude_switches(&meta, &format!("Follow the house rules.\n\n{PLAIN}"));
    assert_eq!(
        meta["fakeSettings"],
        json!({ "hooks": { "PreToolUse": [] } }),
        "{meta}"
    );
    assert!(!line.contains(" -- ") && !line.contains(&s.token), "{line}");
    let env = s.shed.adapter_env();
    for var in ["MODE=review".to_owned(), format!("API_KEY={}", s.token)] {
        assert!(env.lines().any(|l| l == var), "{var} not in {env}");
    }
    let left: Vec<_> = fs::read_dir(data.join("secrets"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(left, ["API_KEY"], "the secrets' file is gone");
    assert_eq!(
        s.shed.sent(&terminal),
        [format!(
            "\x1e{}\n",
            json!({ "prompt": format!("{}hi", s.topic("hi")) })
        )]
    );
    let items = s.cooee.json(&format!("/threads/{id}/messages"));
    assert_eq!(items["items"][0]["text"], "hi", "{items}");
    let row = s.row(&id);
    assert_eq!(
        (&row["profile"], &row["account"]),
        (&json!("coder"), &json!("work")),
        "{row}"
    );
}

#[test]
fn plain_adds_nothing_a_file_profile_is_applied_at_spawn_a_guard_that_cant_run_is_refused() {
    let s = setup("profiles");

    a_plain_thread_adds_nothing(&s);

    the_coder_profile_is_applied_at_spawn(&s);
    let data = s.dir.join("data");

    let runs = s.shed.cli_log().matches(" run ").count();
    let (status, body) = s.start("guard", "hi");
    assert_eq!(status, 400, "{body}");
    assert!(
        body.contains("this profile's guard can't run with other-acp"),
        "{body}"
    );
    assert_eq!(s.shed.cli_log().matches(" run ").count(), runs);
    assert_eq!(
        s.cooee.json("/threads").as_array().unwrap().len(),
        2,
        "two threads, the refused start none"
    );
    for id in ["nope", "leak"] {
        assert_eq!(s.start(id, "hi").0, 400, "{id}");
    }

    let mut written = fs::read_to_string(s.dir.join("cooee.err")).unwrap();
    assert!(
        written.contains("agents/leak/agent.toml"),
        "the left-out profile is logged: {written}"
    );
    written.push_str(&s.shed.cli_log());
    written.push_str(&fs::read_to_string(&s.cooee.config).unwrap());
    for (method, mut params) in s.shed.requests.lock().unwrap().clone() {
        params.as_object_mut().map(|p| p.remove("env"));
        written.extend([method, params.to_string()]);
    }
    Setup::files(&data.join("threads"), &mut written);
    Setup::files(&data.join("accounts"), &mut written);
    assert!(!written.contains(&s.token), "a secret was written");
}

fn guarded_profile_with_a_hook(dir: &std::path::Path) -> serde_json::Value {
    let claude = dir.join("agents/guarded/claude");
    fs::create_dir_all(&claude).unwrap();
    let hooks = json!({ "PreToolUse": [
        { "matcher": "Bash", "hooks": [{ "type": "command", "command": "/x/guard.sh" }] } ] });
    fs::write(
        claude.join("settings.json"),
        json!({ "hooks": hooks }).to_string(),
    )
    .unwrap();
    fs::write(
        dir.join("agents/guarded/agent.toml"),
        "name = \"Guarded\"\nagent = \"fake-acp\"\naccount = \"work\"\nsettings = \"claude\"\n",
    )
    .unwrap();
    hooks
}

fn fake_acp_provider() -> String {
    let mut provider = cooee_core::provider::defaults().remove(0);
    "fake-acp".clone_into(&mut provider.agent);
    let mut extra = "[[providers]]\n".to_owned();
    for (k, v) in serde_json::to_value(&provider)
        .unwrap()
        .as_object()
        .unwrap()
    {
        writeln!(extra, "{k} = {v}").unwrap();
    }
    extra
}

#[test]
fn a_claude_profiles_settings_go_in_its_session_meta_never_in_the_account_folder_variable() {
    let dir = scratch("profiles-claude");
    let hooks = guarded_profile_with_a_hook(&dir);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, &fake_acp_provider());

    let new = json!({ "agent": "fake-acp", "profile": "guarded", "folder": dir.join("root"),
                      "text": "hi", "random_id": "claude-1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let log = shed.cli_log();
    let line = log.lines().rfind(|l| l.contains(" run ")).unwrap();
    assert!(line.contains(" --account work "), "{line}");
    assert!(!line.contains("CLAUDE_CONFIG_DIR"), "{line}");
    let terminal = line
        .split(" --name ")
        .nth(1)
        .unwrap()
        .split(' ')
        .next()
        .unwrap();
    let meta = shed.meta(terminal).unwrap();
    let options = &meta["claudeCode"]["options"];
    assert_eq!(options["settings"]["hooks"], hooks, "{meta}");
    assert_eq!(
        options["strictMcpConfig"], true,
        "the persona's own options stay: {meta}"
    );
    assert_eq!(
        options["settings"]["permissions"]["defaultMode"], "default",
        "{meta}"
    );
}
