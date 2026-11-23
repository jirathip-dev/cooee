use std::{
    fs,
    process::{Command, Stdio},
};

use serde_json::{Value, json};

use crate::setup::{Agent, Mcp, files, named, setup};

#[test]
fn the_agent_sees_and_calls_only_the_allowed_tools_under_claude_and_codex_and_no_value_is_written()
{
    let s = setup("mcp-proxy");
    let (id, servers) = s.thread("coder");
    let names: Vec<&str> = servers
        .iter()
        .map(|s| s["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["cooee", "fs"]);
    let fs_entry = named(&servers, "fs");
    assert_eq!(fs_entry["command"], env!("CARGO_BIN_EXE_cooee"));
    assert_eq!(
        fs_entry["args"],
        json!(["mcp", "proxy", "fs", "--thread", id])
    );
    for agent in [Agent::Claude, Agent::Codex] {
        let mut m = Mcp::start(fs_entry, agent);
        assert_eq!(m.tools(), ["a"], "{agent:?}");
        assert_eq!(
            m.run("a", &json!({})),
            Ok("a ran; the token matches".to_owned()),
            "{agent:?}"
        );
        let refused = m.run("b", &json!({})).unwrap_err();
        assert!(
            refused.contains("tool b isn't allowed"),
            "{agent:?}: {refused}"
        );
        assert!(m.run("c", &json!({})).is_err(), "{agent:?}");
    }
    let deny = json!({ "effect": "deny", "kind": "tool", "pattern": "fs/*" });
    let (status, rule) = s.cooee.post("/personas/coder/rules", &deny);
    assert_eq!(status, 200, "{rule}");
    let mut m = Mcp::start(fs_entry, Agent::Claude);
    let e = m.run("a", &json!({})).unwrap_err();
    assert!(e.contains("fs/a: Deny the tool fs/*"), "#503: {e}");
    let rule: Value = serde_json::from_str(&rule).unwrap();
    let gone = format!("/personas/coder/rules/{}", rule["id"].as_str().unwrap());
    assert_eq!(s.cooee.request("DELETE", &gone, None).0, 200);
    assert!(m.run("a", &json!({})).is_ok());
    drop(m);

    let (plain, servers) = s.thread("");
    assert_eq!(servers.len(), 1, "{servers:?}");
    let proxy = |var: &Value| {
        let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
            .args(["mcp", "proxy", "fs", "--thread", &plain])
            .env("COOEE_MCP", var["env"][0]["value"].as_str().unwrap())
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(!out.status.success());
        String::from_utf8(out.stderr).unwrap()
    };
    let e = proxy(&servers[0]);
    assert!(
        e.contains(&format!(
            "thread {plain}'s profile doesn't pick MCP server fs"
        )),
        "{e}"
    );
    let e = proxy(fs_entry);
    assert!(
        e.contains(&format!("this MCP token isn't thread {plain}'s")),
        "{e}"
    );

    let mut m = Mcp::start(fs_entry, Agent::Claude);
    assert!(m.run("a", &json!({})).is_ok());
    assert_eq!(
        s.cooee.post(&format!("/threads/{id}/close"), &json!({})).0,
        200
    );
    let e = m.run("a", &json!({})).unwrap_err();
    assert!(e.contains("closed"), "{e}");
    drop(m);

    let mut all = fs::read_to_string(&s.cooee.config).unwrap();
    files(&s.dir.join("data"), &mut all);
    files(&s.shed.dir, &mut all);
    all.push_str(&fs::read_to_string(s.dir.join("cooee.err")).unwrap());
    all.push_str(&serde_json::to_string(&*s.shed.requests.lock().unwrap()).unwrap());
    all.push_str(&s.cooee.get(&format!("/threads/{id}/messages")).1);
    assert!(all.contains("secret:TOKEN"), "the scan reads the config");
    assert!(!all.contains(&s.token), "a value was written");
    let left: Vec<_> = fs::read_dir(s.dir.join("data/secrets"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert_eq!(left, ["TOKEN"], "the --mcp file is gone");
}
