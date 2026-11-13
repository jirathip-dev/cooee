//! Agent profiles end to end with the fake shed (tests/support), whose CLI runs the adapter command a start gives
//! it: Plain starts the agent with nothing added; a file profile's account, settings, env (its secret through the
//! adapter's shell), policy and instructions are applied at spawn; a plugin's profile is listed while the plugin is
//! installed and gone once it is removed; a guard its agent can't run refuses the start; no secret reaches cooee's
//! config, files or log, or what it tells shed.

mod support;

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use cooee_core::personas::PLAIN;
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

    /// The agents once they have `what` (up to 20 s: the server runs a plugin's backend within half a second of its
    /// add, and asks it for its profiles once it answers).
    fn picker_with(&self, what: &str) -> String {
        for _ in 0..400 {
            let agents = self.picker();
            if agents.contains(what) {
                return agents;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("never {what}: {}", self.picker());
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
        chat.contains("<i class=\"dt\"></i><span id=\"net\"></span>Plain · fake · main login · "),
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
        [format!(
            "\x1e{}\n",
            json!({ "prompt": format!("Follow the house rules.\n\n{PLAIN}\n\nhi") })
        )]
    );
    let items = s.cooee.json(&format!("/threads/{id}/messages"));
    assert_eq!(items["items"][0]["text"], "hi", "{items}");
    let chat = s.cooee.get(&format!("/t/{id}")).1;
    assert!(
        chat.contains("<i class=\"dt\"></i><span id=\"net\"></span>Coder · "),
        "{chat}"
    );
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
    // Until its backend runs the manifest's profile is offered; once it does, an unreachable one offers none: wait for
    // it so the chat page below sees the backend's 404 (its manifest's), not a backend still starting.
    for _ in 0..400 {
        if s.cooee.get("/p/plug/api/whoami").0 == 200 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(s.picker_with(option).contains("Conductor"));
    let (id, _) = s.thread("plug/conductor", "go");
    assert_eq!(
        s.shed.sent(&format!("cooee-{id}")),
        ["\x1e{\"prompt\":\"You are the conductor.\\n\\ngo\"}\n"]
    );
    assert!(
        s.cooee
            .get(&format!("/t/{id}"))
            .1
            .contains("<i class=\"dt\"></i><span id=\"net\"></span>Conductor · ")
    );

    s.cooee.cli(&["plugin", "remove", "plug"]);
    assert!(!s.picker().contains("Conductor"), "{}", s.picker());
    assert_eq!(s.start("plug/conductor", "go").0, 400);
}

/// A plugin backend (docs/plugins/hello's checks) that serves the profiles in the JSON file `list` at
/// `GET /cooee/profiles` (read at each call) and appends each `POST /cooee/thread-started` body to `log`.
fn serving(list: &Path, log: &Path) -> String {
    format!(
        r#"import fs from "node:fs";
import http from "node:http";
const {{ PORT, TOKEN }} = process.env;
http.createServer(async (req, res) => {{
  if (req.headers.authorization !== `Bearer ${{TOKEN}}`) return res.writeHead(401).end();
  if (req.url === "/tab.js") return res.writeHead(200).end("document.body.append('plug');");
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
fn a_backend_serves_its_plugins_agents_a_hidden_one_is_left_out_and_thread_is_the_threads_id() {
    let s = setup("profiles-served");
    let repo = s.dir.join("plug.git");
    fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "--quiet", "-b", "main"]);
    let (list, log) = (s.dir.join("served.json"), s.dir.join("started.log"));
    let served = |threads: &[&str]| {
        let v = json!([
            { "name": "fleet-a", "label": "Fleet A", "agent": "fake-acp", "avatar": "/a.png",
              "env": { "HERDSMAN_THREAD": "{thread}", "FLEET": "a" }, "threads": threads },
            { "name": "fleet-b", "label": "Fleet B", "agent": "fake-acp", "hidden": true,
              "why": "its guard can't be proven" },
            { "name": "fleet-c", "label": "Fleet C", "agent": "fake-acp" },
        ]);
        fs::write(&list, v.to_string()).unwrap();
    };
    served(&[]);
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

    // The backend's list is the plugin's agents (once the server runs it), under its name after the owner's, with
    // their names and avatars (none: the initials); a hidden one, and the manifest's it doesn't list, aren't there.
    let agents = s.picker_with(r#"href="/new?profile=plug/fleet-a""#);
    let group = agents.find(r#"<p class="lb">plug</p>"#).expect(&agents);
    let a = agents.find(r#"href="/new?profile=plug/fleet-a""#).unwrap();
    assert!(
        agents.find("Coder").unwrap() < group && group < a,
        "{agents}"
    );
    let row = |at: usize| &agents[at..at + agents[at..].find("</a>").unwrap()];
    assert!(
        row(a).contains(r#"<img class="av" src="/p/plug/api/a.png" alt="">"#)
            && row(a).contains(r#"<span class="tt">Fleet A</span>"#),
        "{agents}"
    );
    let c = agents.find(r#"href="/new?profile=plug/fleet-c""#).unwrap();
    assert!(row(c).contains(">FC</i>"), "{agents}");
    assert!(
        !agents.contains("fleet-b") && !agents.contains("Fleet B") && !agents.contains("plug/old"),
        "{agents}"
    );
    let runs = s.shed.cli_log().matches(" run ").count();
    let (status, body) = s.start("plug/fleet-b", "go");
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("hidden: its guard can't be proven"), "{body}");
    assert_eq!(s.start("plug/old", "go").0, 400);
    assert_eq!(s.shed.cli_log().matches(" run ").count(), runs);
    assert!(!log.exists(), "nothing started");

    // `{thread}` is the new thread's id in the agent's env, and the backend is told the thread started.
    let (id, _) = s.thread("plug/fleet-a", "go");
    let env = s.shed.adapter_env();
    for var in [format!("HERDSMAN_THREAD={id}"), "FLEET=a".to_owned()] {
        assert!(env.lines().any(|l| l == var), "{var} not in {env}");
    }
    let told: Value = serde_json::from_str(fs::read_to_string(&log).unwrap().trim()).unwrap();
    assert_eq!(told, json!({ "thread": id, "profile": "fleet-a" }));

    // Threads the backend lists as the agent's are its side threads, the newest its ongoing chat.
    let (old, _) = s.thread("", "older");
    let (new, _) = s.thread("", "newer");
    served(&[&old, &new]);
    let agents = s.picker_with(&format!(r#"href="/t/{new}""#));
    let a = agents.find("Fleet A</span>").unwrap();
    let start = agents[..a].rfind("<a").unwrap();
    assert!(
        agents[start..a].contains(&format!(r#"href="/t/{new}""#)),
        "{agents}"
    );
    let chat = s.cooee.get(&format!("/t/{new}")).1;
    let at = chat.find(r#"<nav class="ags""#).unwrap();
    let side = &chat[at..at + chat[at..].find("</nav>").unwrap()];
    let fleet = side.find("Fleet A</span>").unwrap();
    let under = &side[fleet..side[fleet..].find("+ new chat").unwrap() + fleet];
    assert!(
        under.contains(&format!(r#"href="/t/{old}""#))
            && under.contains(&format!(r#"href="/t/{id}""#)),
        "{side}"
    );
    // They are no longer Plain's.
    assert!(
        side.contains(r#"<a class="agr" href="/new?agent=fake-acp">"#),
        "{side}"
    );
    // A phone's `‹` goes back to the agent that lists it, whose side threads they are.
    let back = r#"href="/?list&amp;profile=plug/fleet-a" aria-label="back to agents""#;
    assert!(chat.contains(back), "{chat}");

    s.cooee.cli(&["plugin", "remove", "plug"]);
}

#[test]
fn a_claude_profiles_settings_folder_is_its_config_folder_and_its_hook_refuses_a_command() {
    let dir = scratch("profiles-claude");
    let data = dir.join("data");
    fs::create_dir_all(data.join("accounts/claude/work")).unwrap();
    // The guard: a fake of herdsman's hook, refusing a push to main.
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
    // A fake claude-acp: runs the PreToolUse hook of `$CLAUDE_CONFIG_DIR/settings.json` on one Bash call, as
    // Claude Code does, noting its output and exit.
    let adapter = shed.dir.join("fake-acp");
    fs::write(
        &adapter,
        "#!/bin/sh\nout=\"$(dirname \"$0\")/hook.out\"\n\
         hook=$(sed -n 's/.*\"command\":\"\\([^\"]*\\)\".*/\\1/p' \"$CLAUDE_CONFIG_DIR/settings.json\")\n\
         echo '{\"tool_name\":\"Bash\",\"tool_input\":{\"command\":\"git push origin main\"}}' | sh \"$hook\" >\"$out\" 2>&1\n\
         echo \"exit $?\" >>\"$out\"\n",
    )
    .unwrap();
    // The built-in Claude Code provider, on the fake shed's agent.
    let mut provider = cooee_core::provider::defaults().remove(0);
    provider.agent = "fake-acp".to_owned();
    (provider.login, provider.status) = (vec!["true".to_owned()], vec!["true".to_owned()]);
    // Its fields are strings and lists of them: as JSON, they are TOML too.
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
