use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::support::{Cooee, FakeShed, agent, scratch, version};

pub struct Setup {
    pub dir: PathBuf,
    pub shed: FakeShed,
    pub cooee: Cooee,
    pub token: String,
}

pub fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let token = format!("sk-{}", "p4".repeat(16));
    let data = dir.join("data");
    fs::create_dir_all(data.join("accounts/fake/work")).unwrap();
    fs::create_dir_all(data.join("secrets")).unwrap();
    fs::write(data.join("secrets/API_KEY"), format!("{token}\n")).unwrap();
    let settings = r#"{"hooks":{"PreToolUse":[]}}"#;
    let coder = agent(
        &dir,
        "coder",
        "name = \"Coder\"\nagent = \"fake-acp\"\naccount = \"work\"\nsettings = \"settings.json\"\n\
         policy = \"allow\"\nenv = { MODE = \"review\", API_KEY = \"secret:API_KEY\" }\n",
        "Follow the house rules.\n",
    );
    fs::write(coder.join("settings.json"), settings).unwrap();
    let guard = agent(
        &dir,
        "guard",
        "name = \"Guarded\"\nagent = \"other-acp\"\nsettings = \"settings.json\"\n",
        "",
    );
    fs::write(guard.join("settings.json"), settings).unwrap();
    let leak = format!("name = \"Leak\"\nagent = \"fake-acp\"\nenv = {{ T = \"{token}\" }}\n");
    agent(&dir, "leak", &leak, "");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = "[[providers]]\nname = \"fake\"\nbase = \"claude\"\nagent = \"fake-acp\"\nhome = \"FAKE_HOME\"\n\
                 settings = \"FAKE_SETTINGS\"\nlogin = [\"true\"]\nstatus = [\"true\"]\n\n\
                 [[mcp]]\nname = \"search\"\ncommand = \"true\"\nprovides = [\"web\"]\n";
    let cooee = Cooee::start_with(&dir, &shed.dir, extra);
    Setup {
        dir,
        shed,
        cooee,
        token,
    }
}

impl Setup {
    pub fn start(&self, profile: &str, text: &str) -> (u16, String) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new = json!({ "agent": "fake-acp", "profile": profile, "folder": self.dir.join("root"),
                          "text": text, "random_id": random });
        self.cooee.post("/threads", &new)
    }

    pub fn topic(&self, title: &str) -> String {
        let folder = fs::canonicalize(self.dir.join("root")).unwrap();
        format!("This topic: {title} (folder {}).\n\n", folder.display())
    }

    pub fn thread(&self, profile: &str, text: &str) -> (String, String) {
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

    pub fn row(&self, id: &str) -> Value {
        let list = self.cooee.json("/threads");
        let row = list.as_array().unwrap().iter().find(|t| t["id"] == id);
        row.cloned().unwrap_or_else(|| panic!("no {id} in {list}"))
    }

    pub fn backend_up(&self, name: &str) {
        for _ in 0..400 {
            if self.cooee.get(&format!("/p/{name}/api/tab.js")).0 == 200 {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("{name}'s backend never answered");
    }

    pub fn files(dir: &Path, out: &mut String) {
        for e in fs::read_dir(dir).into_iter().flatten().flatten() {
            if e.file_type().unwrap().is_dir() {
                Self::files(&e.path(), out);
            } else {
                out.push_str(&String::from_utf8_lossy(&fs::read(e.path()).unwrap()));
            }
        }
    }
}

pub fn git(dir: &Path, args: &[&str]) -> String {
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

pub fn serving(list: &Path, log: &Path) -> String {
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

pub fn options(shed: &FakeShed, line: &str) -> Option<Value> {
    let name = line.split(" --name ").nth(1)?.split(' ').next()?;
    Some(shed.meta(name)?["claudeCode"]["options"].clone())
}

pub fn switches(base: &str, meta: Option<&Value>, env: &str, sent: &str) {
    if base == "codex" {
        let config = env.lines().find_map(|l| l.strip_prefix("CODEX_CONFIG="));
        let config: Value = serde_json::from_str(config.expect(env)).unwrap();
        assert!(
            config["developer_instructions"]
                .as_str()
                .unwrap()
                .starts_with("Be brief.")
        );
        assert_eq!(
            config["features"],
            json!({ "multi_agent": false, "memories": false })
        );
        assert_eq!(
            config["tools"]["web_search"], true,
            "web stays until a connector"
        );
    } else {
        let options = &meta.expect("antigravity's switches are its _meta")["antigravity"];
        assert!(
            options["systemPrompt"]["append"]
                .as_str()
                .unwrap()
                .starts_with("Be brief.")
        );
        assert_eq!(
            options["disabledTools"],
            json!(["invoke_subagent", "run_workflow"]),
            "web stays until a connector"
        );
    }
    assert!(
        !sent.contains("Be brief."),
        "the brief is in the system channel: {sent}"
    );
}

pub fn claude_switches(meta: &Value, brief: &str) {
    assert_eq!(
        meta["systemPrompt"],
        json!({ "append": brief, "excludeDynamicSections": true }),
        "#503: the brief in the system channel, the vendor's prompt kept"
    );
    let options = &meta["claudeCode"]["options"];
    assert_eq!(
        (&options["settingSources"], &options["strictMcpConfig"]),
        (&json!(["project", "local"]), &json!(true)),
        "{meta}"
    );
    assert_eq!(options["settings"]["autoMemoryEnabled"], false, "{meta}");
    let permissions = &options["settings"]["permissions"];
    assert_eq!(permissions["defaultMode"], "default", "{meta}");
    assert_eq!(permissions["disableAutoMode"], "disable", "{meta}");
    let ask = permissions["ask"].as_array().unwrap();
    for t in ["Read", "Grep", "Bash", "Edit", "WebFetch"] {
        assert!(ask.contains(&json!(t)), "#503: {t} asks: {meta}");
    }
    let tools = options["tools"].as_array().unwrap();
    for off in ["Agent", "Skill", "WebSearch"] {
        assert_eq!(
            tools.contains(&json!(off)),
            off == "WebSearch",
            "{off}: {meta}"
        );
    }
}
