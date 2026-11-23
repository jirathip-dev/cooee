use super::{Setup, output, step};

impl Setup {
    pub(super) fn tailnet(&self, port: u16) -> Result<(), String> {
        let served = output(&["tailscale", "serve", "status", "--json"])
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .is_some_and(|s| {
                s.get("TCP")
                    .and_then(|t| t.get(port.to_string()))
                    .is_some_and(|p| !p.is_null())
            });
        if !step(&format!("tailscale serve {port}"), served) {
            let (https, to) = (
                format!("--https={port}"),
                format!("http://127.0.0.1:{port}"),
            );
            self.exec(&["tailscale", "serve", "--bg", &https, &to])?;
        }
        Ok(())
    }
}
