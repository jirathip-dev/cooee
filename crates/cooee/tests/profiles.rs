//! Agent profiles end to end with the fake shed (tests/support), whose CLI runs the adapter command a start gives
//! it: Plain starts the agent with nothing added; a file profile's account, settings, env (its secret through the
//! adapter's shell), policy and instructions are applied at spawn; a plugin's profile is listed while the plugin is
//! installed and gone once it is removed; a guard its agent can't run refuses the start; no secret reaches cooee's
//! config, files or log, or what it tells shed.

mod support;

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    /// A secret-shaped value, built at runtime: `secret:API_KEY`.
    token: String,
}

/// cooee with provider `fake` for `fake-acp` (accounts in `FAKE_HOME`, settings in `FAKE_SETTINGS`), its account
/// `work`, the secret `API_KEY`, and in `<config>/profiles/`: `coder` (all of it), `guard` (settings for an agent
/// with no settings variable) and `leak` (a secret inline).
fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let token = format!("sk-{}", "p4".repeat(16));
    let data = dir.join("data");
    fs::create_dir_all(data.join("accounts/fake/work")).unwrap();
    fs::create_dir_all(data.join("secrets")).unwrap();
    fs::write(data.join("secrets/API_KEY"), format!("{token}\n")).unwrap();
    let profiles = dir.join("profiles");
    fs::create_dir_all(&profiles).unwrap();
    fs::write(profiles.join("coder.md"), "Follow the house rules.\n").unwrap();
    fs::write(
        profiles.join("settings.json"),
        r#"{"hooks":{"PreToolUse":[]}}"#,
    )
    .unwrap();
    let profile = |file: &str, text: &str| fs::write(profiles.join(file), text).unwrap();
    profile(
        "coder.toml",
        "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\naccount = \"work\"\n\
         instructions = \"coder.md\"\nsettings = \"settings.json\"\npolicy = \"allow\"\n\
         env = { MODE = \"review\", API_KEY = \"secret:API_KEY\" }\n",
    );
    profile(
        "guard.toml",
        "name = \"guard\"\nlabel = \"Guarded\"\nagent = \"other-acp\"\nsettings = \"settings.json\"\n",
    );
    profile(
        "leak.toml",
        &format!(
            "name = \"leak\"\nlabel = \"Leak\"\nagent = \"fake-acp\"\nenv = {{ T = \"{token}\" }}\n"
        ),
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = "[[providers]]\nname = \"fake\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\n\
                 settings = \"FAKE_SETTINGS\"\nlogin = [\"true\"]\nstatus = [\"true\"]\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, extra);
    Setup {
        dir,
        shed,
        cooee,
        token,
    }
}

impl Setup {
    /// `POST /threads` in the root with `profile`: the status and body.
    fn start(&self, profile: &str, text: &str) -> (u16, String) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new = json!({ "agent": "fake-acp", "profile": profile, "folder": self.dir.join("root"),
                          "text": text, "random_id": random });
        self.cooee.post("/threads", &new)
    }

    /// A started thread's id and its `shed run` line.
    fn thread(&self, profile: &str, text: &str) -> (String, String) {
        let (status, body) = self.start(profile, text);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let log = self.shed.cli_log();
        let line = log.lines().rfind(|l| l.contains(" run ")).unwrap();
        assert!(line.contains(&format!("cooee-{id}")), "{line}");
        (id, line.to_owned())
    }

    /// The sidebar's agents (#86): Plain on each agent, then each profile.
    fn picker(&self) -> String {
        let page = self.cooee.get("/").1;
        let at = page.find(r#"<nav class="ags""#).expect("the agents");
        page[at..at + page[at..].find("</nav>").unwrap()].to_owned()
    }

    /// Every file under `dir`, as text.
    fn files(dir: &Path, out: &mut String) {
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.file_type().unwrap().is_dir() {
                Self::files(&e.path(), out);
            } else {
                out.push_str(&String::from_utf8_lossy(&fs::read(e.path()).unwrap()));
            }
        }
    }
}

#[test]
fn plain_adds_nothing_a_file_profile_is_applied_at_spawn_a_guard_that_cant_run_is_refused() {
    let s = setup("profiles");
    let picker = s.picker();
    assert!(
        picker.contains(r#"href="/new?agent=fake-acp""#)
            && picker.contains(r#"<span class="tt">Fake</span>"#)
            && picker.contains(r#"href="/new?profile=coder""#)
            && picker.contains(r#"<span class="tt">Coder</span>"#)
            && picker.contains(r#"<span class="tt">Guarded</span>"#)
            && !picker.contains("Leak"),
        "{picker}"
    );

    // Plain: the agent as it is.
    let (id, line) = s.thread("", "hi");
    assert!(
        !line.contains(" -- ") && !line.contains("--policy"),
        "{line}"
    );
    assert_eq!(s.shed.adapter_env(), "", "no adapter command to run");
    let terminal = format!("cooee-{id}");
    assert_eq!(s.shed.sent(&terminal), ["\x1e{\"prompt\":\"hi\"}\n"]);
    let chat = s.cooee.get(&format!("/t/{id}")).1;
    assert!(
        chat.contains(
            "<i class=\"dt\"></i><span id=\"net\"></span>Plain · fake-acp · System default · "
        ),
        "{chat}"
    );

    // A file profile: its account, settings, env and policy on the adapter, its secret through the adapter's shell,
    // its instructions ahead of the first message (stored alone).
    let (id, line) = s.thread("coder", "hi");
    let data = s.dir.join("data");
    let work = data.join("accounts/fake/work");
    let settings = s.dir.join("profiles/settings.json");
    assert!(line.contains(" --policy allow "), "{line}");
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
    let terminal = format!("cooee-{id}");
    assert_eq!(
        s.shed.sent(&terminal),
        ["\x1e{\"prompt\":\"Follow the house rules.\\n\\nhi\"}\n"]
    );
    let items = s.cooee.json(&format!("/threads/{id}/messages"));
    assert_eq!(items["items"][0]["text"], "hi", "{items}");
    let chat = s.cooee.get(&format!("/t/{id}")).1;
    assert!(chat.contains("Coder · fake-acp · work · "), "{chat}");
    // Coder's ongoing chat is this one.
    assert!(s.picker().contains(&format!(r#"href="/t/{id}""#)));

    // A guard its agent can't run: refused, nothing started.
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
    // An unknown profile, and one left out for a secret inline, are refused too.
    for id in ["nope", "leak"] {
        assert_eq!(s.start(id, "hi").0, 400, "{id}");
    }

    // No secret in cooee's config, files or log, nor in what it told shed.
    let mut written = fs::read_to_string(s.dir.join("cooee.err")).unwrap();
    assert!(
        written.contains("leak.toml"),
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

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

#[test]
fn a_plugins_profile_is_listed_while_it_is_installed_and_its_add_confirm_names_it() {
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
    let option = r#"href="/new?profile=plug/conductor""#;
    assert!(!s.picker().contains(option));

    let url = repo.to_string_lossy().into_owned();
    let out = s.cooee.cli(&["plugin", "add", &url, "--yes"]);
    assert!(
        out.contains("agent profiles: Conductor (fake-acp)"),
        "{out}"
    );
    assert!(s.picker().contains(option), "{}", s.picker());
    let (id, _) = s.thread("plug/conductor", "go");
    assert_eq!(
        s.shed.sent(&format!("cooee-{id}")),
        ["\x1e{\"prompt\":\"You are the conductor.\\n\\ngo\"}\n"]
    );
    assert!(
        s.cooee
            .get(&format!("/t/{id}"))
            .1
            .contains("Conductor · fake-acp · ")
    );

    s.cooee.cli(&["plugin", "remove", "plug"]);
    assert!(!s.picker().contains("Conductor"), "{}", s.picker());
    assert_eq!(s.start("plug/conductor", "go").0, 400);
    let pid = s.cooee.child.id().to_string();
    let _ = Command::new("kill").args(["-TERM", &pid]).status();
}
