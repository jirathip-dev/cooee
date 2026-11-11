use std::fmt::Write;

use cooee_core::shed_proto::Sample;

use crate::{Card, DAY, Problem};

pub(crate) fn size(mb: u64) -> String {
    if mb < 1024 {
        format!("{mb} MB")
    } else {
        #[expect(clippy::cast_precision_loss, reason = "a size shown to one decimal")]
        let gb = mb as f64 / 1024.0;
        format!("{gb:.1} GB")
    }
}

pub(crate) fn ago(now: u64, t: u64) -> String {
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
            #[expect(clippy::cast_precision_loss, reason = "a position in a drawing")]
            let x = (at.saturating_sub(start) as f64 / DAY as f64).min(1.0) * w;
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
    let _ = write!(
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
    );
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
        let _ = write!(
            out,
            "<article class=\"m{}\" aria-labelledby=\"m-{id}\"><h2 id=\"m-{id}\"><i aria-hidden=\"true\">●</i> \
             {name}</h2><p class=\"seen\">{seen}</p>",
            if c.online { " on" } else { " off" },
        );
        if let Some(e) = &c.error {
            let _ = write!(out, "<p class=\"er\">{}</p>", esc(e));
        }
        if let Some(s) = &c.now {
            let samples: Vec<&Sample> = c.history.iter().chain([s]).collect();
            let series = |f: fn(&Sample) -> f64| -> Vec<(u64, f64)> {
                samples.iter().map(|s| (s.at, f(s))).collect()
            };
            #[expect(clippy::cast_precision_loss, reason = "MiB and percents, drawn")]
            let (mem, swap, disk) = (
                series(|s| s.mem_pressure_pct as f64),
                series(|s| s.swap_used_mb as f64),
                series(|s| s.disk_free_mb as f64),
            );
            #[expect(clippy::cast_possible_truncation, reason = "MiB, shown")]
            #[expect(clippy::cast_sign_loss, reason = "MiB, never negative")]
            let mb = |v: f64| size(v as u64);
            out.push_str("<div class=\"ms\">");
            let load = (
                "load/cpu",
                format!("{:.2}", s.load_per_cpu),
                s.load_per_cpu >= 1.0,
            );
            let pts = series(|s| s.load_per_cpu);
            metric(
                &mut out,
                &format!("{id}-load"),
                load,
                &pts,
                |v| format!("{v:.2}"),
                now,
            );
            let critical = problems.contains(&Problem::MemoryCritical);
            let word = if critical { " critical" } else { "" };
            let m = ("memory", format!("{}%{word}", s.mem_pressure_pct), critical);
            metric(
                &mut out,
                &format!("{id}-mem"),
                m,
                &mem,
                |v| format!("{v:.0}%"),
                now,
            );
            let w = ("swap", size(s.swap_used_mb), false);
            metric(&mut out, &format!("{id}-swap"), w, &swap, mb, now);
            let low = problems.contains(&Problem::DiskLow);
            let word = if low { " low" } else { "" };
            let d = ("disk free", format!("{}{word}", size(s.disk_free_mb)), low);
            metric(&mut out, &format!("{id}-disk"), d, &disk, mb, now);
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
            let _ = write!(out, "<p class=\"tm\">terminals: {terminals}</p>");
        }
        out.push_str("</article>");
    }
    out
}
