use std::{fs, thread, time::Duration};

use serde_json::json;

use crate::setup::{Agent, Mcp, Setup, files, named};
use crate::support::{Cooee, FakeShed, agent, scratch, version};

#[test]
fn mail_sends_only_on_its_card_or_an_allowed_recipient_and_a_read_code_reaches_no_transcript() {
    let dir = scratch("mcp-mail");
    let sent = dir.join("sent.eml");
    let coder = format!(
        "name = \"Coder\"\nagent = \"fake-acp\"\n\n[mail]\non = true\naddress = \"coder@owner.example\"\nhook = \"MAIL_HOOK\"\nsmtp = \"file://{}\"\n",
        sent.display()
    );
    let home = agent(&dir, "coder", &coder, "");
    let rule = "[[rule]]\nid = \"m1\"\neffect = \"allow\"\nkind = \"tool\"\npattern = \"mail/*@shop.example\"\n";
    fs::write(home.join("rules.toml"), rule).unwrap();
    agent(
        &dir,
        "writer",
        "name = \"Writer\"\nagent = \"fake-acp\"\n",
        "",
    );
    let mails = json!([
        { "id": "m-2", "from": "no-reply@shop.example", "subject": "Your code", "date": "2026-11-20 09:01",
          "text": "Your verification code is 482913.", "attachments": [["receipt.txt", "Total 12.00"]] },
        { "id": "m-1", "from": "ada@shop.example", "subject": "Hello", "date": "2026-11-20 09:00", "text": "Hi" }
    ]);
    fs::create_dir_all(dir.join("data/mail")).unwrap();
    fs::write(dir.join("data/mail/coder.json"), mails.to_string()).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    shed.plain("login");
    let s = Setup {
        cooee: Cooee::start_with(&dir, &shed.dir, ""),
        dir,
        shed,
        token: String::new(),
    };
    let (_, servers) = s.thread("writer");
    let mut off = Mcp::start(named(&servers, "cooee"), Agent::Claude);
    assert!(off.tools().contains(&"mail".to_owned()));
    let none = off.run("mail", &json!({ "list": true })).unwrap();
    assert!(none.starts_with("NO_INBOX"), "{none}");
    let (id, servers) = s.thread("coder");
    let mut m = Mcp::start(named(&servers, "cooee"), Agent::Codex);

    let listed = m.run("mail", &json!({ "list": true })).unwrap();
    assert!(
        listed.starts_with("[m-2] 2026-11-20 09:01 · no-reply@shop.example · Your code\n[m-1]"),
        "{listed}"
    );
    let read = m.run("mail", &json!({ "read": "m-2" })).unwrap();
    assert!(read.contains("verification code is [code]."), "{read}");
    assert!(
        read.contains("Attachment receipt.txt:\nTotal 12.00"),
        "{read}"
    );
    assert!(!read.contains("482913"), "{read}");
    assert!(
        m.run("mail", &json!({ "list": true }))
            .unwrap()
            .starts_with("[m-1]"),
        "read is marked read"
    );
    let plain = m.run("mail", &json!({})).unwrap();
    assert!(plain.starts_with("[m-1]"), "no ask lists: {plain}");
    let typed = m
        .run(
            "secret",
            &json!({ "name": "MAIL_CODE", "terminal": "login" }),
        )
        .unwrap();
    assert_eq!(typed, "secret MAIL_CODE entered");
    assert_eq!(s.shed.sent("login"), ["482913"]);
    let mut kept = s.cooee.json(&format!("/threads/{id}/messages")).to_string();
    files(&s.dir.join("data/threads"), &mut kept);
    kept.extend(s.shed.sent(&format!("cooee-{id}")));
    assert!(!kept.contains("482913"), "the code reached the transcript");

    let outgoing =
        |to: &str| json!({ "send": { "to": to, "subject": "Hello", "text": format!("Hi {to}") } });
    let asked = m.run("mail", &outgoing("bob@bank.example")).unwrap();
    assert!(asked.starts_with("Asked the owner"), "{asked}");
    thread::sleep(Duration::from_millis(300));
    assert!(!sent.exists(), "sent before its card");
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let card = items
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|i| i["form"] == "tool")
        .unwrap()
        .clone();
    assert!(
        card["tool"]
            .as_str()
            .unwrap()
            .contains("send mail to bob@bank.example"),
        "{card}"
    );
    let (status, body) = s.cooee.post(
        &format!("/threads/{id}/approve"),
        &json!({ "item": card["id"], "option": "decline" }),
    );
    assert_eq!(status, 200, "{body}");
    assert!(!sent.exists(), "a declined card sends nothing");
    m.run("mail", &outgoing("bob@bank.example")).unwrap();
    let items = s.cooee.json(&format!("/threads/{id}/messages"))["items"].clone();
    let card = items
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|i| i["form"] == "tool")
        .unwrap()
        .clone();
    let (status, body) = s.cooee.post(
        &format!("/threads/{id}/approve"),
        &json!({ "item": card["id"], "option": "approve" }),
    );
    assert_eq!(status, 200, "{body}");
    let eml = fs::read_to_string(&sent).unwrap();
    assert!(
        eml.starts_with("From: coder@owner.example\r\nTo: bob@bank.example\r\n"),
        "{eml}"
    );
    assert!(eml.contains("Hi bob@bank.example"), "{eml}");
    fs::remove_file(&sent).unwrap();

    assert_eq!(
        m.run("mail", &outgoing("ada@shop.example")).unwrap(),
        "sent to ada@shop.example"
    );
    assert!(
        fs::read_to_string(&sent)
            .unwrap()
            .contains("To: ada@shop.example\r\n")
    );
    let bad = m
        .run("mail", &outgoing("ada@shop.example\r\nBcc: all@x.example"))
        .unwrap();
    assert!(bad.starts_with("BAD_MAIL"), "{bad}");
    assert!(!fs::read_to_string(&sent).unwrap().contains("Bcc"));
}
