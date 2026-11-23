use std::{
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::Path,
    time::Duration,
};

use cooee_core::Config;
use farrier_service::Files;

use super::{Setup, step, wait};
use crate::service::service;

impl Setup {
    pub(super) fn service(
        &self,
        config: &Config,
        path: &Path,
        changed: bool,
        verb: &str,
    ) -> Result<(), String> {
        let service = service(config, self.explicit.then_some(path), verb)?;
        let label = service.label().to_string();
        let status = service.status().map_err(|e| e.to_string())?;
        let same = status.files == Files::Same;
        let up = status.loaded && status.refused.is_none();
        if step(&format!("cooee service {label}"), same && up && !changed) {
            return Ok(());
        }
        let restart = same && up;
        if self.dry_run {
            if !same {
                for (file, text) in service.files() {
                    cooee_core::say!("  would write {}:\n{text}", file.display());
                }
            }
            let commands = service.commands();
            let line = if restart {
                commands.restart
            } else {
                commands.load
            };
            cooee_core::say!("  would run: {line}");
            return Ok(());
        }
        let done = if restart {
            service.restart().map(|()| "restarted".to_owned())
        } else {
            service.install().map(|i| i.to_string())
        };
        cooee_core::say!("  {label}: {}", done.map_err(|e| e.to_string())?);
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, config.port));
        wait(|| TcpStream::connect_timeout(&addr, Duration::from_secs(1)).is_ok());
        Ok(())
    }
}
