use std::sync::Arc;

use serde_json::json;

use crate::{Entry, Provider, Ran, Spec, Status, Usage, Window, login::one_line};

/// Claude Code: `claude auth status`'s JSON, `/usage`'s `N% used · resets …` lines (read in UTC, nonessential traffic
/// on), `CLAUDE_CONFIG_DIR` as its account and settings folder (herdsman#565), and an inherited
/// `CLAUDE_CODE_OAUTH_TOKEN` kept from its commands so the folder's own login answers (#183).
#[derive(Debug)]
pub struct Claude(Spec);

pub const ENTRY: Entry = Entry {
    name: "claude",
    defaults,
    build: |s| Arc::new(Claude(s)),
};

const TOKEN: &str = "CLAUDE_CODE_OAUTH_TOKEN";

fn defaults() -> Spec {
    let v = |s: &str| s.split(' ').map(str::to_owned).collect();
    Spec {
        name: "claude".into(),
        agent: "claude-acp".into(),
        login: v("claude auth login"),
        status: v("claude auth status"),
        install: "npm install -g @anthropic-ai/claude-code".into(),
        home: "CLAUDE_CONFIG_DIR".into(),
        logout: v("claude auth logout"),
        settings: "CLAUDE_CONFIG_DIR".into(),
        usage: v("env -u CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC TZ=UTC claude -p /usage"),
        ..Spec::default()
    }
}

impl Provider for Claude {
    fn spec(&self) -> &Spec {
        &self.0
    }

    fn read_status(&self, ran: &Ran) -> Status {
        let Ok(v @ serde_json::Value::Object(_)) = serde_json::from_str(&ran.stdout) else {
            return crate::login::plain_status(ran);
        };
        let who = v["email"].as_str().or(v["orgName"].as_str());
        let s = match (v["loggedIn"].as_bool(), v["authMethod"].as_str(), who) {
            (Some(true), Some(how), Some(who)) => {
                Status::said(true, &format!("signed in · {how} ({who})"))
            }
            (Some(true), Some(how), None) => Status::said(true, &format!("signed in · {how}")),
            (Some(true), None, _) => Status::said(true, "signed in"),
            (Some(false), ..) => Status::said(false, "signed out"),
            (None, ..) => Status::said(ran.ok, "status unknown"),
        };
        Status {
            who: who.map(one_line),
            ..s
        }
    }

    fn scrub(&self) -> &'static [&'static str] {
        &[TOKEN]
    }

    fn token_var(&self) -> Option<&'static str> {
        Some(TOKEN)
    }

    fn read_usage(&self, out: &str, now: u64) -> Option<Usage> {
        let read =
            |label: &str| (out.lines()).find_map(|l| window(l.trim().strip_prefix(label)?, now));
        let u = Usage {
            five_hour: read("Current session"),
            seven_day: read("Current week (all models)"),
            read_at: now,
        };
        (u.five_hour.is_some() || u.seven_day.is_some()).then_some(u)
    }

    fn no_personal_setup(&self) -> Option<serde_json::Value> {
        let excludes = ["{/Users/*,/home/*,/root}/.claude/{CLAUDE.md,rules/**}"];
        let options = json!({
            "settingSources": ["project", "local"],
            "strictMcpConfig": true,
            "settings": { "claudeMdExcludes": excludes },
        });
        Some(json!({ "claudeCode": { "options": options } }))
    }
}

fn window(line: &str, now: u64) -> Option<Window> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let (pct, when) = line.strip_prefix(':')?.split_once("% used · resets ")?;
    let when = when.trim().strip_suffix(" (UTC)")?;
    let (date, time) = when
        .rsplit_once(" at ")
        .map_or((None, when), |(d, t)| (Some(d), t));
    let (time, half) = match time.strip_suffix("pm") {
        Some(t) => (t, 12),
        None => (time.strip_suffix("am")?, 0),
    };
    let (h, m) = time.split_once(':').unwrap_or((time, "0"));
    let secs = (h.parse::<u64>().ok()? % 12 + half) * 3600 + m.parse::<u64>().ok()? * 60;
    let resets_at = match date {
        None => {
            let t = now - now % 86_400 + secs;
            if t < now { t + 86_400 } else { t }
        }
        Some(d) => {
            let (mon, day) = d.split_once(' ')?;
            let month = MONTHS.iter().position(|m| *m == mon)? as u64 + 1;
            let (day, year) = (day.parse().ok()?, 1970 + now / 31_556_952);
            (year - 1..=year + 1)
                .filter_map(|y| Some(civil_days(y, month, day)? * 86_400 + secs))
                .find(|t| t + 86_400 >= now)?
        }
    };
    Some(Window {
        used_pct: pct.trim().parse().ok()?,
        resets_at,
    })
}

fn civil_days(y: u64, m: u64, d: u64) -> Option<u64> {
    let y = if m <= 2 { y - 1 } else { y };
    let doy = (153 * ((m + 9) % 12) + 2) / 5 + d - 1;
    let doe = (y % 400) * 365 + (y % 400) / 4 - (y % 400) / 100 + doy;
    ((y / 400) * 146_097 + doe).checked_sub(719_468)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_windows_from_the_usage_lines() {
        let day = |y, m, d| civil_days(y, m, d).unwrap() * 86_400;
        let now = day(2026, 11, 10) + 9 * 3600;
        let w = |line: &str, now| window(line, now).map(|w| (w.used_pct, w.resets_at));
        assert_eq!(day(1970, 1, 1), 0);
        assert_eq!(
            w(": 4% used · resets Nov 16 at 2am (UTC)", now),
            Some((4, day(2026, 11, 16) + 2 * 3600))
        );
        assert_eq!(
            w(": 83% used · resets 3:30pm (UTC)", now),
            Some((83, day(2026, 11, 10) + 15 * 3600 + 1800))
        );
        assert_eq!(
            w(": 12% used · resets 4:59am (UTC)", now),
            Some((12, day(2026, 11, 11) + 4 * 3600 + 59 * 60))
        );
        assert_eq!(
            w(": 1% used · resets Jan 2 at 12am (UTC)", day(2026, 12, 30)),
            Some((1, day(2027, 1, 2)))
        );
        assert_eq!(
            w(": 4% used · resets Nov 16 at 2am (Asia/Bangkok)", now),
            None
        );
        assert_eq!(
            w(" (Fable): 0% used · resets Nov 16 at 2am (UTC)", now),
            None
        );
    }

    #[test]
    fn status_from_the_json_status() {
        let c = Claude(defaults());
        let read = |ok, stdout: &str| {
            let s = c.read_status(&Ran {
                ok,
                stdout: stdout.to_owned(),
                stderr: String::new(),
            });
            (s.signed_in, s.line, s.who)
        };
        assert_eq!(
            read(
                true,
                r#"{"loggedIn": true, "authMethod": "claude.ai", "email": "o@example.com"}"#
            ),
            (
                true,
                "signed in · claude.ai (o@example.com)".to_owned(),
                Some("o@example.com".to_owned())
            )
        );
        assert_eq!(
            read(true, r#"{"loggedIn": true, "orgName": "Acme"}"#),
            (true, "signed in".to_owned(), Some("Acme".to_owned()))
        );
        assert_eq!(
            read(false, r#"{"loggedIn": false, "authMethod": "none"}"#),
            (false, "signed out".to_owned(), None)
        );
        assert_eq!(
            read(true, r#"{"a": 1}"#),
            (true, "status unknown".to_owned(), None)
        );
        assert_eq!(
            read(true, "Logged in\n as owner"),
            (true, "Logged in as owner".to_owned(), None)
        );
    }
}
