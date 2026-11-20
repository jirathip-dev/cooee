use std::fmt::Write as _;

use serde_json::{Value, json};

use super::{Api, arg, cut, limit, limited, list, schema};

pub(super) fn tools() -> Value {
    json!([
        {
            "name": "list_calendars",
            "description": "List the owner's Google calendars (read-only): each one's id and name.",
            "inputSchema": schema(&limited(json!({})), &[]),
        },
        {
            "name": "list_events",
            "description": "List events of one Google calendar (read-only) in time order, from a time on: each event's id, start, end, title, place and the start of its notes.",
            "inputSchema": schema(&limited(json!({
                "calendar": { "type": "string", "description": "A calendar id from list_calendars (primary)" },
                "from": { "type": "string", "description": "RFC 3339, as 2026-11-20T09:00:00Z (now)" },
                "to": { "type": "string", "description": "RFC 3339; none: no end" },
                "query": { "type": "string", "description": "Words the event has" },
            })), &[]),
        },
    ])
}

fn when(e: &Value, at: &str) -> String {
    let t = e.get(at).map_or(&Value::Null, |t| t);
    Some(arg(t, "dateTime"))
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| arg(t, "date"))
        .to_owned()
}

pub(super) fn call(api: &Api<'_>, tool: &str, args: &Value) -> Result<String, String> {
    let mut out = String::new();
    match tool {
        "list_calendars" => {
            let got = api.get(
                &["users", "me", "calendarList"],
                &[("maxResults", &limit(args))],
            )?;
            for c in list(&got, "items") {
                let primary = if c.get("primary") == Some(&Value::Bool(true)) {
                    " (primary)"
                } else {
                    ""
                };
                writeln!(out, "{} · {}{primary}", arg(c, "id"), arg(c, "summary")).ok();
            }
        }
        "list_events" => {
            let now = format!(
                "{}:00Z",
                cooee_core::routines::local(api.now, 0).replace(' ', "T")
            );
            let from = Some(arg(args, "from"))
                .filter(|f| !f.is_empty())
                .unwrap_or(&now);
            let cal = Some(arg(args, "calendar"))
                .filter(|c| !c.is_empty())
                .unwrap_or("primary");
            let max = limit(args);
            let mut q = vec![
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("timeMin", from),
                ("maxResults", max.as_str()),
            ];
            q.extend(
                [("timeMax", arg(args, "to")), ("q", arg(args, "query"))]
                    .into_iter()
                    .filter(|(_, v)| !v.is_empty()),
            );
            let got = api.get(&["calendars", cal, "events"], &q)?;
            for e in list(&got, "items") {
                let s = |k: &str| e[k].as_str().unwrap_or_default();
                writeln!(
                    out,
                    "{} · {} – {} · {} · {}",
                    s("id"),
                    when(e, "start"),
                    when(e, "end"),
                    s("summary"),
                    s("location")
                )
                .ok();
                if !s("description").is_empty() {
                    writeln!(out, "  {}", cut(s("description"), 500)).ok();
                }
            }
        }
        _ => {
            return Err(format!(
                "{tool} isn't a Google Calendar tool: it reads only"
            ));
        }
    }
    Ok(if out.is_empty() {
        "Nothing found.".to_owned()
    } else {
        out
    })
}
