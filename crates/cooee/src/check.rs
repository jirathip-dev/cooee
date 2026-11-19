use std::fs;

use cooee_core::plugins::Manifest;
use serde_json::{Value, json};

fn schema() -> Value {
    let defs = blocks::codegen::schema().get("$defs").cloned();
    let places = json!({ "type": "array", "items": { "type": "object", "required": ["type"], "properties": {
        "type": { "enum": ["header-strip", "chat-cards", "menu-row"] }, "id": { "type": "string" },
        "persona": { "type": "string" }, "label": { "type": "string" }, "subtitle": { "type": "string" },
        "screen": { "type": "string" } } } });
    let strings = json!({ "type": "array", "items": { "type": "string" } });
    let permissions = json!({ "type": "object", "properties": { "places": strings, "push": { "type": "boolean" },
        "chat_cards": { "type": "boolean" }, "actions": strings, "destructive": strings, "network": strings } });
    let answer = json!({ "seq": { "type": "integer" }, "ops": { "type": "array", "items": { "$ref": "#/$defs/Op" } },
        "toast": { "type": "string" }, "close": { "type": "boolean" }, "open": { "type": "string" } });
    let when = |key: &str, then: Value| json!({ "if": { "type": "object", "required": [key] }, "then": then });
    json!({ "$defs": defs, "allOf": [
        { "if": { "type": "array" }, "then": { "items": { "$ref": "#" } } },
        when("type", json!({ "$ref": "#/$defs/Block" })),
        when("ops", json!({ "properties": answer })),
        when("places", json!({ "properties": { "places": places, "claims": strings } })),
        when("permissions", json!({ "properties": { "permissions": permissions } })),
        when("block", json!({ "properties": { "block": { "$ref": "#/$defs/Block" } } })),
        { "type": ["array", "object"] },
    ] })
}

/// Checks each file (`what`: `plugin` or `layout`), printing `ok` or `FAIL` and its `$.path: why` lines.
pub fn run(what: &str, paths: &[&str]) -> Result<(), String> {
    let schema = schema();
    let mut failed = 0;
    for p in paths {
        let read = fs::read_to_string(p).map_err(|e| e.to_string());
        let fails = if what == "layout" {
            let checked = read.and_then(|t| {
                cooee_server::layout_check(&t).map_err(|b| format!("line {}: {}", b.line, b.why))
            });
            checked.err().into_iter().collect()
        } else if std::path::Path::new(p)
            .extension()
            .is_some_and(|e| e == "toml")
        {
            let manifest = read.and_then(|t| Manifest::parse(&t).map_err(|e| e.to_string()));
            manifest.err().into_iter().collect()
        } else {
            let answer =
                read.and_then(|t| serde_json::from_str(&t).map_err(|e| format!("not JSON: {e}")));
            answer.map_or_else(|e| vec![e], |a| errors(&schema, &schema, &a, "$"))
        };
        if fails.is_empty() {
            cooee_core::say!("ok   {p}");
        } else {
            failed += 1;
            cooee_core::say!("FAIL {p}\n     {}", fails.join("\n     "));
        }
    }
    match (paths.len(), failed) {
        (0, _) => Err("usage: cooee plugin|layout check <file>...".to_owned()),
        (_, 0) => Ok(()),
        _ => Err(format!("{failed} file(s) failed the check")),
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
        let target = match r.strip_prefix("#/$defs/") {
            Some(d) => root.get("$defs").and_then(|defs| defs.get(d)),
            None => (r == "#").then_some(root),
        };
        let target = target.unwrap_or(&Value::Null);
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
    let forms: Vec<&Value> = list(s, "anyOf").chain(list(s, "oneOf")).collect();
    let mut fit: Vec<&Value> = forms.iter().copied().filter(|f| fits(f, v)).collect();
    if fit.iter().any(|f| named(f)) {
        fit.retain(|f| named(f));
    }
    let tries: Vec<Vec<String>> = (if fit.is_empty() { &forms } else { &fit })
        .iter()
        .map(|b| errors(root, b, v, at))
        .collect();
    match tries.as_slice() {
        [one] => out.extend(one.iter().cloned()),
        _ if tries.iter().all(|e| !e.is_empty()) && !tries.is_empty() => {
            let each: Vec<String> = tries.iter().map(|e| e.join(", ")).collect();
            out.push(format!(
                "{at}: matches none of its forms ({})",
                each.join(" | ")
            ));
        }
        _ => {}
    }
    if s.get("not")
        .is_some_and(|n| errors(root, n, v, at).is_empty())
    {
        out.push(format!("{at}: {} is not allowed", short(v)));
    }
    if let (Some(cond), Some(then)) = (s.get("if"), s.get("then"))
        && errors(root, cond, v, at).is_empty()
    {
        check(root, then, v, at, out);
    }
}

fn consts(form: &Value) -> impl Iterator<Item = (&String, &Value)> {
    let props = form.get("properties").and_then(Value::as_object);
    props
        .into_iter()
        .flatten()
        .filter_map(|(k, p)| Some((k, p.get("const")?)))
}

fn named(form: &Value) -> bool {
    consts(form).next().is_some()
}

fn fits(form: &Value, v: &Value) -> bool {
    let typed = form.get("type").and_then(Value::as_str);
    typed.is_none_or(|t| is(v, t)) && consts(form).all(|(k, c)| v.get(k) == Some(c))
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
        .is_some_and(|m| u64::try_from(len).is_ok_and(|l| l > m))
    {
        out.push(format!("{at}: has {len} items, over {}", s["maxItems"]));
    }
}
