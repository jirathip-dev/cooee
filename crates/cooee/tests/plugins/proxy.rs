use std::{
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::TcpStream,
    process::Command,
    sync::{atomic::Ordering, mpsc},
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::setup::{TOKEN_A, TOKEN_B, new_thread, setup, status};
use crate::support::{HANG, others_go_on, scratch};

#[test]
fn the_proxy_sends_the_plugins_own_token_and_never_the_cookie() {
    let s = setup("plugin-proxy");
    let (code, body) = s.cooee.post("/p/a/api/echo?x=1", &json!({ "n": 1 }));
    assert_eq!((code, body.as_str()), (200, r#"{"path":"/echo?x=1"}"#));
    let (head, sent) = s.a.seen().pop().unwrap();
    assert!(head.starts_with("post /echo?x=1 http/1.0\r\n"), "{head}");
    assert!(
        head.contains(&format!("authorization: bearer {TOKEN_A}\r\n")),
        "{head}"
    );
    assert!(head.contains("content-type: application/json"), "{head}");
    assert!(
        !head.contains("cookie") && !head.contains(&s.cooee.cookie),
        "{head}"
    );
    assert_eq!(sent, r#"{"n":1}"#);

    assert_eq!(s.cooee.get("/p/b/api/echo").0, 200);
    let (head, _) = s.b.seen().pop().unwrap();
    assert!(
        head.contains(&format!("authorization: bearer {TOKEN_B}\r\n")),
        "{head}"
    );
    assert_eq!(s.a.seen().len(), 1);
    assert!(s.b.seen().iter().all(|(h, _)| !h.contains(TOKEN_A)));

    let (head, _) = s.cooee.raw("GET", "/p/a/api/echo", &[], "");
    assert_eq!(status(&head), 401);
    assert_eq!(s.a.seen().len(), 1);
    assert_eq!(s.cooee.get("/p/nope/api/echo").0, 404);
}

#[test]
fn a_backends_event_stream_comes_through_the_proxy() {
    let s = setup("plugin-sse");
    let mut stream = TcpStream::connect(&s.cooee.addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "GET /p/a/api/events HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\nLast-Event-ID: 6\r\n\r\n",
        s.cooee.addr, s.cooee.cookie
    )
    .unwrap();
    let mut lines = BufReader::new(stream).lines().map(Result::unwrap);
    let head: Vec<String> = lines.by_ref().take_while(|l| !l.is_empty()).collect();
    assert!(head[0].contains(" 200 "), "{head:?}");
    let event: Vec<String> = lines
        .skip_while(|l| l.is_empty() || l.starts_with(':'))
        .take_while(|l| !l.is_empty())
        .collect();
    assert!(event.contains(&"id: 7".to_owned()), "{event:?}");
    let data = event.iter().find_map(|l| l.strip_prefix("data: ")).unwrap();
    let data: Value = serde_json::from_str(data).unwrap();
    assert_eq!(data, json!({ "event": "tick", "data": "one\ntwo" }));
    let (head, _) = s.a.seen().pop().unwrap();
    assert!(head.contains("last-event-id: 6\r\n"), "{head}");
}

#[test]
fn a_closed_page_closes_its_backend_stream() {
    let s = setup("plugin-sse-closed");
    let mut stream = TcpStream::connect(&s.cooee.addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(20)))
        .unwrap();
    write!(
        stream,
        "GET /p/a/api/beats HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n\r\n",
        s.cooee.addr, s.cooee.cookie
    )
    .unwrap();
    let mut lines = BufReader::new(stream).lines().map(Result::unwrap);
    let head = lines.next().unwrap();
    assert!(head.contains(" 200 "), "{head}");
    drop(lines);
    let closed = || s.a.seen().iter().any(|(h, _)| h == "closed /beats");
    for _ in 0..100 {
        if closed() {
            return;
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("the backend's stream is still open 5 s after its page closed");
}

#[test]
fn a_plugin_without_its_token_is_refused() {
    let s = setup("plugin-token");
    let push = json!({ "text": "build green", "path": "lane/3" });
    let card = json!({ "thread": "t1", "text": "Ship it?", "options": ["yes"] });
    for auth in ["", "Bearer", "Bearer wrong-token-0123456789", TOKEN_A] {
        assert_eq!(s.as_backend("/plugin/push", auth, &push).0, 401, "{auth}");
        assert_eq!(s.as_backend("/plugin/card", auth, &card).0, 401, "{auth}");
    }
    assert_eq!(s.cooee.post("/plugin/push", &push).0, 401);

    let (code, note) = s.as_backend("/plugin/push", &format!("Bearer {TOKEN_A}"), &push);
    assert_eq!(code, 200, "{note}");
    assert_eq!(
        serde_json::from_str::<Value>(&note).unwrap(),
        json!({ "title": "Alpha", "body": "build green", "tag": "plugin-a", "url": "/p/a#lane/3", "item": 0 })
    );
}

#[test]
fn a_plugin_with_no_token_configured_does_not_start() {
    let dir = scratch("plugin-config");
    let config = dir.join("config.toml");
    std::fs::write(
        &config,
        "port = 0\n[[plugins]]\nname = \"a\"\nlabel = \"A\"\nurl = \"http://127.0.0.1:1\"\nmodule = \"/tab.js\"\ntoken = \"\"\n",
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["serve", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success() && err.contains("`token`"), "{err}");
}

#[test]
fn a_hanging_plugin_answer_never_holds_another_thread() {
    let s = setup("plugin-hang");
    let id = new_thread(&s, "Say hello");
    let other = new_thread(&s, "Say hi");
    let (code, card) =
        s.card(&json!({ "thread": id, "text": "Merge it?", "options": ["yes", "no"] }));
    assert_eq!(code, 200, "{card}");
    s.a.hang.store(true, Ordering::SeqCst);
    let tap = json!({ "item": card["id"], "option": "yes" });
    let path = format!("/threads/{id}/approve");
    thread::scope(|scope| {
        let tapped = scope.spawn(|| s.cooee.post(&path, &tap));
        while s.a.choices().is_empty() {
            thread::sleep(Duration::from_millis(20));
        }
        others_go_on(&s.cooee, &other);
        assert!(!tapped.is_finished(), "the plugin's answer still hangs");
        s.a.hang.store(false, Ordering::SeqCst);
        let (code, body) = tapped.join().unwrap();
        assert_eq!(code, 200, "{body}");
    });
}

#[test]
fn a_hung_plugin_holds_only_its_own_calls() {
    let s = setup("plugin-busy");
    let other = new_thread(&s, "Say hi");
    s.a.hang.store(true, Ordering::SeqCst);
    let (addr, cookie) = (&s.cooee.addr, &s.cooee.cookie);
    let get = format!("GET /p/a/api/hang HTTP/1.0\r\nHost: {addr}\r\nCookie: {cookie}\r\n\r\n");
    let call = || {
        let mut last = String::new();
        for _ in 0..500 {
            let mut answer = String::new();
            let got = TcpStream::connect(addr).and_then(|mut c| {
                c.set_read_timeout(Some(Duration::from_secs(20)))?;
                c.write_all(get.as_bytes())?;
                BufReader::new(c).read_to_string(&mut answer)
            });
            match got {
                Ok(_) => return answer,
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return e.to_string();
                }
                Err(e) => {
                    last = e.to_string();
                    thread::sleep(Duration::from_millis(20));
                }
            }
        }
        last
    };
    thread::scope(|scope| {
        let (tx, answers) = mpsc::channel();
        for _ in 0..600 {
            let tx = tx.clone();
            scope.spawn(move || {
                tx.send(call()).ok();
            });
        }
        let busy: Vec<String> = answers.iter().take(592).collect();
        let not_busy = busy
            .iter()
            .find(|a| !a.contains("busy: 8 calls already in flight"));
        assert!(not_busy.is_none(), "{not_busy:?}");
        assert_eq!(s.a.seen().len(), 8);
        others_go_on(&s.cooee, &other);
        let at = Instant::now();
        assert_eq!(s.cooee.get("/p/b/api/echo").0, 200);
        let took = at.elapsed();
        assert!(took < HANG / 2, "plugin b took {took:?}");
        assert!(answers.try_recv().is_err(), "the 8 calls to a still hang");
        s.a.hang.store(false, Ordering::SeqCst);
        for answer in answers.iter().take(8) {
            assert!(answer.contains(r#"{"path":"/hang"}"#), "{answer}");
        }
    });
}
