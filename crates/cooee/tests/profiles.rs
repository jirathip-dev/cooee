//! Agent profiles end to end with the fake shed (tests/support), whose CLI runs the adapter command a start gives
//! it: Plain starts the agent with nothing added; a file profile's account, settings, env (its secret through the
//! adapter's shell), policy, its provider's "no personal setup" options and instructions are applied at spawn; a
//! plugin's profile starts while the plugin is installed and is refused once it is removed; a guard its agent can't
//! run refuses the start; no secret reaches cooee's config, files or log, or what it tells shed.

mod support;

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use cooee_core::personas::PLAIN;
use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

struct Setup {
    dir: PathBuf,
    shed: FakeShed,
    cooee: Cooee,
    token: String,
}

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
    let extra = "[[providers]]\nname = \"fake\"\nbase = \"claude\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\n\
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
    fn start(&self, profile: &str, text: &str) -> (u16, String) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new = json!({ "agent": "fake-acp", "profile": profile, "folder": self.dir.join("root"),
                          "text": text, "random_id": random });
        self.cooee.post("/threads", &new)
    }

    fn thread(&self, profile: &str, text: &str) -> (String, String) {
        let until = Instant::now() + Duration::from_secs(20);
        let (mut status, mut body) = self.start(profile, text);
        while status == 400 && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(50));
            (status, body) = self.start(profile, text);
        }
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

    fn row(&self, id: &str) -> Value {
        let list = self.cooee.json("/threads");
        let row = list.as_array().unwrap().iter().find(|t| t["id"] == id);
        row.cloned().unwrap_or_else(|| panic!("no {id} in {list}"))
    }

    fn backend_up(&self, name: &str) {
        for _ in 0..400 {
            if self.cooee.get(&format!("/p/{name}/api/tab.js")).0 == 200 {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("{name}'s backend never answered");
    }

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

    let (id, line) = s.thread("coder", "hi");
    let data = s.dir.join("data");
    let work = data.join("accounts/fake/work");
    let settings = s.dir.join("profiles/settings.json");
    assert!(line.contains(" --policy allow "), "{line}");
    let meta = line
        .split(" --meta ")
        .nth(1)
        .and_then(|m| m.split(' ').next());
    let meta: Value = serde_json::from_str(meta.expect("#320: no personal setup")).unwrap();
    let options = json!({ "settingSources": ["project", "local"], "strictMcpConfig": true,
        "settings": { "claudeMdExcludes": ["{/Users/*,/home/*,/root}/.claude/{CLAUDE.md,rules/**}"] } });
    assert_eq!(
        meta,
        json!({ "claudeCode": { "options": options } }),
        "{line}"
    );
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
        [format!(
            "\x1e{}\n",
            json!({ "prompt": format!("Follow the house rules.\n\n{PLAIN}\n\nhi") })
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
    assert_eq!(
        s.shed.sent(&format!("cooee-{id}")),
        ["\x1e{\"prompt\":\"You are the conductor.\\n\\ngo\"}\n"]
    );
    assert_eq!(s.row(&id)["profile"], "plug/conductor");

    fs::create_dir_all(s.dir.join("personas")).unwrap();
    fs::write(
        s.dir.join("personas/lead.toml"),
        "name = \"Lead\"\nagent = \"fake-acp\"\nprofile = \"plug/conductor\"\ncharacter = \"Calm.\"\n",
    )
    .unwrap();
    let (id, _) = s.thread("lead", "go");
    assert_eq!(
        s.shed.sent(&format!("cooee-{id}")),
        ["\x1e{\"prompt\":\"Calm.\\n\\nYou are the conductor.\\n\\ngo\"}\n"]
    );

    s.cooee.cli(&["plugin", "remove", "plug"]);
    assert_eq!(s.start("plug/conductor", "go").0, 400);
}

fn serving(list: &Path, log: &Path) -> String {
    format!(
        r#"import fs from "node:fs";
import http from "node:http";
const {{ PORT, TOKEN }} = process.env;
http.createServer(async (req, res) => {{
  if (req.headers.authorization !== `Bearer ${{TOKEN}}`) return res.writeHead(401).end();
  if (req.url === "/tab.js") return res.writeHead(200).end("document.body.append('plug');");
  if (req.url === "/cooee/profiles") fs.appendFileSync({list:?} + ".asked", "\n");
  if (req.url === "/cooee/profiles" && fs.existsSync({list:?} + ".hang")) return;
  if (req.url === "/cooee/profiles") return res.writeHead(200).end(`{{"profiles":${{fs.readFileSync({list:?})}}}}`);
  let b = ""; for await (const c of req) b += c;
  if (req.url === "/cooee/thread-started") {{ fs.appendFileSync({log:?}, b + "\n"); return res.writeHead(200).end("{{}}"); }}
  res.writeHead(404).end();
}}).listen(Number(PORT), "127.0.0.1");
"#,
        list = list.to_string_lossy(),
        log = log.to_string_lossy()
    )
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

#[test]
fn a_claude_profiles_settings_folder_is_its_config_folder_and_its_hook_refuses_a_command() {
    let dir = scratch("profiles-claude");
    let data = dir.join("data");
    fs::create_dir_all(data.join("accounts/claude/work")).unwrap();
    let claude = dir.join("profiles/claude");
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
        dir.join("profiles/guarded.toml"),
        "name = \"guarded\"\nlabel = \"Guarded\"\nagent = \"fake-acp\"\naccount = \"work\"\nsettings = \"claude\"\n",
    )
    .unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let adapter = shed.dir.join("fake-acp");
    fs::write(
        &adapter,
        "#!/bin/sh\nout=\"$(dirname \"$0\")/hook.out\"\n\
         hook=$(sed -n 's/.*\"command\":\"\\([^\"]*\\)\".*/\\1/p' \"$CLAUDE_CONFIG_DIR/settings.json\")\n\
         echo '{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"git push origin main\"}}' | sh \"$hook\" >\"$out\" 2>&1\n\
         echo \"exit $?\" >>\"$out\"\n",
    )
    .unwrap();
    let mut provider = cooee_core::provider::defaults().remove(0);
    provider.agent = "fake-acp".to_owned();
    (provider.login, provider.status) = (vec!["true".to_owned()], vec!["true".to_owned()]);
    let mut extra = "[[providers]]\n".to_owned();
    for (k, v) in serde_json::to_value(&provider)
        .unwrap()
        .as_object()
        .unwrap()
    {
        writeln!(extra, "{k} = {v}").unwrap();
    }
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);

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

fn options(line: &str) -> Option<Value> {
    let meta = line.split(" --meta ").nth(1)?.split(' ').next()?;
    let meta: Value = serde_json::from_str(meta).unwrap();
    Some(meta["claudeCode"]["options"].clone())
}

#[test]
fn a_persona_with_no_profile_and_a_room_member_start_without_personal_setup_plain_keeps_it() {
    let s = setup("persona-no-personal");
    fs::create_dir_all(s.dir.join("personas")).unwrap();
    fs::write(
        s.dir.join("personas/ada.toml"),
        "name = \"Ada\"\nagent = \"fake-acp\"\n",
    )
    .unwrap();
    let personal = |line: &str| {
        let o = options(line).unwrap_or_else(|| panic!("#416: no options in {line}"));
        assert_eq!(o["strictMcpConfig"], true, "{line}");
        assert_eq!(o["settingSources"], json!(["project", "local"]), "{line}");
    };

    let (_, line) = s.thread("ada", "hi");
    personal(&line);
    let (_, line) = s.thread("", "hi");
    assert_eq!(options(&line), None, "{line}");

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
            let line = log.lines().find(|l| l.contains(&label)).map(str::to_owned);
            line.or_else(|| {
                std::thread::sleep(std::time::Duration::from_millis(50));
                None
            })
        });
        personal(&line.unwrap_or_else(|| panic!("{member}'s room session never started")));
    }
}
