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
        (&json!(""), &json!("fake-acp"), &json!("")),
        "{row}"
    );
}

fn the_coder_profile_is_applied_at_spawn(s: &Setup) {
    let (id, line) = s.thread("coder", "hi");
    let data = s.dir.join("data");
    let work = data.join("accounts/fake/work");
    let settings = s.dir.join("agents/coder/settings.json");
    assert!(line.contains(" --policy ask "), "{line}");
    let terminal = format!("cooee-{id}");
    let meta = s.shed.meta(&terminal).expect("#320: no personal setup");
    claude_switches(&meta, &format!("Follow the house rules.\n\n{PLAIN}"));
    let head = format!(
        " -- env FAKE_HOME={} FAKE_SETTINGS={} MODE=review sh -c ",
        work.display(),
        settings.display()
    );
    assert!(
        line.contains(&head) && line.ends_with(" fake-acp"),
        "{line}"
    );
    let env = s.shed.adapter_env();
    for var in [
        format!("FAKE_HOME={}", work.display()),
        format!("FAKE_SETTINGS={}", settings.display()),
        "MODE=review".to_owned(),
        format!("API_KEY={}", s.token),
    ] {
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
    for (method, params) in s.shed.requests.lock().unwrap().iter() {
        written.extend([method.clone(), params.to_string()]);
    }
    Setup::files(&data.join("threads"), &mut written);
    Setup::files(&data.join("accounts"), &mut written);
    assert!(!written.contains(&s.token), "a secret was written");
}

fn guarded_profile_with_a_hook(dir: &std::path::Path) -> std::path::PathBuf {
    let data = dir.join("data");
    fs::create_dir_all(data.join("accounts/claude/work")).unwrap();
    let claude = dir.join("agents/guarded/claude");
    fs::create_dir_all(&claude).unwrap();
    let guard = claude.join("guard.sh");
    fs::write(
        &guard,
        "#!/bin/sh\ngrep -q 'git push origin main' && { echo 'REFUSED MAIN_TARGET' >&2; exit 2; }\nexit 0\n",
    )
    .unwrap();
    let hooks = json!({ "hooks": { "PreToolUse": [
        { "matcher": "Bash", "hooks": [{ "type": "command", "command": guard }] } ] } });
    fs::write(claude.join("settings.json"), hooks.to_string()).unwrap();
    fs::write(
        dir.join("agents/guarded/agent.toml"),
        "name = \"Guarded\"\nagent = \"fake-acp\"\naccount = \"work\"\nsettings = \"claude\"\n",
    )
    .unwrap();
    claude
}

fn fake_acp_running_the_hook(shed: &FakeShed) {
    let adapter = shed.dir.join("fake-acp");
    fs::write(
        &adapter,
        "#!/bin/sh\nout=\"$(dirname \"$0\")/hook.out\"\n\
         hook=$(sed -n 's/.*\"command\":\"\\([^\"]*\\)\".*/\\1/p' \"$CLAUDE_CONFIG_DIR/settings.json\")\n\
         echo '{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"git push origin main\"}}' | sh \"$hook\" >\"$out\" 2>&1\n\
         echo \"exit $?\" >>\"$out\"\n",
    )
    .unwrap();
}

fn fake_acp_provider() -> String {
    let mut provider = cooee_core::provider::defaults().remove(0);
    "fake-acp".clone_into(&mut provider.agent);
    (provider.login, provider.status, provider.usage) = (
        vec!["true".to_owned()],
        vec!["true".to_owned()],
        vec!["true".to_owned()],
    );
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
fn a_claude_profiles_settings_folder_is_its_config_folder_and_its_hook_refuses_a_command() {
    let dir = scratch("profiles-claude");
    let claude = guarded_profile_with_a_hook(&dir);
    let shed = FakeShed::start(&dir.join("shed"), version());
    fake_acp_running_the_hook(&shed);
    let cooee = Cooee::start_with(&dir, &shed.dir, &fake_acp_provider());

    let new = json!({ "agent": "fake-acp", "profile": "guarded", "folder": dir.join("root"),
                      "text": "hi", "random_id": "claude-1" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let log = shed.cli_log();
    let line = log.lines().rfind(|l| l.contains(" run ")).unwrap();
    assert_eq!(
        line.matches("CLAUDE_CONFIG_DIR=").count(),
        1,
        "the settings folder replaces the account's: {line}"
    );
    assert!(
        line.contains(&format!(" -- env CLAUDE_CONFIG_DIR={} ", claude.display())),
        "{line}"
    );
    let out = fs::read_to_string(shed.dir.join("hook.out")).unwrap();
    assert_eq!(out, "REFUSED MAIN_TARGET\nexit 2\n");
}
