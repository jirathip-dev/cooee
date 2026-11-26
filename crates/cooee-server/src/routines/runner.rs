use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use cooee_core::routines::{Hooks, Routines, Who};
use topcoat::Result;

use super::hooks;
use crate::hub::{Hub, HubError, Recipe, now};

/// How a `new_thread` run starts as `who`: its agent, the agent's name, its folder and its recipe.
pub type Start =
    Box<dyn Fn(&Hub, &Who) -> Result<(String, String, PathBuf, Recipe), HubError> + Send + Sync>;

/// The model and effort a background run of an agent picks when it names none (#219): its provider's cheapest.
pub type Cheapest = Box<dyn Fn(&str) -> (String, String) + Send + Sync>;

/// The routines and what runs them.
pub struct Runner {
    pub(super) hub: Arc<Hub>,
    pub(super) data: PathBuf,
    pub(super) config: Option<PathBuf>,
    pub(super) cheapest: Cheapest,
    pub(super) hooks: Option<Hooks>,
    pub(super) file: Routines,
    pub(super) lock: Mutex<()>,
    pub(super) start: Start,
    pub(super) offset: Mutex<(i32, u64)>,
}

impl Runner {
    /// The routines of config folder `config` (none: the data folder), their run state in `data`, run on `hub`;
    /// `new_thread` runs start with `start`, a persona's consolidation runs (#219) on `cheapest` unless it names a
    /// model; GitHub events come on `hooks`.
    #[must_use]
    pub fn new(
        hub: Arc<Hub>,
        (config, data): (Option<&std::path::Path>, &std::path::Path),
        (start, cheapest): (Start, Cheapest),
        hooks: Option<Hooks>,
    ) -> Self {
        Self {
            hub,
            data: data.to_owned(),
            config: config.map(ToOwned::to_owned),
            cheapest,
            hooks,
            file: Routines::new(config.unwrap_or(data), data),
            lock: Mutex::new(()),
            start,
            offset: Mutex::new((0, 0)),
        }
    }

    /// Runs [`Runner::tick`] every `beat`, from now on; with `[hooks]`, [`hooks`] on its `bind` and
    /// [`Runner::catch_up`] every 15 minutes. Call it inside the server's runtime.
    ///
    /// # Errors
    /// When `[hooks] bind` can't be listened on.
    pub fn every(runner: Arc<Self>, beat: Duration) -> std::io::Result<()> {
        if let Some(h) = &runner.hooks {
            let listener = std::net::TcpListener::bind(h.bind)?;
            listener.set_nonblocking(true)?;
            let listener = tokio::net::TcpListener::from_std(listener)?;
            tokio::spawn(topcoat::serve(listener, hooks(Arc::clone(&runner))));
            let poll = Arc::clone(&runner);
            std::thread::spawn(move || {
                loop {
                    poll.catch_up(now());
                    cooee_core::clock::sleep(Duration::from_mins(15));
                }
            });
        }
        std::thread::spawn(move || {
            loop {
                runner.tick(now());
                cooee_core::clock::sleep(beat);
            }
        });
        Ok(())
    }
}
