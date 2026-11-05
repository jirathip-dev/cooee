//! The binary end to end: `--version`, and `cooee serve` on the configured port: an unpaired browser gets the
//! pairing page and the PWA's files.

use std::{
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Command, Stdio},
};

const BIN: &str = env!("CARGO_BIN_EXE_cooee");

#[test]
fn version() {
    let out = Command::new(BIN).arg("--version").output().unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("cooee {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn unknown_args_and_config_keys_fail() {
    assert_eq!(
        Command::new(BIN)
            .arg("serv")
            .output()
            .unwrap()
            .status
            .code(),
        Some(2)
    );
    let config = env::temp_dir().join(format!("cooee-bad-config-{}.toml", std::process::id()));
    fs::write(&config, "prot = 1\n").unwrap();
    let out = Command::new(BIN)
        .arg("serve")
        .arg("--config")
        .arg(&config)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown field `prot`"));
}

fn get(addr: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(addr).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    String::from_utf8_lossy(&response).into_owned()
}

#[test]
fn serve_listens_on_localhost_at_the_configured_port() {
    // Port 0 in the config: the server binds a free port on 127.0.0.1 and prints it.
    let config = env::temp_dir().join(format!("cooee-config-{}.toml", std::process::id()));
    fs::write(
        &config,
        format!(
            "port = 0\ndata = {:?}\n",
            env::temp_dir().join("cooee-cli-data")
        ),
    )
    .unwrap();
    let mut child = Command::new(BIN)
        .arg("serve")
        .arg("--config")
        .arg(&config)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let addr = line
        .trim()
        .strip_prefix("cooee: listening on http://")
        .unwrap()
        .to_owned();

    let page = get(&addr, "/");
    let pair = get(&addr, "/pair");
    let manifest = get(&addr, "/manifest.webmanifest");
    let worker = get(&addr, "/sw.js");
    let icon = get(&addr, "/icon-512.png");
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(
        addr.starts_with("127.0.0.1:") && !addr.ends_with(":0"),
        "{addr}"
    );
    assert!(
        page.starts_with("HTTP/1.1 303") && page.to_ascii_lowercase().contains("location: /pair"),
        "{page}"
    );
    assert!(pair.starts_with("HTTP/1.1 200"), "{pair}");
    assert!(
        pair.contains("pair this device") && pair.contains(r#"rel="manifest""#),
        "{pair}"
    );
    assert!(worker.contains(r#"addEventListener("push""#), "{worker}");
    assert!(icon.contains("image/png"), "{icon}");
    assert!(
        manifest
            .to_ascii_lowercase()
            .contains("content-type: application/manifest+json"),
        "{manifest}"
    );
    assert!(
        manifest.contains(r#""display": "standalone""#)
            && manifest.contains(r#""sizes": "512x512""#),
        "{manifest}"
    );
}

#[test]
fn listening_beyond_localhost_needs_allowed_hosts() {
    let config = env::temp_dir().join(format!("cooee-bind-{}.toml", std::process::id()));
    fs::write(&config, "port = 0\nbind = \"0.0.0.0\"\n").unwrap();
    let out = Command::new(BIN)
        .args(["serve", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("needs `allowed_hosts`"),
        "{out:?}"
    );
}
