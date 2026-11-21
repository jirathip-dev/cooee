use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::support::{Cooee, FakeShed, scratch, version};

pub type Case<'a> = (&'a str, Vec<(&'a str, &'a str)>, &'a str);

pub struct Setup {
    pub dir: PathBuf,
    pub shed: FakeShed,
    pub cooee: Cooee,
}

pub fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    Setup { dir, shed, cooee }
}

pub fn until(what: &str, f: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !f() {
        assert!(Instant::now() < deadline, "never: {what}");
        thread::sleep(Duration::from_millis(20));
    }
}

pub fn start(s: &Setup, text: &str, random_id: &str) -> Value {
    json!({ "agent": "fake-acp", "folder": s.dir.join("root/project"), "text": text, "random_id": random_id })
}

pub fn id_of(body: &str) -> String {
    serde_json::from_str::<Value>(body).unwrap()["id"]
        .as_str()
        .unwrap_or_else(|| panic!("{body}"))
        .to_owned()
}

pub fn runs(shed: &FakeShed) -> usize {
    shed.cli_log()
        .lines()
        .filter(|l| l.contains(" run "))
        .count()
}

pub fn prompts(shed: &FakeShed, id: &str) -> Vec<String> {
    let sent = shed.sent(&format!("cooee-{id}"));
    sent.into_iter()
        .filter(|t| t.contains("\"prompt\""))
        .collect()
}

pub fn listed(cooee: &Cooee) -> Vec<String> {
    let rows = cooee.json("/threads");
    let ids = rows.as_array().unwrap().iter();
    ids.map(|r| r["id"].as_str().unwrap().to_owned()).collect()
}

pub fn post_raw(addr: &str, cookie: &str, path: &str, body: &Value) -> Option<String> {
    let mut stream = TcpStream::connect(addr).ok()?;
    let body = body.to_string();
    let request = format!(
        "POST {path} HTTP/1.0\r\nHost: {addr}\r\nCookie: {cookie}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).ok()?;
    let mut answer = String::new();
    stream.read_to_string(&mut answer).ok()?;
    Some(answer)
}

#[cfg(target_os = "macos")]
pub fn fake_security(dir: &std::path::Path, script: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("security"), script).unwrap();
    fs::set_permissions(bin.join("security"), fs::Permissions::from_mode(0o755)).unwrap();
    format!("{}:{}", bin.display(), std::env::var("PATH").unwrap())
}

pub fn approval(cooee: &Cooee, id: &str) -> Option<Value> {
    let items = cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let items = items.as_array().unwrap().clone();
    items.into_iter().find(|i| i["kind"] == "approval")
}
