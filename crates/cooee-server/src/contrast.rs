use std::collections::BTreeMap;

use serde::Serialize;

use crate::look::Scheme;

/// The tokens every scheme sets: the ones the gate checks.
pub const TOKENS: [&str; 19] = [
    "--bg", "--card", "--edge", "--line", "--scrim", "--fg", "--mute", "--l", "--link", "--g",
    "--a", "--r", "--p", "--c", "--fk-l", "--fk-p", "--fk-c", "--fk-g", "--fk-a",
];

type Rgb = [f64; 3];

pub(crate) fn hex(s: &str) -> Option<Rgb> {
    let n = u32::from_str_radix(s.strip_prefix('#')?.get(..6)?, 16).ok()?;
    Some([n >> 16, (n >> 8) & 255, n & 255].map(f64::from))
}

fn lin(c: f64) -> f64 {
    let c = c / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn lum(c: Rgb) -> f64 {
    0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])
}

pub(crate) fn ratio(a: Rgb, b: Rgb) -> f64 {
    let (x, y) = (lum(a), lum(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

fn over(c: Rgb, bg: Rgb, p: f64) -> Rgb {
    let mix = |x: f64, y: f64| x * p + y * (1.0 - p);
    let ([red, green, blue], [back_red, back_green, back_blue]) = (c, bg);
    [
        mix(red, back_red),
        mix(green, back_green),
        mix(blue, back_blue),
    ]
}

fn lab(rgb: Rgb) -> Rgb {
    let lc = rgb.map(lin);
    let [cx, cy, cz] = [
        [0.4124, 0.3576, 0.1805],
        [0.2126, 0.7152, 0.0722],
        [0.0193, 0.1192, 0.9505],
    ]
    .map(|r| r[0] * lc[0] + r[1] * lc[1] + r[2] * lc[2]);
    let curve = |t: f64| {
        if t > 216.0 / 24389.0 {
            t.cbrt()
        } else {
            (24389.0 / 27.0 * t + 16.0) / 116.0
        }
    };
    let (fx, fy, fz) = (curve(cx / 0.950_47), curve(cy), curve(cz / 1.088_83));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

#[allow(clippy::many_single_char_names, reason = "the formula's own names")]
pub(crate) fn de(c1: Rgb, c2: Rgb) -> f64 {
    let ([l1, a1, b1], [l2, a2, b2]) = (lab(c1), lab(c2));
    let rad = std::f64::consts::PI / 180.0;
    let c = f64::midpoint(a1.hypot(b1), a2.hypot(b2));
    let g = 0.5 * (1.0 - (c.powi(7) / (c.powi(7) + 25f64.powi(7))).sqrt());
    let (ap1, ap2) = (a1 * (1.0 + g), a2 * (1.0 + g));
    let (cp1, cp2) = (ap1.hypot(b1), ap2.hypot(b2));
    let h = |b: f64, a: f64| {
        let v = b.atan2(a) / rad;
        if v < 0.0 { v + 360.0 } else { v }
    };
    let (h1, h2, dl, dc) = (h(b1, ap1), h(b2, ap2), l2 - l1, cp2 - cp1);
    let dh = if cp1 * cp2 == 0.0 {
        0.0
    } else if h2 - h1 > 180.0 {
        h2 - h1 - 360.0
    } else if h2 - h1 < -180.0 {
        h2 - h1 + 360.0
    } else {
        h2 - h1
    };
    let d_h = 2.0 * (cp1 * cp2).sqrt() * (dh * rad / 2.0).sin();
    let (lm, cm) = (f64::midpoint(l1, l2), f64::midpoint(cp1, cp2));
    let hm = if cp1 * cp2 == 0.0 {
        h1 + h2
    } else if (h1 - h2).abs() > 180.0 {
        (h1 + h2 + 360.0) / 2.0
    } else {
        f64::midpoint(h1, h2)
    };
    let t = 1.0 - 0.17 * ((hm - 30.0) * rad).cos()
        + 0.24 * (2.0 * hm * rad).cos()
        + 0.32 * ((3.0 * hm + 6.0) * rad).cos()
        - 0.2 * ((4.0 * hm - 63.0) * rad).cos();
    let sl = 1.0 + 0.015 * (lm - 50.0).powi(2) / (20.0 + (lm - 50.0).powi(2)).sqrt();
    let (sc, sh) = (1.0 + 0.045 * cm, 1.0 + 0.015 * cm * t);
    let rt = -2.0
        * (cm.powi(7) / (cm.powi(7) + 25f64.powi(7))).sqrt()
        * (60.0 * (-((hm - 275.0) / 25.0).powi(2)).exp() * rad).sin();
    ((dl / sl).powi(2) + (dc / sc).powi(2) + (d_h / sh).powi(2) + rt * (dc / sc) * (d_h / sh))
        .sqrt()
}

/// One check of the scheme gate: the token it judges (the one to change to pass it), what it measures, its value and
/// minimum.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    /// The token a fix changes.
    pub token: &'static str,
    /// What is measured, as the gate says it (`--a on an amber group header`).
    pub what: String,
    /// The contrast ratio or ΔE2000.
    pub value: f64,
    /// The gate's minimum.
    pub min: f64,
}

/// Every check of herdsman's scheme gate (serve.js `gate`, via docs/design/look/gate.mjs): text tokens at 4.5:1 and
/// state colours at 3:1 on the background and cards, the needs-you and group tints, and the state colours ΔE2000 ≥ 20
/// apart.
///
/// # Errors
///
/// The first token missing or not `#rrggbb`.
pub fn checks(s: &Scheme) -> Result<Vec<Check>, String> {
    let mut c = BTreeMap::new();
    for k in TOKENS {
        match s.tokens.get(k).and_then(|v| hex(v)) {
            Some(v) => drop(c.insert(k, v)),
            None => return Err(format!("{k}: missing or not #rrggbb")),
        }
    }
    let mut out = Vec::new();
    let mut check = |token, what: String, value, min| {
        out.push(Check {
            token,
            what,
            value,
            min,
        });
    };
    let c = |k: &str| c.get(k).copied().unwrap_or_default();
    let (bg, r, a) = (c("--bg"), c("--r"), c("--a"));
    let tints = [
        ("the needs-you card", over(r, bg, 0.15)),
        ("a red group header", over(r, bg, 0.12)),
        ("an amber group header", over(a, bg, 0.12)),
    ];
    for b in ["--bg", "--card"] {
        for k in [
            "--fg", "--mute", "--l", "--link", "--fk-l", "--fk-p", "--fk-c", "--fk-g", "--fk-a",
        ] {
            check(k, format!("{k} on {b}"), ratio(c(k), c(b)), 4.5);
        }
        for k in ["--g", "--a", "--r"] {
            check(k, format!("{k} on {b}"), ratio(c(k), c(b)), 3.0);
        }
    }
    check("--l", "--bg on --l".to_owned(), ratio(bg, c("--l")), 4.5);
    for k in ["--fg", "--mute", "--l"] {
        let what = format!("{k} on {}", tints[0].0);
        check(k, what, ratio(c(k), tints[0].1), 4.5);
    }
    for (k, (on, tint)) in [("--r", tints[0]), ("--r", tints[1]), ("--a", tints[2])] {
        check(k, format!("{k} on {on}"), ratio(c(k), tint), 3.0);
    }
    let mut pairs = vec![("--g", "--a"), ("--a", "--r"), ("--g", "--r")];
    for x in ["--l", "--link"] {
        pairs.extend(["--g", "--a", "--r"].map(|y| (x, y)));
    }
    for (x, y) in pairs {
        check(y, format!("ΔE2000 {x} ↔ {y}"), de(c(x), c(y)), 20.0);
    }
    Ok(out)
}

/// herdsman's scheme gate: every [`checks`] at or over its minimum.
///
/// # Errors
///
/// The checks that fail, each with its value and minimum, or the tokens missing or not `#rrggbb`.
pub fn gate(s: &Scheme) -> Result<(), String> {
    let fails: Vec<String> = checks(s)?
        .into_iter()
        .filter(|c| c.value < c.min)
        .map(|c| format!("{} {:.2} (min {})", c.what, c.value, c.min))
        .collect();
    if fails.is_empty() {
        Ok(())
    } else {
        Err(format!("fails the contrast gate: {}", fails.join("; ")))
    }
}

/// The checks that name `token`: the ones a change to it can break.
#[must_use]
pub fn about<'a>(all: &'a [Check], token: &str) -> Vec<&'a Check> {
    all.iter()
        .filter(|c| c.what.split_whitespace().any(|w| w == token))
        .collect()
}

/// The nearest colour to `token`'s, darker or lighter, with which every check naming it passes; none when no shade
/// does.
#[must_use]
pub fn fix(s: &Scheme, token: &str) -> Option<String> {
    let n = u32::from_str_radix(s.tokens.get(token)?.strip_prefix('#')?, 16).ok()?;
    let [red, green, blue] = [n >> 16, (n >> 8) & 255, n & 255];
    (1..=64u32)
        .flat_map(|i| [0, 255].map(|t| [red, green, blue].map(|v| (v * (64 - i) + t * i) / 64)))
        .map(|[r, g, b]| format!("#{r:02x}{g:02x}{b:02x}"))
        .find(|candidate| {
            let mut t = s.clone();
            t.tokens.insert(token.to_owned(), candidate.clone());
            checks(&t).is_ok_and(|all| about(&all, token).iter().all(|c| c.value >= c.min))
        })
}
