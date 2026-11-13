//! Signals to the process group a child leads (#336).

use std::process::{Child, Command, Stdio};

/// Sends `sig` (`-TERM`, `-KILL`) to the process group `child` leads; whether it did. Refuses pid 1 or below, a
/// child already reaped (its pid may be reused) and one that leads no group of its own (it shares the caller's).
pub fn signal(child: &mut Child, sig: &str) -> bool {
    let pid = child.id().to_string();
    if child.id() <= 1 || !matches!(child.try_wait(), Ok(None)) || !leads_group(&pid) {
        return false;
    }
    Command::new("kill")
        .args([sig, "--", &format!("-{pid}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn leads_group(pid: &str) -> bool {
    Command::new("ps")
        .args(["-o", "pgid=", "-p", pid])
        .stderr(Stdio::null())
        .output()
        .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == pid)
}
