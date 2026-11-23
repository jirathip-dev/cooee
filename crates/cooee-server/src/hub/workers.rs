use cooee_core::Calls as _;
use std::{
    sync::{Arc, mpsc::Receiver},
    thread,
};

use super::{Hub, SERVED_EVERY, TICK, now};

impl Hub {
    pub(super) fn start_workers(
        self: &Arc<Self>,
        next: Receiver<String>,
        unsent: Receiver<String>,
    ) {
        let worker = Arc::clone(self);
        thread::spawn(move || {
            for room in next {
                if let Err(e) = worker.take_turn(&room) {
                    cooee_core::note!("cooee: room {room}: {e}");
                }
            }
        });
        let flusher = Arc::clone(self);
        thread::spawn(move || {
            for id in unsent {
                flusher.flush_text(id);
            }
        });
        let follower = Arc::clone(self);
        thread::spawn(move || follower.follow());
        let asker = Arc::clone(self);
        thread::spawn(move || {
            loop {
                let began = cooee_core::clock::instant();
                asker.refresh_served();
                cooee_core::clock::sleep(SERVED_EVERY.saturating_sub(began.elapsed()));
            }
        });
        let ticker = Arc::clone(self);
        thread::spawn(move || {
            ticker.refresh(now(), false);
            loop {
                cooee_core::clock::sleep(TICK);
                ticker.refresh(now(), false);
                ticker.personas();
                ticker.watch(now());
                ticker.unload(now());
                let at = now();
                if let Ok(terminals) = ticker.shed.ls(&cooee_core::shed_proto::LsParams::default()) {
                    ticker.prune(&terminals, at);
                }
            }
        });
    }
}
