use std::fmt::Write;

use cooee_core::shed_proto::Sample;

use crate::{Card, DAY, Problem};

pub(crate) fn float(n: u64) -> f64 {
    let half = |x: u64| f64::from(u32::try_from(x).unwrap_or(u32::MAX));
    half(n >> 32) * 4_294_967_296.0 + half(n & u64::from(u32::MAX))
}

pub(crate) fn size(mb: f64) -> String {
    if mb < 1024.0 {
        format!("{mb:.0} MB")
    } else {
        format!("{:.1} GB", mb / 1024.0)
    }
}

/// How long before `now` unix second `t` was: `just now`, `12 min ago`, `3 h ago`, `2 d ago`.
#[must_use]
pub fn ago(now: u64, t: u64) -> String {
    match now.saturating_sub(t) {
        0..60 => "just now".to_owned(),
        s @ 60..3600 => format!("{} min ago", s / 60),
        s @ 3600..DAY => format!("{} h ago", s / 3600),
        s => format!("{} d ago", s / DAY),
    }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn line(points: &[(u64, f64)], now: u64, w: f64, h: f64) -> String {
    let lo = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let hi = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let start = now.saturating_sub(DAY);
    points
        .iter()
        .map(|&(at, v)| {
            let x = (float(at.saturating_sub(start)) / float(DAY)).min(1.0) * w;
            let y = if hi > lo {
                h - (v - lo) / (hi - lo) * h
            } else {
                h / 2.0
            };
            format!("{x:.1},{y:.1}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn metric(
    out: &mut String,
    id: &str,
    (name, value, bad): (&str, String, bool),
    points: &[(u64, f64)],
    show: impl Fn(f64) -> String,
    now: u64,
) {
    let lo = points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let hi = points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    let range = if points.is_empty() {
        "no samples yet".to_owned()
    } else {
        format!("low {} · high {}", show(lo), show(hi))
    };
    write!(
        out,
        "<details id=\"{id}\"><summary><span class=\"k\">{name}</span><b{}>{value}</b>\
         <svg class=\"sp\" viewBox=\"0 0 100 20\" preserveAspectRatio=\"none\" aria-hidden=\"true\">\
         <polyline points=\"{}\"/></svg></summary>\
         <div class=\"ch\"><svg viewBox=\"0 0 300 100\" preserveAspectRatio=\"none\" role=\"img\" \
         aria-label=\"{name}, last 24 h: {range}\"><polyline points=\"{}\"/></svg>\
         <p><span>24 h ago</span><span>{range}</span><span>now</span></p></div></details>",
        if bad { " class=\"bad\"" } else { "" },
        line(points, now, 100.0, 20.0),
        line(points, now, 300.0, 100.0),
    ).ok();
}

fn sample(out: &mut String, (card, s): (&Card, &Sample), id: &str, problems: &[Problem], now: u64) {
    let samples: Vec<&Sample> = card.history.iter().chain([s]).collect();
    let series = |f: fn(&Sample) -> f64| -> Vec<(u64, f64)> {
        samples.iter().map(|s| (s.at, f(s))).collect()
    };
    let (mem, swap, disk) = (
        series(|s| float(s.mem_pressure_pct)),
        series(|s| float(s.swap_used_mb)),
        series(|s| float(s.disk_free_mb)),
    );
    let mb = size;
    out.push_str("<div class=\"ms\">");
    let load = (
        "load/cpu",
        format!("{:.2}", s.load_per_cpu),
        s.load_per_cpu >= 1.0,
    );
    let pts = series(|s| s.load_per_cpu);
    metric(
        out,
        &format!("{id}-load"),
        load,
        &pts,
        |v| format!("{v:.2}"),
        now,
    );
    let critical = problems.contains(&Problem::MemoryCritical);
    let word = if critical { " critical" } else { "" };
    let memory = ("memory", format!("{}%{word}", s.mem_pressure_pct), critical);
    metric(
        out,
        &format!("{id}-mem"),
        memory,
        &mem,
        |v| format!("{v:.0}%"),
        now,
    );
    let swapped = ("swap", size(float(s.swap_used_mb)), false);
    metric(out, &format!("{id}-swap"), swapped, &swap, mb, now);
    let low = problems.contains(&Problem::DiskLow);
    let word = if low { " low" } else { "" };
    let free = (
        "disk free",
        format!("{}{word}", size(float(s.disk_free_mb))),
        low,
    );
    metric(out, &format!("{id}-disk"), free, &disk, mb, now);
    out.push_str("</div>");
    let terminals: Vec<String> = s
        .terminals
        .iter()
        .map(|(state, n)| format!("{n} {}", esc(state)))
        .collect();
    let terminals = if terminals.is_empty() {
        "none".to_owned()
    } else {
        terminals.join(" · ")
    };
    write!(out, "<p class=\"tm\">terminals: {terminals}</p>").ok();
}

/// The cards, or a line while shed hasn't been asked yet.
#[must_use]
pub fn render(cards: &[Card], now: u64) -> String {
    let mut out = String::new();
    if cards.is_empty() {
        out.push_str("<p class=\"mt\">asking shed…</p>");
    }
    for c in cards {
        let problems = c.problems();
        let name = esc(&c.name);
        let id: String = c
            .name
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
            .collect();
        let seen = match (c.online, c.last_seen) {
            (true, _) => "online".to_owned(),
            (false, Some(t)) => format!("offline · last seen {}", ago(now, t)),
            (false, None) => "offline · never seen".to_owned(),
        };
        write!(
            out,
            "<article class=\"m{}\" aria-labelledby=\"m-{id}\"><h2 id=\"m-{id}\"><i aria-hidden=\"true\">●</i> \
             {name}</h2><p class=\"seen\">{seen}</p>",
            if c.online { " on" } else { " off" },
        ).ok();
        if let Some(e) = &c.error {
            write!(out, "<p class=\"er\">{}</p>", esc(e)).ok();
        }
        if let Some(s) = &c.now {
            sample(&mut out, (c, s), &id, &problems, now);
        }
        out.push_str("</article>");
    }
    out
}
