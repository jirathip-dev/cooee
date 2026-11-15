//! #336: a group signal reaches only a group the child leads, while it is not reaped: never the caller's own group
//! (a child spawned without `process_group(0)` shares it, the CI runner included) nor a stale pid.
#![cfg(unix)]

use std::{
    io::{BufRead, BufReader},
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use cooee_core::group;

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .unwrap()
        .success()
}

fn gone(pid: &str) -> bool {
    let end = Instant::now() + Duration::from_secs(2);
    while alive(pid) && Instant::now() < end {
        thread::sleep(Duration::from_millis(20));
    }
    !alive(pid)
}

/// `sh -c script` leading its own group; the first line it prints.
fn own_group(script: &str) -> (Child, String) {
    let mut c = Command::new("sh")
        .args(["-c", script])
        .stdout(Stdio::piped())
        .process_group(0)
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(c.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    (c, line.trim().to_owned())
}

#[test]
fn a_child_in_the_callers_group_is_refused() {
    let mut c = Command::new("sleep").arg("30").spawn().unwrap();
    assert!(!group::signal(&mut c, "-TERM"), "signalled our own group");
    assert!(alive(&c.id().to_string()));
    c.kill().unwrap();
    c.wait().unwrap();
}

#[test]
fn a_reaped_child_is_refused() {
    let (mut c, sleeper) = own_group("sleep 30 & echo $!; exit 0");
    c.wait().unwrap();
    assert!(!group::signal(&mut c, "-TERM"), "signalled a reaped pid");
    assert!(alive(&sleeper), "the stale group was signalled");
    Command::new("kill").arg(&sleeper).status().unwrap();
}

#[test]
fn a_group_the_child_leads_is_signalled_whole() {
    let (mut c, sleeper) = own_group("sleep 30 & echo $!; wait");
    assert!(group::signal(&mut c, "-TERM"));
    c.wait().unwrap();
    assert!(gone(&sleeper), "pid {sleeper} outlived its group's signal");
}
