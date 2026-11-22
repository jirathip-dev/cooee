use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    os::unix::fs::PermissionsExt,
    thread,
};

use serde_json::json;

use crate::setup::{Agent, Mcp, Setup, fake_web, files, named};
use crate::support::{Cooee, FakeShed, agent, is_item, scratch, version};

#[test]
fn a_streamable_http_and_an_sse_server_answer_through_the_pipe_with_headers_on_curls_stdin_and_no_redirect()
 {
    let dir = scratch("mcp-pipe");
    let token = format!("tk-{}", "w3".repeat(16));
    fs::create_dir_all(dir.join("data/secrets")).unwrap();
    fs::write(dir.join("data/secrets/WEB"), format!("Bearer {token}\n")).unwrap();
    let real = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|p| p.join("curl"))
        .find(|p| p.is_file())
        .unwrap();
    let wrap = dir.join("wrap/curl");
    fs::create_dir_all(wrap.parent().unwrap()).unwrap();
    let log = dir.join("curl-argv");
    fs::write(
        &wrap,
        format!("#!/bin/sh\nprintf '%s\\n' \"$*\" >>{log:?}\nexec {real:?} \"$@\"\n"),
    )
    .unwrap();
    fs::set_permissions(&wrap, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        wrap.parent().unwrap().display(),
        std::env::var("PATH").unwrap()
    );
    let picks = "name = \"Web\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"web\"\ntools = [\"a\"]\n\n[[mcp]]\nserver = \"old\"\n\n[[mcp]]\nserver = \"moved\"\n";
    agent(&dir, "web", picks, "");
    let (port, seen) = fake_web(token.clone());
    let server = |name: &str, kind: &str, at: &str| {
        format!(
            "[[mcp]]\nname = \"{name}\"\ntype = \"{kind}\"\nurl = \"http://127.0.0.1:{port}/{at}\"\nheaders = {{ Authorization = \"secret:WEB\" }}\nread = [\"a\", \"c\"]\n\n"
        )
    };
    let extra = server("web", "http", "mcp")
        + &server("old", "sse", "sse")
        + &server("moved", "http", "moved");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[("PATH", &path)]);
    let s = Setup {
        dir: dir.clone(),
        shed,
        cooee,
        token: token.clone(),
    };
    let (id, servers) = s.thread("web");

    let mut web = Mcp::start(named(&servers, "web"), Agent::Claude);
    assert_eq!(web.tools(), ["a"]);
    assert_eq!(
        web.run("a", &json!({})),
        Ok("a ran; the token matches".to_owned())
    );
    let refused = web.run("b", &json!({})).unwrap_err();
    assert!(refused.contains("tool b isn't allowed"), "{refused}");
    let last = seen.lock().unwrap().last().unwrap().to_ascii_lowercase();
    assert!(
        last.contains("mcp-session-id: s-1\r\n")
            && last.contains("mcp-protocol-version: 2025-06-18\r\n"),
        "{last}"
    );

    let mut old = Mcp::start(named(&servers, "old"), Agent::Codex);
    assert_eq!(old.tools(), ["a", "b", "c"]);
    assert_eq!(
        old.run("c", &json!({})),
        Ok("c ran; the token matches".to_owned())
    );

    let mut moved = Mcp::spawn(named(&servers, "moved"), Agent::Claude);
    let e = moved.call("initialize", &json!({})).unwrap_err();
    assert!(e.contains("HTTP 307"), "{e}");
    assert!(
        !seen
            .lock()
            .unwrap()
            .iter()
            .any(|h| h.contains("redirected"))
    );

    let var = named(&servers, "web")["env"][0]["value"]
        .as_str()
        .unwrap()
        .to_owned();
    let (url, bearer) = var.split_once(' ').unwrap();
    let mut c = TcpStream::connect(url.strip_prefix("http://").unwrap()).unwrap();
    let body = json!({ "thread": id, "server": "web" }).to_string();
    write!(c, "POST /mcp/agent/grant HTTP/1.0\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {bearer}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut grant = String::new();
    c.read_to_string(&mut grant).unwrap();
    assert!(grant.ends_with("\"pipe\""), "{grant}");
    assert!(
        !grant.contains(&s.token) && !grant.contains(&port.to_string()),
        "{grant}"
    );

    let argv = fs::read_to_string(&log).unwrap();
    assert!(argv.lines().count() >= 8, "{argv}");
    assert!(argv.lines().all(|l| l == "-K -"), "{argv}");
    let mut all = fs::read_to_string(&s.cooee.config).unwrap();
    files(&dir.join("data"), &mut all);
    all.push_str(&fs::read_to_string(dir.join("cooee.err")).unwrap());
    assert!(!all.contains(&s.token), "a value was written");
}

#[test]
fn a_read_tool_runs_with_no_card_and_a_write_tool_runs_only_once_its_card_is_allowed() {
    let dir = scratch("mcp-gate");
    let token = format!("tk-{}", "g5".repeat(16));
    fs::create_dir_all(dir.join("data/secrets")).unwrap();
    fs::write(dir.join("data/secrets/WEB"), format!("Bearer {token}\n")).unwrap();
    let picks = "name = \"Web\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"web\"\n";
    agent(&dir, "web", picks, "");
    let (port, seen) = fake_web(token.clone());
    let extra = format!(
        "[[mcp]]\nname = \"web\"\ntype = \"http\"\nurl = \"http://127.0.0.1:{port}/mcp\"\nheaders = {{ Authorization = \"secret:WEB\" }}\nread = [\"a\"]\n"
    );
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    let s = Setup {
        dir,
        shed,
        cooee,
        token,
    };
    let (id, servers) = s.thread("web");
    let messages = || s.cooee.json(&format!("/threads/{id}/messages"));
    let sent = |tool: &str| {
        let name = format!("\"name\":\"{tool}\"");
        seen.lock()
            .unwrap()
            .iter()
            .filter(|h| h.contains(&name))
            .count()
    };
    let mut web = Mcp::start(named(&servers, "web"), Agent::Claude);

    assert_eq!(
        web.run("a", &json!({})),
        Ok("a ran; the token matches".to_owned())
    );
    let items = messages()["items"].clone();
    assert!(
        !items
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "approval"),
        "{items}"
    );

    for (tool, option) in [("b", "approve"), ("c", "decline")] {
        let cursor = messages()["cursor"].as_str().unwrap().to_owned();
        let mut events = s.cooee.events(&id, &cursor, false);
        let asked = thread::spawn(move || {
            let r = web.run(tool, &json!({ "title": "Fix it" }));
            (web, r)
        });
        let card = events
            .until(|e| is_item(e, "approval") && e.data["form"] == "tool")
            .pop()
            .unwrap()
            .data;
        assert_eq!(
            card["tool"],
            format!("Web · web: {tool} {{\"title\":\"Fix it\"}}"),
            "{card}"
        );
        assert_eq!(sent(tool), 0, "nothing is sent before the owner answers");
        let tap = json!({ "item": card["id"], "option": option });
        let (status, body) = s.cooee.post(&format!("/threads/{id}/approve"), &tap);
        assert_eq!(status, 200, "{body}");
        let (back, r) = asked.join().unwrap();
        web = back;
        if option == "approve" {
            assert_eq!(r, Ok("b ran; the token matches".to_owned()));
            assert_eq!(sent(tool), 1);
        } else {
            assert_eq!(
                r,
                Err("the owner didn't allow c: its card was declined or expired".to_owned())
            );
            assert_eq!(sent(tool), 0);
        }
    }
}
