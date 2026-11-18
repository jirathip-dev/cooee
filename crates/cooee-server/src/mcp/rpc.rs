use std::{
    io::{self, Write},
    sync::{Mutex, PoisonError},
};

use serde_json::{Value, json};

pub(super) fn reply(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

pub(super) fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

pub(super) fn put(out: &Mutex<io::Stdout>, msg: &Value) {
    let mut out = out.lock().unwrap_or_else(PoisonError::into_inner);
    writeln!(out, "{msg}").and_then(|()| out.flush()).ok();
}
