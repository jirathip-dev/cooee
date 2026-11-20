use std::{
    io::{self, Read},
    os::unix::process::CommandExt,
    process::Stdio,
    time::Duration,
};

use cooee_core::{
    Body, clock,
    routines::{Mode, Outcome, Place, Routine, Script, Skip},
};

use super::Runner;
use crate::push::Note;

const LIMIT: Duration = Duration::from_secs(30);

const CAP: usize = 4000;

#[derive(Debug)]
pub(super) struct Ran {
    ok: bool,
    said: String,
}

pub(super) fn exec(cmd: &str, limit: Duration) -> Result<Ran, String> {
    let mut child = cooee_core::runner::command("sh")
        .arg("-c")
        .arg(format!("exec 2>&1\n{cmd}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("sh: {e}"))?;
    let out = child.stdout.take();
    let read = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut out) = out {
            let cap = u64::try_from(CAP * 4 + 1).unwrap_or(u64::MAX);
            out.by_ref().take(cap).read_to_end(&mut buf).ok();
            io::copy(&mut out, &mut io::sink()).ok();
        }
        buf
    });
    let deadline = clock::instant() + limit;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if clock::instant() < deadline => clock::sleep(Duration::from_millis(20)),
            _ => {
                cooee_core::group::signal(&mut child, "-KILL");
                child.kill().ok();
                child.wait().ok();
                break None;
            }
        }
    };
    let text = String::from_utf8_lossy(&read.join().unwrap_or_default()).into_owned();
    let mut out: String = text.trim().chars().take(CAP).collect();
    if text.trim().chars().nth(CAP).is_some() {
        out.push('…');
    }
    let head = match status.map(|s| s.code()) {
        Some(Some(0)) => String::new(),
        Some(Some(n)) => format!("exit {n}\n"),
        Some(None) => "killed by a signal\n".to_owned(),
        None => format!("timed out after {limit:?}\n"),
    };
    Ok(Ran {
        ok: head.is_empty(),
        said: head + &out,
    })
}

impl Runner {
    pub(super) fn script(&self, r: &Routine, s: &Script, key: &str) -> Outcome {
        let ran = match exec(&s.run, LIMIT) {
            Ok(ran) => ran,
            Err(why) => return Outcome::Failed { why },
        };
        let thread = match &r.place {
            Place::Chat { thread } => thread.clone(),
            Place::NewThread { .. } => r.made_in.clone(),
        };
        match (s.mode, ran.ok) {
            (Mode::Wake, true) => Outcome::Skipped { why: Skip::Quiet },
            (Mode::Wake, false) => match self.admit_wake(r, crate::hub::now()) {
                Ok(()) => self.prompt(r, &ran.said, key),
                Err(why) => Outcome::Skipped { why },
            },
            (Mode::Post, ok) => {
                let text = format!("⟳ {}: {}", r.name, ran.said);
                if let Err(e) = self.hub.say(&thread, Body::Note { text }) {
                    return Outcome::Failed { why: e.to_string() };
                }
                if !ok {
                    self.hub.push().send(&Note {
                        title: format!("Routine {} failed", r.name),
                        body: ran.said.lines().next().unwrap_or_default().to_owned(),
                        tag: r.id.clone(),
                        url: format!("/t/{thread}"),
                        item: 0,
                        say: false,
                    });
                }
                Outcome::Ran { thread }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use cooee_core::clock;

    use super::exec;

    #[test]
    fn a_command_says_its_exit_and_capped_output() {
        let ok = exec("echo fine; echo also >&2", Duration::from_secs(5)).unwrap();
        assert_eq!((ok.ok, ok.said.as_str()), (true, "fine\nalso"));
        let bad = exec("echo broke; exit 3", Duration::from_secs(5)).unwrap();
        assert_eq!((bad.ok, bad.said.as_str()), (false, "exit 3\nbroke"));
        let long = exec("printf '%020000d' 0", Duration::from_secs(5)).unwrap();
        assert_eq!(long.said.chars().count(), super::CAP + 1);
        assert!(long.said.ends_with('…'));
    }

    #[test]
    fn the_timeout_kills_the_command_and_what_it_started() {
        let started = clock::instant();
        let hung = exec(
            "sleep 600 & echo started; sleep 600",
            Duration::from_millis(300),
        )
        .unwrap();
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            started.elapsed()
        );
        assert_eq!(
            (hung.ok, hung.said.as_str()),
            (false, "timed out after 300ms\nstarted")
        );
    }
}
