use serde::{Deserialize, Serialize};

use super::{Paused, Routine, local, parse_local};

/// When and how a routine ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ended {
    /// When, in Unix ms.
    pub at: u64,
    /// By what.
    pub by: End,
}

/// What ended a routine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum End {
    /// Its end time came.
    EndTime,
    /// Its maker's `stop_routine`.
    StopRoutine,
}

pub(super) fn ends_at(
    until: Option<&str>,
    (now, offset_min): (u64, i32),
) -> Result<Option<u64>, String> {
    until
        .map(|u| {
            parse_local(u.trim(), offset_min)
                .filter(|at| *at > now)
                .ok_or_else(|| format!("BAD_UNTIL: {u:?} is no local YYYY-MM-DD HH:MM after now"))
        })
        .transpose()
}

impl Routine {
    /// Ends it once its end time is past `now`: whether it ended now.
    pub fn end(&mut self, now: u64) -> bool {
        let Some(at) = self.ends_at.filter(|at| *at <= now) else {
            return false;
        };
        if matches!(self.paused, Some(Paused::Ended(_))) {
            return false;
        }
        self.paused = Some(Paused::Ended(Ended {
            at,
            by: End::EndTime,
        }));
        true
    }

    /// Its end in words, in UTC offset `offset_min`: `until …`, `ended … by …`, or empty.
    #[must_use]
    pub fn until(&self, offset_min: i32) -> String {
        match (&self.paused, self.ends_at) {
            (Some(Paused::Ended(e)), _) => {
                let by = match e.by {
                    End::EndTime => "its end time",
                    End::StopRoutine => "stop_routine",
                };
                format!("ended {} by {by}", local(e.at, offset_min))
            }
            (_, Some(at)) => format!("until {}", local(at, offset_min)),
            (_, None) => String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Ask, Edit, Skip};
    use super::*;

    const MIN: u64 = 60_000;
    const MON: u64 = 1_731_283_200_000;

    fn chat() -> crate::Thread {
        crate::Thread {
            id: "t-1".to_owned(),
            ..crate::Thread::default()
        }
    }

    fn ask(when: &str) -> Ask {
        Ask {
            name: "Watch".to_owned(),
            when: Some(when.to_owned()),
            github: None,
            webhook: None,
            mail: false,
            prompt: "watch".to_owned(),
            script: None,
            place: String::new(),
            until: None,
        }
    }

    #[test]
    fn an_end_time_runs_it_before_and_ends_it_at_it_and_resume_drops_a_past_one() {
        let nine = MON + 9 * 60 * MIN;
        let until = Ask {
            until: Some("2024-11-18 09:00".to_owned()),
            ..ask("0 9 * * *")
        };
        let past = Ask {
            until: Some("2024-11-11 08:00".to_owned()),
            ..until.clone()
        };
        let e = Routine::make(&[], past, &chat(), (nine, 0)).unwrap_err();
        assert!(e.starts_with("BAD_UNTIL"), "{e}");
        let mut r = Routine::make(&[], until, &chat(), (MON, 0)).unwrap();
        let end = nine + 7 * 1440 * MIN;
        assert_eq!(
            (r.ends_at, r.until(0)),
            (Some(end), "until 2024-11-18 09:00".to_owned())
        );
        assert!(!r.end(end - MIN));
        assert_eq!(
            (r.due(end - MIN, 0), &r.paused),
            (Some(end - 1440 * MIN), &None)
        );
        assert!(r.end(end) && !r.end(end + MIN));
        let ended = Paused::Ended(Ended {
            at: end,
            by: End::EndTime,
        });
        assert_eq!(r.paused, Some(ended));
        assert_eq!(r.until(0), "ended 2024-11-18 09:00 by its end time");
        assert_eq!(r.admit(end + MIN), Err(Skip::Paused));
        let resume = Edit {
            paused: Some(false),
            ..Edit::default()
        };
        r.edit(resume, end + MIN).unwrap();
        assert!(!r.end(end + 1440 * MIN));
        assert_eq!((&r.paused, r.ends_at), (&None, None));
    }
}
