use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex, PoisonError},
    thread,
};

use super::{Note, Push};

const QUEUED: usize = 64;

type Waiting = VecDeque<(Option<String>, Note)>;

#[derive(Default)]
pub(super) struct Queue {
    notes: Mutex<(bool, Waiting)>,
    ready: Condvar,
}

impl Push {
    /// Queues `note` for every paired device's subscriptions and APNs tokens (only `to`'s, if set): one worker
    /// sends them in turn, and a newer note for the same thread replaces one still waiting.
    pub fn send_to(&self, to: Option<&str>, note: &Note) {
        let mut q = self
            .queue
            .notes
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let (started, notes) = &mut *q;
        let item = (to.map(str::to_owned), note.clone());
        if let Some(old) = notes
            .iter_mut()
            .find(|(t, n)| *t == item.0 && n.tag == note.tag)
        {
            *old = item;
        } else if notes.len() < QUEUED {
            notes.push_back(item);
        } else {
            return cooee_core::note!("cooee: push: {QUEUED} waiting, dropped {}", note.tag);
        }
        if !*started {
            *started = true;
            let push = self.clone();
            thread::spawn(move || push.work());
        }
        self.queue.ready.notify_one();
    }

    /// [`Push::send_to`] every device.
    pub fn send(&self, note: &Note) {
        self.send_to(None, note);
    }

    /// Sends `note` now, as [`Push::send_to`] does: a 404/410, APNs' `BadDeviceToken` or an unpaired device drops
    /// its subscription; other failures are logged.
    pub fn send_now(&self, to: Option<&str>, note: &Note) {
        match self.deliver(to, note) {
            Ok(log) => log.iter().for_each(|l| cooee_core::note!("cooee: push{l}")),
            Err(e) => cooee_core::note!("cooee: push: {e}"),
        }
    }

    fn work(&self) {
        loop {
            let q = self
                .queue
                .notes
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let mut q = (self.queue.ready.wait_while(q, |q| q.1.is_empty()))
                .unwrap_or_else(PoisonError::into_inner);
            let Some((to, note)) = q.1.pop_front() else {
                continue;
            };
            drop(q);
            self.send_now(to.as_deref(), &note);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
            mpsc,
        },
        time::Duration,
    };

    use std::os::unix::fs::PermissionsExt;

    use cooee_core::write_private;

    use super::*;
    use crate::push::{
        apns::tests::{apns, p8, paired},
        crypto::new_key,
        key::tests::{dir, mode},
        note::tests::test_note,
        send::tests::serve,
    };

    #[test]
    fn a_burst_of_notes_is_sent_one_curl_at_a_time_newest_per_thread_with_the_key_read_once() {
        let dir = dir("burst");
        let device = paired(&dir, 1).remove(0);
        let (busy, most) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
        let (release, first) = mpsc::channel::<()>();
        let first = Mutex::new(Some(first));
        let (b, m) = (Arc::clone(&busy), Arc::clone(&most));
        let (url, got) = serve(move || {
            m.fetch_max(b.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
            let held = first.lock().unwrap().take();
            held.map(|h| h.recv());
            b.fetch_sub(1, Ordering::SeqCst);
            200
        });
        let key = dir.join("apns.p8");
        write_private(&key, p8(&new_key().unwrap()).as_bytes()).unwrap();
        let push = Push::new(&dir).with_apns(Some(apns(&url)));
        push.register_apns(&device, &"a".repeat(64), None).unwrap();
        let note = |i: usize, tag: &str| Note {
            tag: tag.to_owned(),
            body: i.to_string(),
            ..test_note()
        };
        let body = |rx: &mpsc::Receiver<String>| {
            let b: serde_json::Value =
                serde_json::from_str(&rx.recv_timeout(Duration::from_secs(10)).unwrap()).unwrap();
            b["aps"]["alert"]["body"].as_str().unwrap().to_owned()
        };

        push.send(&note(0, "t0"));
        assert_eq!(body(&got), "0");
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap();
        for i in 1..100 {
            push.send(&note(i, &format!("t{}", i % 5)));
        }
        push.send(&note(100, "end"));
        release.send(()).unwrap();
        let sent: Vec<String> = (0..6).map(|_| body(&got)).collect();
        assert_eq!(sent, ["96", "97", "98", "99", "95", "100"]);
        assert_eq!(most.load(Ordering::SeqCst), 1);
        assert_eq!(mode(&key), 0o644, "the key was read again");

        std::fs::write(&key, "not a key").unwrap();
        push.send_now(None, &note(101, "t9"));
        assert_eq!(mode(&key), 0o600, "a changed key file is read again");
    }
}
