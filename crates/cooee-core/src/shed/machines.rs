use std::{collections::BTreeMap, path::PathBuf, time::UNIX_EPOCH};

use serde_json::json;
use shed_proto::{Link, Method, RunParams, WaitOutcome};

use super::{Shed, ShedError};

impl Shed {
    /// Machine `machine`'s held link (`machine.links`); `None` for a machine shed doesn't have.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn link(&self, machine: &str) -> Result<Option<Link>, ShedError> {
        Ok(self.links()?.remove(machine))
    }

    /// Every machine's held link, by name (`machine.links`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn links(&self) -> Result<BTreeMap<String, Link>, ShedError> {
        self.call(Method::MachineLinks, &json!({}))
    }

    /// Runs `cmd` to its end in a plain terminal on `machine` (#150), then removes it: its exit code, `None` when
    /// it ran past `timeout` seconds or ended without one.
    ///
    /// # Errors
    /// [`ShedError`] when it can't start or be waited for.
    pub fn run_on(
        &self,
        machine: &str,
        cmd: Vec<String>,
        timeout: f64,
    ) -> Result<Option<i32>, ShedError> {
        let at = crate::clock::now().duration_since(UNIX_EPOCH);
        let run = RunParams {
            name: Some(format!("cooee-check-{:x}", at.map_or(0, |d| d.as_nanos()))),
            labels: BTreeMap::from([("cooee-check".to_owned(), "1".to_owned())]),
            cwd: PathBuf::from("/"),
            keep: false,
            idle_after: None,
            cmd,
            acp: None,
            policy: None,
            resume: None,
            mcp_servers: Vec::new(),
            meta: None,
            no_hibernate: false,
            machine: Some(machine.to_owned()),
            parent: None,
            rows: None,
            cols: None,
        };
        let name = self.run(&run)?.name;
        let waited = self.exited(&name, timeout);
        self.close(&name)?;
        let waited = waited?;
        Ok(waited
            .terminal
            .exit
            .filter(|_| waited.outcome != WaitOutcome::Timeout))
    }
}
