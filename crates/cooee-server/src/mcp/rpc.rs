use std::{
    io::{self, Write},
    sync::{Mutex, PoisonError},
};

use serde_json::{Value, json};

pub(super) fn reply(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// A JSON-RPC error answer to request `id`.
#[must_use]
pub fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// `msg`, a `tools/list` answer, with only the `tools` allowed (none: all).
#[must_use]
pub fn only(mut msg: Value, tools: Option<&[String]>) -> Value {
    if let (Some(allow), Some(list)) = (
        tools,
        msg.pointer_mut("/result/tools")
            .and_then(Value::as_array_mut),
    ) {
        list.retain(|t| {
            t["name"]
                .as_str()
                .is_some_and(|n| allow.iter().any(|a| a == n))
        });
    }
    msg
}

pub(super) fn put(out: &Mutex<io::Stdout>, msg: &Value) {
    let mut out = out.lock().unwrap_or_else(PoisonError::into_inner);
    writeln!(out, "{msg}").and_then(|()| out.flush()).ok();
}
