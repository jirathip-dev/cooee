//! The desktop app (#715) against a real server: unpaired it lands on the pair screen, a typed code pairs it, the
//! sessions list follows the device stream, a chat opens and a reply streams in, each drawn by blocks-desktop.

mod support;

use std::{fs, time::Duration};

use blocks::Action;
use blocks_desktop::{Event, Kit};
use cooee_desktop::Shell;
use serde_json::{Value, json};
use support::{Cooee, FakeShed, scratch, version};

fn until(shell: &mut Shell, what: &str, done: impl Fn(&Shell, &Value) -> bool) {
    for _ in 0..300 {
        let screen = serde_json::to_value(shell.screen()).unwrap();
        if done(shell, &screen) {
            return;
        }
        shell.take(Duration::from_millis(100));
    }
    panic!("never: {what}; at {} ({})", shell.key(), shell.toast());
}

fn draw(shell: &mut Shell) -> Vec<String> {
    let ctx = egui::Context::default();
    for _ in 0..2 {
        ctx.run_ui(egui::RawInput::default(), |ui| {
            shell.show(ui, &Kit::default(), 0.0);
        })
        .textures_delta
        .clear();
    }
    shell.drawn().unwrap().blocks.clone()
}

fn act(shell: &mut Shell, screen: &str, block: &str, action: &str, values: Value) {
    shell.event(Event::Act(Action {
        screen: screen.to_owned(),
        block: Some(block.to_owned()),
        action: action.to_owned(),
        values: serde_json::from_value(values).unwrap(),
    }));
}

fn code(cooee: &Cooee) -> String {
    let out = cooee.cli(&["pair"]);
    let line = out.lines().find_map(|l| l.strip_prefix("code  ")).unwrap();
    line.chars().take(7).filter(char::is_ascii_digit).collect()
}

fn pair(cooee: &Cooee, dir: &std::path::Path) -> Shell {
    let mut shell = Shell::new(&format!("http://{}/", cooee.addr), dir.join("jar.txt"));
    until(&mut shell, "the pair screen", |s, v| {
        s.key() == "/screens/pair" && v["id"] == "pair"
    });
    assert!(draw(&mut shell).contains(&"code".to_owned()));
    let right = code(cooee);
    act(
        &mut shell,
        "pair",
        "code",
        "code",
        json!({ "code": "000000" }),
    );
    until(&mut shell, "a wrong code's toast", |s, _| {
        s.toast().contains("wrong")
    });
    act(&mut shell, "pair", "code", "code", json!({ "code": right }));
    shell
}

#[test]
fn it_pairs_by_code_follows_the_sessions_and_streams_a_reply_into_the_chat() {
    let dir = scratch("desktop");
    fs::create_dir_all(dir.join("profiles")).unwrap();
    let coder = "name = \"coder\"\nlabel = \"Coder\"\nagent = \"fake-acp\"\n";
    fs::write(dir.join("profiles/coder.toml"), coder).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let cooee = Cooee::start(&dir, &shed.dir);
    let new =
        json!({ "agent": "fake-acp", "profile": "coder", "text": "echo one", "random_id": "one" });
    let (status, body) = cooee.post("/threads", &new);
    assert_eq!(status, 200, "{body}");
    let thread = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();

    let mut shell = pair(&cooee, &dir);
    let row = format!("t-{thread}");
    until(&mut shell, "the live sessions list", |s, v| {
        s.connected() && v["id"] == "sessions" && v.to_string().contains(&row)
    });
    assert_eq!(
        serde_json::to_value(shell.screen()).unwrap(),
        cooee.json("/screens/sessions")
    );
    assert!(draw(&mut shell).contains(&row));

    let key = format!("/screens/chat/{thread}");
    shell.event(Event::Open(key.clone()));
    until(&mut shell, "the chat", |s, v| {
        s.key() == key && v["id"] == "chat"
    });
    act(
        &mut shell,
        "chat",
        "say",
        "send",
        json!({ "text": "echo two" }),
    );
    until(&mut shell, "the reply", |_, v| {
        v.to_string().contains("\"two\"")
    });
    until(&mut shell, "the stored reply", |_, v| {
        *v == cooee.json(&key)
    });
    let drawn = draw(&mut shell);
    assert!(drawn.contains(&"say".to_owned()), "{drawn:?}");
    assert_eq!(
        drawn.iter().filter(|b| b.ends_with("-text")).count(),
        2,
        "{drawn:?}"
    );

    shell.back();
    until(&mut shell, "back on the sessions", |s, v| {
        s.key() == "/screens/sessions" && v["id"] == "sessions"
    });
}
