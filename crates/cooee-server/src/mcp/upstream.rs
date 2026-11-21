use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Stdio},
    sync::mpsc::Sender,
};

use serde_json::Value;

use super::VAR;

pub(super) struct Upstream(pub(super) Child, pub(super) ChildStdin);

pub(super) fn open(g: &super::Child, tx: Sender<Value>) -> Result<Upstream, String> {
    let mut child = cooee_core::runner::command(&g.command)
        .args(&g.args)
        .envs(g.env.iter().map(|(k, v)| (k, v)))
        .env_remove(VAR)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{}: {e}", g.command))?;
    let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
    let (Some(stdin), Some(stdout)) = (stdin, stdout) else {
        return Err("no pipes".to_owned());
    };
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Ok(v) = serde_json::from_str(&line)
                && tx.send(v).is_err()
            {
                return;
            }
        }
    });
    Ok(Upstream(child, stdin))
}

impl Upstream {
    pub(super) fn send(&mut self, msg: &Value) -> Result<(), String> {
        writeln!(self.1, "{msg}").map_err(|e| e.to_string())
    }
}
