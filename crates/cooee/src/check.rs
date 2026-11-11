//! `cooee plugin check <answer.json>...` (#138, #125 Q4): validates a plugin's answers against
//! docs/plugins/screens.schema.json, naming each field that breaks it. It reads the schema keywords that file uses.

use std::fs;

use serde_json::Value;

const SCHEMA: &str = include_str!("../../../docs/plugins/screens.schema.json");

/// Checks each file, printing `ok` or `FAIL` and its `$.path: why` lines.
pub fn run(paths: &[&str]) -> Result<(), String> {
    let schema: Value = serde_json::from_str(SCHEMA).map_err(|e| format!("the schema: {e}"))?;
    let mut failed = 0;
    for p in paths {
        let read = fs::read_to_string(p).map_err(|e| e.to_string());
        let answer =
            read.and_then(|t| serde_json::from_str(&t).map_err(|e| format!("not JSON: {e}")));
        let fails = answer.map_or_else(|e| vec![e], |a| errors(&schema, &schema, &a, "$"));
        if fails.is_empty() {
            println!("ok   {p}");
        } else {
            failed += 1;
            println!("FAIL {p}\n     {}", fails.join("\n     "));
        }
    }
    match (paths.len(), failed) {
        (0, _) => Err("usage: cooee plugin check <answer.json>...".to_owned()),
        (_, 0) => Ok(()),
        _ => Err(format!(
            "{failed} answer(s) break docs/plugins/screens.schema.json"
        )),
    }
}

fn errors(root: &Value, s: &Value, v: &Value, at: &str) -> Vec<String> {
    let mut out = Vec::new();
    check(root, s, v, at, &mut out);
    out
}

fn is(v: &Value, t: &str) -> bool {
    match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        _ => v.is_null(),
    }
}

fn short(v: &Value) -> String {
    match v {
        Value::Object(_) => "an object".to_owned(),
        Value::Array(_) => "an array".to_owned(),
        _ => v.to_string(),
    }
}

fn list<'a>(s: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    s.get(key).and_then(Value::as_array).into_iter().flatten()
}

fn check(root: &Value, s: &Value, v: &Value, at: &str, out: &mut Vec<String>) {
    if let Some(r) = s.get("$ref").and_then(Value::as_str) {
        let target = r
            .strip_prefix("#/$defs/")
            .map_or(root, |d| &root["$defs"][d]);
        check(root, target, v, at, out);
    }
    if let Some(t) = s.get("type") {
        let types: Vec<&str> = t.as_str().map_or_else(
            || list(s, "type").filter_map(Value::as_str).collect(),
            |t| vec![t],
        );
        if !types.iter().any(|t| is(v, t)) {
            return out.push(format!("{at}: {} is not {}", short(v), types.join(" or ")));
        }
    }
    if s.get("enum")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.contains(v))
    {
        let names: Vec<String> = list(s, "enum").map(Value::to_string).collect();
        out.push(format!("{at}: {v} is not one of {}", names.join(", ")));
    }
    if s.get("const").is_some_and(|c| c != v) {
        out.push(format!("{at}: {v} is not {}", s["const"]));
    }
    fields(root, s, v, at, out);
    for sub in list(s, "allOf") {
        check(root, sub, v, at, out);
    }
    let tries: Vec<Vec<String>> = list(s, "anyOf").map(|b| errors(root, b, v, at)).collect();
    if !tries.is_empty() && tries.iter().all(|e| !e.is_empty()) {
        let each: Vec<String> = tries.iter().map(|e| e.join(", ")).collect();
        out.push(format!(
            "{at}: matches none of its forms ({})",
            each.join(" | ")
        ));
    }
    if s.get("not")
        .is_some_and(|n| errors(root, n, v, at).is_empty())
    {
        out.extend(
            list(&s["not"], "required")
                .map(|k| format!("{at}.{}: must not be sent", k.as_str().unwrap_or(""))),
        );
    }
    if let (Some(cond), Some(then)) = (s.get("if"), s.get("then"))
        && errors(root, cond, v, at).is_empty()
    {
        check(root, then, v, at, out);
    }
}

fn fields(root: &Value, s: &Value, v: &Value, at: &str, out: &mut Vec<String>) {
    if let Some(o) = v.as_object() {
        let missing = list(s, "required")
            .filter_map(Value::as_str)
            .filter(|k| !o.contains_key(*k));
        out.extend(missing.map(|k| format!("{at}.{k}: is required")));
        for (k, p) in s
            .get("properties")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            if let Some(x) = o.get(k) {
                check(root, p, x, &format!("{at}.{k}"), out);
            }
        }
    }
    if let (Some(a), Some(items)) = (v.as_array(), s.get("items")) {
        for (i, x) in a.iter().enumerate() {
            check(root, items, x, &format!("{at}[{i}]"), out);
        }
    }
    let n = v.as_f64();
    let past = |key: &str, over: fn(f64, f64) -> bool| {
        s.get(key)
            .and_then(Value::as_f64)
            .filter(|m| n.is_some_and(|n| over(n, *m)))
    };
    if let Some(m) = past("minimum", |n, m| n < m) {
        out.push(format!("{at}: {v} is under {m}"));
    }
    if let Some(m) = past("maximum", |n, m| n > m) {
        out.push(format!("{at}: {v} is over {m}"));
    }
    let len = v.as_array().map_or(0, Vec::len);
    if s.get("maxItems")
        .and_then(Value::as_u64)
        .is_some_and(|m| len as u64 > m)
    {
        out.push(format!("{at}: has {len} items, over {}", s["maxItems"]));
    }
}
