use std::fs;

use serde_json::{Value, json};

use crate::setup::{options, setup, switches};
use crate::support::{Cooee, FakeShed, agent, scratch, version};

#[test]
fn a_persona_with_no_profile_and_a_room_member_start_without_personal_setup_plain_keeps_it() {
    let s = setup("persona-no-personal");
    agent(&s.dir, "ada", "name = \"Ada\"\nagent = \"fake-acp\"\n", "");
    let personal = |line: &str| {
        let o = options(&s.shed, line).unwrap_or_else(|| panic!("#416: no options in {line}"));
        assert_eq!(o["strictMcpConfig"], true, "{line}");
        assert_eq!(o["settingSources"], json!(["project", "local"]), "{line}");
    };

    let (_, line) = s.thread("ada", "hi");
    personal(&line);
    let (_, line) = s.thread("", "hi");
    assert_eq!(options(&s.shed, &line), None, "{line}");

    let room = json!({ "members": ["ada", "coder"], "random_id": "room" });
    let (status, body) = s.cooee.post("/rooms", &room);
    assert_eq!(status, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let said = json!({ "text": "hi all", "random_id": "said" });
    let (status, body) = s.cooee.post(&format!("/threads/{id}/send"), &said);
    assert_eq!(status, 200, "{body}");
    for member in ["ada", "coder"] {
        let label = format!("cooee.member={member}");
        let line = (0..200).find_map(|_| {
            let log = s.shed.cli_log();
            let line = log
                .lines()
                .find(|l| l.contains(&label) && options(&s.shed, l).is_some())
                .map(str::to_owned);
            line.or_else(|| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                None
            })
        });
        personal(&line.unwrap_or_else(|| panic!("{member}'s room session never started")));
    }
}

#[test]
fn a_persona_with_a_web_connector_starts_with_the_harnesss_own_web_tools_off() {
    let s = setup("persona-web");
    let pick = "name = \"Wes\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"search\"\n";
    agent(&s.dir, "wes", pick, "");

    let (_, line) = s.thread("wes", "hi");
    let o = options(&s.shed, &line).unwrap_or_else(|| panic!("no options in {line}"));
    let tools = o["tools"].as_array().unwrap();
    for web in ["WebSearch", "WebFetch"] {
        assert!(!tools.contains(&json!(web)), "{web}: {o}");
    }
    assert!(tools.contains(&json!("Read")), "{o}");
}

#[test]
fn a_persona_on_codex_or_antigravitys_default_login_starts_with_one_line_an_added_account_without()
{
    for base in ["codex", "antigravity"] {
        let dir = scratch(&format!("profiles-{base}"));
        fs::create_dir_all(dir.join("data/accounts/fake/work")).unwrap();
        fs::create_dir_all(dir.join("root")).unwrap();
        agent(
            &dir,
            "ada",
            "name = \"Ada\"\nagent = \"fake-acp\"\naccount = \"default\"\n",
            "Be brief.",
        );
        let work = "name = \"Wes\"\nagent = \"fake-acp\"\naccount = \"work\"\n";
        agent(&dir, "wes", work, "");
        let shed = FakeShed::start(&dir.join("shed"), version());
        let extra = format!(
            "[[providers]]\nname = \"fake\"\nbase = \"{base}\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\n\
             login = [\"true\"]\nstatus = [\"true\"]\n"
        );
        let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
        let start = |profile: &str| {
            let new = json!({ "agent": "fake-acp", "profile": profile, "folder": dir.join("root"),
                              "text": "hi", "random_id": profile });
            let (status, body) = cooee.post("/threads", &new);
            assert_eq!(status, 200, "{base}: {body}");
            let id = serde_json::from_str::<Value>(&body).unwrap()["id"].clone();
            let id = id.as_str().unwrap().to_owned();
            let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
            let notes: Vec<Value> = (items.as_array().unwrap().iter())
                .filter(|i| i["kind"] == "note")
                .map(|i| i["text"].clone())
                .collect();
            (id, notes)
        };
        let personal = cooee_core::provider::Gap::PersonalFiles.words();

        let (id, notes) = start("ada");
        assert_eq!(notes, [json!(personal)], "{base}");
        let sent = shed.sent(&format!("cooee-{id}")).concat();
        let meta = shed.meta(&format!("cooee-{id}"));
        switches(base, meta.as_ref(), &shed.adapter_env(), &sent);

        let (_, notes) = start("wes");
        assert_eq!(
            notes,
            Vec::<Value>::new(),
            "{base}: an added account has a clean folder"
        );
        drop(cooee);
        fs::remove_dir_all(&dir).ok();
    }
}
