use std::{
    fs,
    path::Path,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::setup::{git, serving, setup};

#[test]
fn a_plugins_profile_starts_while_it_is_installed_and_its_add_confirm_names_it() {
    let s = setup("profiles-plugin");
    let repo = s.dir.join("plug.git");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    let hello = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/hello/server.mjs");
    fs::copy(hello, repo.join("server.mjs")).unwrap();
    fs::write(repo.join("tab.js"), "document.body.append('plug');\n").unwrap();
    fs::write(repo.join("brief.md"), "You are the conductor.\n").unwrap();
    fs::write(
        repo.join("cooee-plugin.toml"),
        "name = \"plug\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"TOKEN=$COOEE_PLUGIN_TOKEN exec node server.mjs\"\n\
         [[tabs]]\nname = \"plug\"\nlabel = \"Plug\"\nmodule = \"/tab.js\"\n\
         [[profiles]]\nname = \"conductor\"\nlabel = \"Conductor\"\nagent = \"fake-acp\"\ninstructions = \"brief.md\"\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "1"]);
    assert_eq!(s.start("plug/conductor", "go").0, 400, "not installed yet");

    let url = repo.to_string_lossy().into_owned();
    let out = s.cooee.cli(&["plugin", "add", &url, "--yes"]);
    assert!(
        out.contains("agent profiles: Conductor (fake-acp)"),
        "{out}"
    );
    s.backend_up("plug");
    let (id, _) = s.thread("plug/conductor", "go");
    let terminal = format!("cooee-{id}");
    assert_eq!(
        s.shed.sent(&terminal),
        [format!(
            "\x1e{}\n",
            json!({ "prompt": format!("{}go", s.topic("go")) })
        )]
    );
    let system = &s.shed.meta(&terminal).unwrap()["systemPrompt"]["append"];
    assert_eq!(
        system, "You are the conductor.",
        "#503: the brief is in the system channel"
    );
    assert_eq!(s.row(&id)["profile"], "plug/conductor");

    s.cooee.cli(&["plugin", "remove", "plug"]);
    assert_eq!(s.start("plug/conductor", "go").0, 400);
}

#[test]
fn a_backend_serves_its_plugins_agents_a_hidden_one_is_refused_and_thread_is_the_threads_id() {
    let s = setup("profiles-served");
    let repo = s.dir.join("plug.git");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    let (list, log) = (s.dir.join("served.json"), s.dir.join("started.log"));
    let served = json!([
        { "name": "fleet-a", "label": "Fleet A", "agent": "fake-acp",
          "env": { "HERDSMAN_THREAD": "{thread}", "FLEET": "a" } },
        { "name": "fleet-b", "label": "Fleet B", "agent": "fake-acp", "hidden": true,
          "why": "its guard can't be proven" },
    ]);
    fs::write(&list, served.to_string()).unwrap();
    fs::write(repo.join("server.mjs"), serving(&list, &log)).unwrap();
    fs::write(
        repo.join("cooee-plugin.toml"),
        "name = \"plug\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"TOKEN=$COOEE_PLUGIN_TOKEN exec node server.mjs\"\n\
         [[tabs]]\nname = \"plug\"\nlabel = \"Plug\"\nmodule = \"/tab.js\"\n\
         [[profiles]]\nname = \"old\"\nlabel = \"Old\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "1"]);
    s.cooee
        .cli(&["plugin", "add", &repo.to_string_lossy(), "--yes"]);

    s.backend_up("plug");
    let runs = s.shed.cli_log().matches(" run ").count();
    let until = Instant::now() + Duration::from_secs(20);
    let (mut status, mut body) = s.start("plug/fleet-b", "go");
    while !body.contains("hidden") && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
        (status, body) = s.start("plug/fleet-b", "go");
    }
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("hidden: its guard can't be proven"), "{body}");
    assert_eq!(s.start("plug/old", "go").0, 400);
    assert_eq!(s.shed.cli_log().matches(" run ").count(), runs);
    assert!(!log.exists(), "nothing started");

    let (id, _) = s.thread("plug/fleet-a", "go");
    let env = s.shed.adapter_env();
    for var in [format!("HERDSMAN_THREAD={id}"), "FLEET=a".to_owned()] {
        assert!(env.lines().any(|l| l == var), "{var} not in {env}");
    }
    let told: Value = serde_json::from_str(fs::read_to_string(&log).unwrap().trim()).unwrap();
    assert_eq!(told, json!({ "thread": id, "profile": "fleet-a" }));

    s.cooee.cli(&["plugin", "remove", "plug"]);
}

#[test]
fn a_hanging_backend_never_slows_a_start_which_takes_its_last_answered_profiles() {
    let s = setup("profiles-hang");
    let repo = s.dir.join("plug.git");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    let (list, log) = (s.dir.join("served.json"), s.dir.join("started.log"));
    let (asked, hang) = (
        s.dir.join("served.json.asked"),
        s.dir.join("served.json.hang"),
    );
    let serve = |name: &str| {
        let v = json!([{ "name": name, "label": name, "agent": "fake-acp" }]);
        fs::write(&list, v.to_string()).unwrap();
    };
    fs::write(repo.join("server.mjs"), serving(&list, &log)).unwrap();
    fs::write(
        repo.join("cooee-plugin.toml"),
        "name = \"plug\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"TOKEN=$COOEE_PLUGIN_TOKEN exec node server.mjs\"\n\
         [[tabs]]\nname = \"plug\"\nlabel = \"Plug\"\nmodule = \"/tab.js\"\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "--quiet", "-m", "1"]);
    let hung_twice = |shown: Option<&str>, not: &str| {
        let count = || fs::read_to_string(&asked).map_or(0, |t| t.len());
        let (from, until) = (count(), Instant::now() + Duration::from_secs(30));
        let mut took = Vec::new();
        let mut timed = |profile: &str| {
            let at = Instant::now();
            let (status, body) = s.start(profile, "go");
            took.push(at.elapsed());
            assert!(took[took.len() - 1] < Duration::from_secs(2), "{took:?}");
            (status, body)
        };
        while count() < from + 2 {
            assert!(Instant::now() < until, "never asked twice");
            assert_eq!(timed(not).0, 400, "{not} before its answer");
            std::thread::sleep(Duration::from_millis(50));
        }
        if let Some(shown) = shown {
            let (status, body) = timed(shown);
            assert_eq!(status, 200, "{shown}, the last answer: {body}");
        }
        took.sort();
        assert!(
            took[took.len() / 2] < Duration::from_millis(200),
            "{took:?}"
        );
    };
    let starts = |profile: &str| {
        let at = Instant::now();
        while s.start(profile, "go").0 != 200 {
            assert!(
                at.elapsed() < Duration::from_secs(20),
                "{profile} never starts"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        at.elapsed()
    };

    fs::write(&hang, "").unwrap();
    serve("fleet-a");
    s.cooee
        .cli(&["plugin", "add", &repo.to_string_lossy(), "--yes"]);
    hung_twice(None, "plug/fleet-a");
    fs::remove_file(&hang).unwrap();
    starts("plug/fleet-a");

    fs::write(&hang, "").unwrap();
    serve("fleet-z");
    hung_twice(Some("plug/fleet-a"), "plug/fleet-z");
    fs::remove_file(&hang).unwrap();
    let took = starts("plug/fleet-z");
    assert!(took < Duration::from_secs(11), "{took:?}");

    s.cooee.cli(&["plugin", "remove", "plug"]);
}
