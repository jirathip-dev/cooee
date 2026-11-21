//! A persona's inbox (#220): `<persona>@<domain>`, read and sent through one [`Backend`]: a catch-all router's signed
//! POSTs kept in `<data>/mail/<persona>.json`, or IMAP; sent over SMTP. Every result is capped and a verification
//! code is hidden from it.

use std::{
    fs,
    io::Write as _,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{Mutex, PoisonError},
};

use serde::{Deserialize, Serialize};

/// How many mails a list or a search shows.
pub const LISTED: usize = 10;

/// How many characters a read shows.
pub const SHOWN: usize = 8000;

/// The secret name a hidden verification code is typed under.
pub const CODE: &str = "MAIL_CODE";

const KEPT: usize = 200;

static FILES: Mutex<()> = Mutex::new(());

/// A persona's `[mail]`: off unless `on`; each credential a secret's name, never its value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Inbox {
    /// Switched on.
    pub on: bool,
    /// Its address, `<persona>@<owner's domain>`.
    pub address: String,
    /// The secret a catch-all router signs its POSTs to `/hooks/mail/<persona>` with; empty: read over `imap`.
    pub hook: String,
    /// The IMAP mailbox URL, like `imaps://mail.example.com/INBOX`.
    pub imap: String,
    /// The SMTP URL it sends through, like `smtps://mail.example.com:465`.
    pub smtp: String,
    /// The IMAP and SMTP user.
    pub user: String,
    /// The secret holding that user's password.
    pub password: String,
}

/// One mail, its attachments as text.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mail {
    /// Its id in the inbox.
    pub id: String,
    /// Its sender.
    pub from: String,
    /// Its subject.
    pub subject: String,
    /// When it came, as shown.
    pub date: String,
    /// Its text.
    pub text: String,
    /// Its attachments, each `(name, text)`.
    pub attachments: Vec<(String, String)>,
    /// Read already.
    pub read: bool,
}

/// Where a persona's mail is read and sent.
pub trait Backend: Send {
    /// The newest mails first, their text possibly left empty.
    ///
    /// # Errors
    /// The inbox can't be read.
    fn mails(&self) -> Result<Vec<Mail>, String>;
    /// Mail `id`'s text with its attachments, marking it read.
    ///
    /// # Errors
    /// No such mail, or the inbox can't be read.
    fn open(&self, id: &str) -> Result<String, String>;
    /// Sends `text` to `to` under `subject`.
    ///
    /// # Errors
    /// The server refuses it.
    fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String>;
}

fn line(m: &Mail) -> String {
    format!(
        "[{}] {} · {} · {}",
        m.id,
        m.date,
        m.from,
        hide(&m.subject).0
    )
}

fn lines<'a>(mails: impl Iterator<Item = &'a Mail>) -> String {
    let shown: Vec<String> = mails.take(LISTED).map(line).collect();
    if shown.is_empty() {
        "no mail".to_owned()
    } else {
        shown.join("\n")
    }
}

/// The unread mails, newest first, one line each.
///
/// # Errors
/// The backend's.
pub fn list(b: &dyn Backend) -> Result<String, String> {
    Ok(lines(b.mails()?.iter().filter(|m| !m.read)))
}

/// The mails whose sender contains `from` and whose date contains `date`, newest first, one line each.
///
/// # Errors
/// The backend's.
pub fn search(b: &dyn Backend, from: &str, date: &str) -> Result<String, String> {
    let from = from.to_lowercase();
    let hit = |m: &&Mail| m.from.to_lowercase().contains(&from) && m.date.contains(date);
    Ok(lines(b.mails()?.iter().filter(hit)))
}

/// Mail `id` as text, marked read, and the verification code hidden from it.
///
/// # Errors
/// No such mail, or the backend's.
pub fn read(b: &dyn Backend, id: &str) -> Result<(String, Option<String>), String> {
    let m = b.mails()?.into_iter().find(|m| m.id == id);
    let m = m.ok_or_else(|| format!("NO_MAIL: no mail {id}"))?;
    let (subject, first) = hide(&m.subject);
    let (body, code) = hide(&b.open(id)?);
    let code = first.or(code);
    let text = format!(
        "From: {}\nSubject: {subject}\nDate: {}\n\n{body}",
        m.from, m.date
    );
    let told = code.as_ref().map_or(String::new(), |_| {
        format!(
            "A verification code was in it: have it typed with secret {CODE}; you never see it.\n\n"
        )
    });
    let all = told + &text;
    if all.chars().nth(SHOWN).is_none() {
        return Ok((all, code));
    }
    let shown: String = all.chars().take(SHOWN - 1).collect();
    Ok((shown + "…", code))
}

fn names_code(line: &str) -> bool {
    line.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
        let w = w.to_ascii_lowercase();
        ["code", "otp", "pin", "passcode", "2fa", "mfa", "token"].contains(&w.as_str())
            || w.starts_with("verif")
    })
}

/// `text` with each 4 to 8 digit run on a line naming a code, or on the next line when that one had none, as `[code]`, and the first one.
#[must_use]
pub fn hide(text: &str) -> (String, Option<String>) {
    let (mut out, mut code, mut after) = (String::new(), None, false);
    for line in text.split_inclusive('\n') {
        let named = names_code(line);
        if !(named || after) {
            out.push_str(line);
            continue;
        }
        let mut hid = false;
        for word in line.split_inclusive(|c: char| !c.is_ascii_alphanumeric()) {
            let run = word.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
            if (4..=8).contains(&run.len()) && run.bytes().all(|b| b.is_ascii_digit()) {
                code.get_or_insert_with(|| run.to_owned());
                out.push_str("[code]");
                out.push_str(word.get(run.len()..).unwrap_or_default());
                hid = true;
            } else {
                out.push_str(word);
            }
        }
        if named || !line.trim().is_empty() {
            after = named && !hid;
        }
    }
    (out, code)
}

/// Whether `to` is one address and `subject` one line.
///
/// # Errors
/// `BAD_MAIL` and why.
pub fn addressed(to: &str, subject: &str) -> Result<(), String> {
    let one = !to.contains(char::is_whitespace) && to.split('@').count() == 2;
    if !one || subject.contains(['\r', '\n']) {
        return Err(format!(
            "BAD_MAIL: send to one address under a one-line subject, not {to:?}"
        ));
    }
    Ok(())
}

/// The backend of `persona`'s `inbox`, its password read from `data`'s secrets.
///
/// # Errors
/// A password secret that can't be read.
pub fn open(data: &Path, persona: &str, inbox: &Inbox) -> Result<Box<dyn Backend>, String> {
    let password = if inbox.password.is_empty() {
        String::new()
    } else {
        crate::secrets::get(data, &inbox.password)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("no secret:{}", inbox.password))?
    };
    let smtp = Smtp {
        inbox: inbox.clone(),
        password,
        out: data.join("mail").join(format!("{persona}.eml")),
    };
    Ok(if inbox.hook.is_empty() {
        Box::new(Imap(smtp))
    } else {
        Box::new(Hooked {
            file: file(data, persona),
            smtp,
        })
    })
}

fn file(data: &Path, persona: &str) -> PathBuf {
    data.join("mail").join(format!("{persona}.json"))
}

fn load(file: &Path) -> Vec<Mail> {
    let text = fs::read_to_string(file).unwrap_or_default();
    serde_json::from_str(&text).unwrap_or_default()
}

fn save(file: &Path, mails: &[Mail]) -> Result<(), String> {
    let bytes = serde_json::to_vec(mails).map_err(|e| e.to_string())?;
    crate::write_private(file, &bytes).map_err(|e| e.to_string())
}

/// Keeps mail `m` that a catch-all POST brought `persona`, newest first: whether it is new.
///
/// # Errors
/// The inbox can't be written.
pub fn keep(data: &Path, persona: &str, m: Mail) -> Result<bool, String> {
    let _held = FILES.lock().unwrap_or_else(PoisonError::into_inner);
    let file = file(data, persona);
    let mut mails = load(&file);
    if mails.iter().any(|o| o.id == m.id) {
        return Ok(false);
    }
    mails.insert(0, m);
    mails.truncate(KEPT);
    save(&file, &mails).map(|()| true)
}

struct Hooked {
    file: PathBuf,
    smtp: Smtp,
}

impl Backend for Hooked {
    fn mails(&self) -> Result<Vec<Mail>, String> {
        Ok(load(&self.file))
    }

    fn open(&self, id: &str) -> Result<String, String> {
        let _held = FILES.lock().unwrap_or_else(PoisonError::into_inner);
        let mut mails = load(&self.file);
        let m = mails.iter_mut().find(|m| m.id == id);
        let m = m.ok_or_else(|| format!("NO_MAIL: no mail {id}"))?;
        m.read = true;
        let mut text = m.text.clone();
        for (name, body) in &m.attachments {
            text = text + "\n\nAttachment " + name + ":\n" + body;
        }
        save(&self.file, &mails).map(|()| text)
    }

    fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        self.smtp.send(to, subject, text)
    }
}

struct Smtp {
    inbox: Inbox,
    password: String,
    out: PathBuf,
}

fn quoted(s: &str) -> String {
    let s = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{}\"", s.replace('\n', "\\n"))
}

impl Smtp {
    fn curl(&self, url: &str, extra: &str) -> Result<String, String> {
        let mut config = format!("silent\nshow-error\nmax-time = 30\nurl = {}\n", quoted(url));
        if !self.inbox.user.is_empty() {
            let user = format!("{}:{}", self.inbox.user, self.password);
            config = config + "user = " + &quoted(&user) + "\n";
        }
        config.push_str(extra);
        let mut c = crate::runner::command("curl")
            .args(["-K", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("curl: {e}"))?;
        if let Some(mut stdin) = c.stdin.take() {
            stdin
                .write_all(config.as_bytes())
                .map_err(|e| e.to_string())?;
        }
        let out = c.wait_with_output().map_err(|e| e.to_string())?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_owned())
        }
    }

    fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        if self.inbox.smtp.is_empty() {
            return Err("NO_SMTP: this inbox has no smtp to send through".to_owned());
        }
        let message = format!(
            "From: {}\r\nTo: {to}\r\nSubject: {subject}\r\n\r\n{text}\r\n",
            self.inbox.address
        );
        crate::write_private(&self.out, message.as_bytes()).map_err(|e| e.to_string())?;
        let extra = format!(
            "mail-from = {}\nmail-rcpt = {}\nupload-file = {}\n",
            quoted(&self.inbox.address),
            quoted(to),
            quoted(&self.out.to_string_lossy())
        );
        let sent = self.curl(&self.inbox.smtp, &extra);
        fs::remove_file(&self.out).ok();
        sent.map(|_| ())
    }
}

struct Imap(Smtp);

impl Backend for Imap {
    fn mails(&self) -> Result<Vec<Mail>, String> {
        let url = &self.0.inbox.imap;
        let all = self.0.curl(url, "request = \"UID SEARCH ALL\"\n")?;
        let uids: Vec<u64> = all
            .split_whitespace()
            .filter_map(|w| w.parse().ok())
            .collect();
        let (Some(lo), Some(hi)) = (uids.iter().rev().take(50).min(), uids.last()) else {
            return Ok(Vec::new());
        };
        let fetch = format!(
            "request = \"UID FETCH {lo}:{hi} (UID FLAGS BODY.PEEK[HEADER.FIELDS (FROM SUBJECT DATE)])\"\n"
        );
        let got = self.0.curl(url, &fetch)?;
        let mut mails: Vec<Mail> = got.split("\n* ").map(header).collect();
        mails.retain(|m| !m.id.is_empty());
        mails.reverse();
        Ok(mails)
    }

    fn open(&self, id: &str) -> Result<String, String> {
        if id.is_empty() || !id.bytes().all(|b| b.is_ascii_digit()) {
            return Err(format!("NO_MAIL: no mail {id}"));
        }
        let url = format!("{}/;UID={id}/;SECTION=TEXT", self.0.inbox.imap);
        self.0.curl(&url, "")
    }

    fn send(&self, to: &str, subject: &str, text: &str) -> Result<(), String> {
        self.0.send(to, subject, text)
    }
}

fn header(fetched: &str) -> Mail {
    let id = fetched.split_once("UID ").map_or("", |(_, rest)| {
        rest.split(|c: char| !c.is_ascii_digit())
            .next()
            .unwrap_or_default()
    });
    let field = |name: &str| {
        let found = fetched.lines().find_map(|l| l.strip_prefix(name));
        found.unwrap_or_default().trim().to_owned()
    };
    Mail {
        id: id.to_owned(),
        from: field("From:"),
        subject: field("Subject:"),
        date: field("Date:"),
        read: fetched.contains("\\Seen"),
        ..Mail::default()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Default)]
    struct Fake {
        mails: Mutex<Vec<Mail>>,
        sent: Mutex<Vec<String>>,
    }

    impl Backend for Fake {
        fn mails(&self) -> Result<Vec<Mail>, String> {
            Ok(self.mails.lock().unwrap().clone())
        }

        fn open(&self, id: &str) -> Result<String, String> {
            let mut mails = self.mails.lock().unwrap();
            let m = mails.iter_mut().find(|m| m.id == id).ok_or("no such")?;
            m.read = true;
            Ok(m.text.clone())
        }

        fn send(&self, to: &str, _: &str, _: &str) -> Result<(), String> {
            self.sent.lock().unwrap().push(to.to_owned());
            Ok(())
        }
    }

    fn fake(n: usize) -> Fake {
        let mail = |i: usize| Mail {
            id: format!("m{i}"),
            from: if i.is_multiple_of(2) {
                "ada@shop.example"
            } else {
                "bob@bank.example"
            }
            .to_owned(),
            subject: format!("note {i}"),
            date: format!("2026-11-{:02} 09:00", 1 + i % 3),
            text: "x".repeat(9000),
            ..Mail::default()
        };
        Fake {
            mails: Mutex::new((0..n).rev().map(mail).collect()),
            ..Fake::default()
        }
    }

    #[test]
    fn list_and_search_show_at_most_ten_lines_newest_first_and_list_only_unread() {
        let b = fake(25);
        let listed = list(&b).unwrap();
        let lines: Vec<&str> = listed.lines().collect();
        assert_eq!(lines.len(), LISTED, "{listed}");
        assert!(lines[0].starts_with("[m24] "), "{listed}");
        assert!(lines[9].starts_with("[m15] "), "{listed}");
        read(&b, "m24").unwrap();
        assert!(
            list(&b).unwrap().starts_with("[m23] "),
            "a read mail isn't listed"
        );

        let found = search(&b, "BOB@", "").unwrap();
        assert_eq!(found.lines().count(), LISTED, "{found}");
        assert!(
            found.lines().all(|l| l.contains("bob@bank.example")),
            "{found}"
        );
        let day = search(&b, "ada", "2026-11-01").unwrap();
        let ids: Vec<&str> = day.lines().map(|l| l.split(']').next().unwrap()).collect();
        assert_eq!(ids, ["[m24", "[m18", "[m12", "[m6", "[m0"], "{day}");
        assert!(day.contains("[m24]"), "a search shows read mail too");
        assert_eq!(search(&b, "carol", "").unwrap(), "no mail");
    }

    #[test]
    fn a_read_is_capped_marks_the_mail_read_and_hides_its_code() {
        let b = fake(1);
        let (text, code) = read(&b, "m0").unwrap();
        assert_eq!(text.chars().count(), SHOWN, "{}", text.len());
        assert!(text.starts_with("From: ada@shop.example\nSubject: note 0\n"));
        assert!(text.ends_with("x…"));
        assert_eq!(code, None);
        assert!(b.mails().unwrap()[0].read);
        assert!(read(&b, "m9").unwrap_err().starts_with("NO_MAIL"));

        b.mails.lock().unwrap()[0] = Mail {
            id: "c".to_owned(),
            subject: "Your code: 482913".to_owned(),
            text: "Hi,\nUse verification code 482913 to sign in.\nOrder 55512 ships Monday.\n"
                .to_owned(),
            ..Mail::default()
        };
        assert_eq!(list(&b).unwrap(), "[c]  ·  · Your code: [code]");
        let (text, code) = read(&b, "c").unwrap();
        assert_eq!(code.as_deref(), Some("482913"));
        assert!(!text.contains("482913"), "{text}");
        assert!(
            text.contains("Use verification code [code] to sign in."),
            "{text}"
        );
        assert!(
            text.contains("Order 55512 ships"),
            "a number off a code line stays: {text}"
        );
        assert!(
            text.starts_with("A verification code was in it: have it typed with secret MAIL_CODE"),
            "{text}"
        );
        assert!(b.sent.lock().unwrap().is_empty());
    }

    #[test]
    fn a_code_on_its_own_line_or_under_another_label_is_hidden() {
        for body in [
            "Your verification code is:\n\n482913\nthanks\n",
            "Your OTP is 482913\n",
            "Your sign-in PIN: 482913 expires\n",
        ] {
            let (text, code) = hide(body);
            assert_eq!(code.as_deref(), Some("482913"), "{body}");
            assert!(!text.contains("482913"), "{text}");
        }
        assert_eq!(hide("Shipping 4 items, order 55512\n").1, None);
    }
}
