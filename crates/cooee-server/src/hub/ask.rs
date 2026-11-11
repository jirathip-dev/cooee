use std::{sync::mpsc, time::Duration};

use cooee_core::personas;

use super::{Hub, HubError, lock, now};

const DEPTH: usize = 2;

fn no<T>(code: &str, why: &str) -> Result<T, HubError> {
    Err(HubError::Refused(format!("{code}: {why}")))
}

impl Hub {
    /// Thread `id`'s persona asks persona `target` `message` and waits up to `wait` for its reply, in the exchange
    /// thread linked to `id` (started on the first ask, reused after).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token or a closed thread, and with a code: `NOT_ALLOWED` when the persona's
    /// `can_message` doesn't name `target`, `DEPTH` past G4's depth, `OFFLINE` when `target`'s session can't take
    /// it, `TIMEOUT` when no reply came in time.
    pub fn ask_persona(
        &self,
        (id, token): (&str, &str),
        target: &str,
        message: &str,
        wait: Duration,
    ) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let asker = personas::find(self.config_dir(), &self.data, &meta.profile).ok();
        let Some(asker) = asker.filter(|p| p.can_message.iter().any(|t| t == target)) else {
            return no("NOT_ALLOWED", &format!("can_message doesn't name {target}"));
        };
        let (mut depth, mut up) = (0, meta.asked_by);
        while !up.is_empty() {
            depth += 1;
            up = self.thread(&up).map(|t| t.0.asked_by).unwrap_or_default();
        }
        if depth >= DEPTH {
            return no("DEPTH", "an asked persona may ask one more, no further");
        }
        let key = format!("{id}>{target}");
        let (tx, rx) = mpsc::channel();
        lock(&self.asks).insert(key.clone(), tx);
        let open = self.all().into_iter().find_map(|l| {
            let m = &lock(&l).meta;
            (m.asked_by == id && m.profile == target && !m.closed).then(|| m.id.clone())
        });
        let said = (
            &*format!("[{}]: {message}", asker.name),
            &*format!("ask-{key}-{}", now()),
        );
        let sent = match open {
            Some(x) => self.send(&x, said.0, said.1, &[]).map(drop),
            None => self
                .seat(target, said, |r| {
                    id.clone_into(&mut r.asked_by);
                })
                .and_then(|x| {
                    self.link(id, &x, &format!("asked {target}"), true)
                        .map(drop)
                }),
        };
        let got = sent.map(|()| rx.recv_timeout(wait));
        lock(&self.asks).remove(&key);
        match got {
            Err(e) => no("OFFLINE", &format!("{target} can't take it: {e}")),
            Ok(Err(_)) => no("TIMEOUT", &format!("no reply from {target} in time")),
            Ok(Ok(reply)) => Ok(reply),
        }
    }
}
