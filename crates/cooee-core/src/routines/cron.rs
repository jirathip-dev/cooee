//! A routine's schedule (#75): five-field cron in the Mac's local time, with no time crate.

use serde::{Deserialize, Serialize};

const DAY: i64 = 1440;
const NAMES: [&str; 5] = ["minute", "hour", "day-of-month", "month", "day-of-week"];
const RANGES: [(u8, u8); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
const DAYS: [&str; 7] = [
    "Sundays",
    "Mondays",
    "Tuesdays",
    "Wednesdays",
    "Thursdays",
    "Fridays",
    "Saturdays",
];

/// Five-field cron in the Mac's local time: minute hour day-of-month month day-of-week; `*`, `n`, `a-b`, `*/s`,
/// `a-b/s` and comma lists; day-of-week 0–7 (0 and 7 Sunday); day-of-month and day-of-week both set: either matches.
/// No names, no seconds. Refused when two times of a day are under 15 minutes apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Schedule {
    text: String,
    fields: [u64; 5],
    either_day: bool,
}

impl Schedule {
    /// The schedule `text` says.
    ///
    /// # Errors
    /// `BAD_SCHEDULE: …` naming the field that is wrong, or the 15-minute floor.
    pub fn parse(text: &str) -> Result<Self, String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let bad = |why: String| format!("BAD_SCHEDULE: {text:?} {why}");
        if words.len() != 5 {
            return Err(bad(
                "isn't 5 fields: minute hour day-of-month month day-of-week".to_owned(),
            ));
        }
        let mut fields = [0; 5];
        for (i, w) in words.iter().enumerate() {
            fields[i] = field(w, RANGES[i]).ok_or_else(|| bad(format!("has a bad {}", NAMES[i])))?;
        }
        if (fields[4] >> 7) & 1 == 1 {
            fields[4] = (fields[4] | 1) & !(1 << 7);
        }
        let s = Self {
            text: words.join(" "),
            fields,
            either_day: !words[2].starts_with('*') && !words[4].starts_with('*'),
        };
        let times = s.times();
        let wrap = times.first().map_or(DAY, |f| f + DAY) - times.last().copied().unwrap_or(0);
        let gap = times.windows(2).map(|w| w[1] - w[0]).fold(wrap, i64::min);
        if gap < 15 {
            return Err(bad("runs more often than every 15 minutes".to_owned()));
        }
        Ok(s)
    }

    /// The newest due time at or before `now` (ms), in UTC offset `offset_min`; None in the year before it.
    #[must_use]
    pub fn latest(&self, now: u64, offset_min: i32) -> Option<u64> {
        let offset = i64::from(offset_min);
        let local = i64::try_from(now / 60_000).ok()? + offset;
        let (today, minute) = (local.div_euclid(DAY), local.rem_euclid(DAY));
        let times = self.times();
        (0..=366).find_map(|back| {
            let day = today - back;
            let until = if back == 0 { minute } else { DAY - 1 };
            let t = times.iter().rev().find(|t| **t <= until).filter(|_| self.on(day))?;
            u64::try_from((day * DAY + t - offset) * 60_000).ok()
        })
    }

    /// In words: "Mondays 09:00", "every day 18:30", else the cron text.
    #[must_use]
    pub fn describe(&self) -> String {
        let w: Vec<&str> = self.text.split(' ').collect();
        let (Ok(m), Ok(h)) = (w[0].parse::<u8>(), w[1].parse::<u8>()) else {
            return self.text.clone();
        };
        let day = w[4].parse::<usize>().ok().and_then(|d| DAYS.get(d % 7));
        match (w[2], w[3], w[4], day) {
            ("*", "*", "*", _) => format!("every day {h:02}:{m:02}"),
            ("*", "*", _, Some(d)) => format!("{d} {h:02}:{m:02}"),
            _ => self.text.clone(),
        }
    }

    fn has(&self, f: usize, n: i64) -> bool {
        (self.fields[f] >> n) & 1 == 1
    }

    /// The minutes of a day it runs at, in order.
    fn times(&self) -> Vec<i64> {
        let hours = (0..24).filter(|h| self.has(1, *h));
        hours
            .flat_map(|h| (0..60).filter(|m| self.has(0, *m)).map(move |m| h * 60 + m))
            .collect()
    }

    /// Whether it runs on day `day` (days since 1970-01-01, a Thursday).
    fn on(&self, day: i64) -> bool {
        let (_, month, date) = civil(day);
        let (d, w) = (self.has(2, date), self.has(4, (day + 4).rem_euclid(7)));
        self.has(3, month) && if self.either_day { d || w } else { d && w }
    }
}

impl TryFrom<String> for Schedule {
    type Error = String;
    fn try_from(text: String) -> Result<Self, String> {
        Self::parse(&text)
    }
}

impl From<Schedule> for String {
    fn from(s: Schedule) -> Self {
        s.text
    }
}

/// One field's values as bits: a comma list of `*`, `n`, `a-b`, each optionally `/step`.
fn field(word: &str, (lo, hi): (u8, u8)) -> Option<u64> {
    let mut bits = 0;
    for item in word.split(',') {
        let (range, step) = match item.split_once('/') {
            Some((r, s)) => (r, s.parse::<u8>().ok().filter(|s| *s > 0)?),
            None => (item, 1),
        };
        let (a, b) = match range.split_once('-') {
            _ if range == "*" => (lo, hi),
            Some((a, b)) => (a.parse().ok()?, b.parse().ok()?),
            None => range.parse().ok().map(|n| (n, if step > 1 { hi } else { n }))?,
        };
        if a < lo || b > hi || a > b {
            return None;
        }
        bits |= (a..=b).step_by(step.into()).fold(0, |m, n| m | (1 << n));
    }
    Some(bits)
}

/// Year, month and day of day `z` since 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        yoe + era * 400 + i64::from(month <= 2),
        month,
        doy - (153 * mp + 2) / 5 + 1,
    )
}

/// Time `at` (ms) in UTC offset `offset_min` as `YYYY-MM-DD HH:MM`.
#[must_use]
pub fn local(at: u64, offset_min: i32) -> String {
    let m = i64::try_from(at / 60_000).unwrap_or(0) + i64::from(offset_min);
    let ((y, mo, d), t) = (civil(m.div_euclid(DAY)), m.rem_euclid(DAY));
    format!("{y:04}-{mo:02}-{d:02} {:02}:{:02}", t / 60, t % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60_000;
    /// 2024-11-11 00:00 UTC, a Monday.
    const MON: u64 = 1_731_283_200_000;

    #[test]
    fn latest_is_the_newest_due_time_in_local_time() {
        let s = Schedule::parse("0 9 * * 1").unwrap();
        assert_eq!(s.describe(), "Mondays 09:00");
        // Bangkok (+07:00): Monday 09:00 local is 02:00 UTC.
        let nine = MON + 2 * 60 * MIN;
        assert_eq!(s.latest(nine, 420), Some(nine));
        assert_eq!(s.latest(nine + 3 * MIN, 420), Some(nine));
        assert_eq!(s.latest(nine - MIN, 420), Some(nine - 7 * 1440 * MIN));
        assert_eq!(local(nine, 420), "2024-11-11 09:00");
        assert_eq!(local(nine, 0), "2024-11-11 02:00");
        let daily = Schedule::parse("30 18 * * *").unwrap();
        assert_eq!(daily.describe(), "every day 18:30");
        assert_eq!(daily.latest(MON, 0), Some(MON - 330 * MIN));
    }

    #[test]
    fn day_of_month_and_day_of_week_both_set_match_either() {
        // The 1st, or a Friday: 2024-11-08 is a Friday, 2024-11-01 is one too; Monday the 11th is neither.
        let s = Schedule::parse("0 0 1 * 5").unwrap();
        assert_eq!(s.latest(MON, 0), Some(MON - 3 * 1440 * MIN));
        let first = Schedule::parse("0 0 1 * *").unwrap();
        assert_eq!(first.latest(MON, 0), Some(MON - 10 * 1440 * MIN));
        let sunday = Schedule::parse("0 0 * * 7").unwrap();
        assert_eq!(sunday.latest(MON, 0), Some(MON - 1440 * MIN));
    }

    #[test]
    fn steps_ranges_and_the_fifteen_minute_floor() {
        assert!(Schedule::parse("*/15 9-17 * * 1-5").is_ok());
        assert!(Schedule::parse("0,30 * * * *").is_ok());
        for bad in [
            "*/5 * * * *",
            "0,10 9 * * *",
            "5,55 0,23 * * *",
            "* 9 * * *",
            "0 9 * *",
            "60 9 * * *",
            "0 9 * * mon",
            "0 9 0 * *",
            "0 9 * * */0",
        ] {
            let e = Schedule::parse(bad).unwrap_err();
            assert!(e.starts_with("BAD_SCHEDULE: "), "{bad}: {e}");
        }
        assert!(
            Schedule::parse("*/5 * * * *")
                .unwrap_err()
                .ends_with("runs more often than every 15 minutes")
        );
        let s: Schedule = serde_json::from_str("\"0  9 * * 1\"").unwrap();
        assert_eq!(serde_json::to_string(&s).unwrap(), "\"0 9 * * 1\"");
    }
}
