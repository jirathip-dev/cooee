use std::{panic::AssertUnwindSafe, sync::Arc, thread};

use cooee_core::{Provider, Sheet, Status};

use super::{Logins, logins::Login};
use crate::hub::{lock, now};

const EVERY: u64 = 10 * 60_000;
const SLOWEST: u64 = 80 * 60_000;
const GAP: u64 = 30_000;

#[derive(Default)]
pub(super) struct Check {
    pub(super) status: Option<Status>,
    at: u64,
    every: u64,
    pub(super) next: u64,
    busy: bool,
}

impl Check {
    fn start(&mut self, at: u64) -> bool {
        let due = !self.busy && at >= self.next;
        self.busy |= due;
        due
    }

    pub(super) fn settle(&mut self, s: Status, at: u64) {
        self.every = if s.unknown {
            (self.every.max(EVERY) * 2).min(SLOWEST)
        } else {
            EVERY
        };
        (self.at, self.next) = (at, at + self.every);
        self.status = Some(match self.status.take() {
            Some(old) if s.unknown => Status {
                signed_in: old.signed_in,
                who: old.who,
                usage: old.usage,
                ..s
            },
            _ => s,
        });
    }

    fn ask(&mut self, at: u64) {
        if self.every <= EVERY && at >= self.at + GAP {
            self.next = self.next.min(at);
        }
    }

    pub(super) fn known(&self, at: u64) -> (Status, Option<u64>) {
        match &self.status {
            Some(s) => (s.clone(), Some(at.saturating_sub(self.at) / 1000)),
            None => (Status::unknown("not checked yet"), None),
        }
    }
}

impl Logins {
    /// Every provider with its accounts' status, as last checked, and how its last login ended.
    #[must_use]
    pub fn statuses(&self) -> Vec<Sheet> {
        let at = now();
        let checks = lock(&self.checks);
        let live = lock(&self.live);
        let sheet = |p: &Arc<dyn Provider>| {
            let name = &p.spec().name;
            let mut accounts: Vec<_> = (self.accounts.list(name).into_iter())
                .map(|a| {
                    let check = checks.get(&(name.clone(), a.name.clone()));
                    let status = check.map_or_else(|| Check::default().known(at), |c| c.known(at));
                    (a, status.0)
                })
                .collect();
            if p.missing() {
                accounts.truncate(1);
            }
            let why = match live.get(name) {
                Some(Login::Ended(why, _)) => Some(why.clone()),
                _ => None,
            };
            Sheet {
                provider: p.spec().clone(),
                accounts,
                why,
            }
        };
        self.providers.iter().map(sheet).collect()
    }

    pub(super) fn tick(self: &Arc<Self>) {
        for p in self.providers.iter() {
            for a in self.accounts.list(&p.spec().name) {
                let key = (p.spec().name.clone(), a.name.clone());
                if !lock(&self.checks)
                    .entry(key.clone())
                    .or_default()
                    .start(now())
                {
                    continue;
                }
                let (logins, p) = (Arc::clone(self), Arc::clone(p));
                thread::spawn(move || {
                    let check = || {
                        let mut s = p.check(a.dir.as_deref());
                        if s.signed_in {
                            let at = now() / 1000;
                            s.usage =
                                logins
                                    .accounts
                                    .usage(p.as_ref(), &a.name, a.dir.as_deref(), at);
                        }
                        s
                    };
                    let status = std::panic::catch_unwind(AssertUnwindSafe(check))
                        .unwrap_or_else(|_| Status::unknown("can't tell: the status check failed"));
                    let mut checks = lock(&logins.checks);
                    if let Some(c) = checks.get_mut(&key) {
                        c.busy = false;
                        c.settle(status, now());
                    }
                });
            }
        }
    }

    /// Asks for every account's status check again (the sign-ins card opened): those not checked in the last 30 s
    /// nor backing off after a timeout run within a second.
    pub fn ask(&self) {
        let at = now();
        for c in lock(&self.checks).values_mut() {
            c.ask(at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(signed_in: bool, line: &str) -> Status {
        Status {
            signed_in,
            unknown: false,
            line: line.to_owned(),
            install: None,
            who: Some("o@example.com".to_owned()),
            usage: None,
        }
    }

    #[test]
    fn a_hanging_check_starts_once_per_growing_interval() {
        let min = 60_000;
        let timeout = Status::unknown("can't tell: the status command timed out");
        let mut c = Check::default();
        let mut t = 0;
        let mut started = Vec::new();
        for _ in 0..6 {
            while !c.start(t) {
                c.ask(t);
                t += 1000;
            }
            started.push(t / min);
            for _ in 0..10 {
                c.ask(t);
                t += 1000;
                assert!(!c.start(t));
            }
            c.busy = false;
            c.settle(timeout.clone(), t);
        }
        assert_eq!(started, [0, 20, 60, 140, 220, 300]);
        assert_eq!(c.every, 80 * min);

        assert!(c.start(t + 80 * min));
        c.busy = false;
        c.settle(status(true, "signed in"), t + 80 * min);
        assert_eq!(c.every, 10 * min);
        c.ask(t + 80 * min + 29_000);
        assert!(!c.start(t + 80 * min + 29_000));
        c.ask(t + 80 * min + 30_000);
        assert!(c.start(t + 80 * min + 30_000));
    }

    #[test]
    fn a_timeout_keeps_a_signed_in_account_signed_in_as_unknown() {
        let mut c = Check::default();
        assert_eq!(c.known(0), (Status::unknown("not checked yet"), None));
        c.settle(status(true, "signed in"), 1000);
        let timeout = Status::unknown("can't tell: the status command timed out");
        c.settle(timeout.clone(), 5000);
        c.settle(timeout, 9000);
        let (s, age) = c.known(12_000);
        assert!(s.signed_in && s.unknown, "{s:?}");
        assert_eq!(s.line, "can't tell: the status command timed out");
        assert_eq!(s.who.as_deref(), Some("o@example.com"));
        assert_eq!(age, Some(3));
        c.settle(status(false, "Not signed in"), 20_000);
        assert!(!c.known(20_000).0.signed_in && !c.known(20_000).0.unknown);
    }
}
