use std::fmt;

use blocks::{Action, Fields, catalog::Catalog, model};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Checked, prefix};

const MAX: usize = 8_000;
const TEXT: usize = 2_000;

/// The one answer a reply took.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sent {
    /// The action.
    pub action: String,
    /// Its values, the prefix off their names.
    pub values: Fields,
    /// What the chat shows under the reply: the option's or button's label, else the action.
    pub line: String,
    /// The device that sent it.
    pub by: String,
    /// When, in Unix milliseconds.
    pub at: u64,
}

/// A persona's reply as blocks, as a chat item holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    /// The blocks, ids `b<item>.<n>`.
    pub blocks: Checked,
    /// The owner's one answer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sent: Option<Sent>,
}

/// An accepted answer as the persona's next turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Told {
    /// The reply it answers.
    pub item: u64,
    /// What the persona gets.
    pub text: String,
    /// The queue's id for it: one per reply.
    pub random_id: String,
}

/// Why an answer is not delivered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unsent {
    /// No such reply of blocks.
    Gone,
    /// The reply took another answer.
    Answered,
    /// The persona's catalog doesn't send it.
    NotListed,
    /// A value the block doesn't take, by its name.
    Value(String),
}

impl fmt::Display for Unsent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gone => f.write_str("that reply is gone"),
            Self::Answered => f.write_str("that reply has its answer already"),
            Self::NotListed => f.write_str("the persona's blocks don't send that"),
            Self::Value(k) => write!(f, "`{k}` isn't a value that block takes"),
        }
    }
}

fn find<'a>(v: &'a Value, id: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) if o.get("id").and_then(Value::as_str) == Some(id) => Some(v),
        Value::Object(o) => o.values().find_map(|x| find(x, id)),
        Value::Array(l) => l.iter().find_map(|x| find(x, id)),
        _ => None,
    }
}

fn object(v: Option<&Value>) -> Fields {
    v.and_then(Value::as_object).cloned().unwrap_or_default()
}

fn text(v: Option<&Value>) -> &str {
    v.and_then(Value::as_str).unwrap_or_default()
}

fn choice(b: &Value, values: &Fields) -> Option<String> {
    let mut want = object(b.get("values"));
    let line = if let Some(said) = values.get("other") {
        let other = b.get("other")?;
        let said = said.as_str().filter(|s| s.chars().count() <= TEXT)?;
        want.insert("choice".into(), Value::from(text(other.get("for"))));
        want.insert("other".into(), Value::from(said));
        said.to_owned()
    } else {
        let options = b.get("options").and_then(Value::as_array)?;
        let picked = values.get("choice")?;
        let o = options.iter().find(|o| o.get("value") == Some(picked))?;
        want.extend(object(o.get("values")));
        want.insert("choice".into(), picked.clone());
        text(o.get("label")).to_owned()
    };
    (want == *values).then_some(line)
}

fn field(f: &Value, v: &Value) -> bool {
    match text(f.get("type")) {
        "text" => v.as_str().is_some_and(|s| s.chars().count() <= TEXT),
        "toggle" => v.is_boolean(),
        "picker" => f
            .get("options")
            .and_then(Value::as_array)
            .is_some_and(|o| o.contains(v)),
        _ => false,
    }
}

fn form(b: &Value, values: &Fields, head: &str) -> Result<String, Unsent> {
    let submit = b.get("submit");
    let fixed = object(submit.and_then(|s| s.get("values")));
    let fields = b.get("fields").and_then(Value::as_array);
    for (k, v) in values {
        let named = |f: &&Value| text(f.get("name")).strip_prefix(head) == Some(k.as_str());
        let fits = match fields.into_iter().flatten().find(named) {
            Some(f) => field(f, v),
            None => fixed.get(k) == Some(v),
        };
        if !fits {
            return Err(Unsent::Value(k.clone()));
        }
    }
    let label = text(submit.and_then(|s| s.get("label")));
    Ok(label.to_owned())
}

fn fit(b: &Value, a: &Action, values: &Fields, head: &str) -> Result<String, Unsent> {
    let action = |v: Option<&Value>| text(v) == a.action;
    let line = match text(b.get("type")) {
        "button" if action(b.get("action")) => (object(b.get("values")) == *values)
            .then(|| text(b.get("label")).to_owned())
            .ok_or_else(|| Unsent::Value("values".into()))?,
        "choice" if action(b.get("action")) => {
            choice(b, values).ok_or_else(|| Unsent::Value("choice".into()))?
        }
        "form"
            if action(b.get("action")) || action(b.get("submit").and_then(|s| s.get("action"))) =>
        {
            form(b, values, head)?
        }
        _ => return Err(Unsent::Value("action".into())),
    };
    Ok(if line.is_empty() {
        a.action.clone()
    } else {
        line
    })
}

/// The answer `a` to reply `item` (its blocks and its answer so far) under `catalog`, checked; `None` when the reply
/// took this same answer already.
///
/// # Errors
/// [`Unsent`] for an action no block of the reply sends under `catalog`, values that don't fit the block, or a reply
/// that took another answer.
pub fn told(
    catalog: &Catalog,
    (item, blocks, sent): (u64, &Checked, Option<&Sent>),
    a: &Action,
    (by, at): (&str, u64),
) -> Result<Option<(Sent, Told)>, Unsent> {
    let head = prefix(item);
    let values = model::own(&head, a).ok_or(Unsent::NotListed)?;
    if let Some(s) = sent {
        let same = s.action == a.action && s.values == values;
        return if same {
            Ok(None)
        } else {
            Err(Unsent::Answered)
        };
    }
    if !catalog.sends(&a.action) {
        return Err(Unsent::NotListed);
    }
    let tree = serde_json::to_value(blocks).unwrap_or_default();
    let block = a.block.as_deref().and_then(|id| find(&tree, id));
    let line = fit(block.ok_or(Unsent::Gone)?, a, &values, &format!("{head}."))?;
    let json = Value::Object(values.clone()).to_string();
    if json.len() > MAX {
        return Err(Unsent::Value("values".into()));
    }
    let told = Told {
        item,
        text: format!("[The owner answered your blocks]: {}\n{json}", a.action),
        random_id: format!("act-{item}"),
    };
    let sent = Sent {
        action: a.action.clone(),
        values,
        line,
        by: by.to_owned(),
        at,
    };
    Ok(Some((sent, told)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::show::{catalog, read};

    const FORM: &str = r#"{"blocks": [{"type": "form", "submit": {"label": "Book", "action": "submit"}, "fields": [
        {"name": "city", "type": "text", "label": "City"},
        {"name": "late", "type": "toggle", "label": "Late check-in"},
        {"name": "room", "type": "picker", "label": "Room", "options": ["single", "double"]}]}]}"#;

    fn send(values: &serde_json::Value) -> Result<Option<(Sent, Told)>, Unsent> {
        let ask = catalog("ask").unwrap();
        let blocks = read(ask, 4, FORM).unwrap();
        let tree = serde_json::to_value(&blocks).unwrap();
        let a = Action {
            screen: "/screens/chat/t".into(),
            block: tree[0]["id"].as_str().map(str::to_owned),
            action: "submit".into(),
            values: values.as_object().unwrap().clone(),
        };
        told(ask, (4, &blocks, None), &a, ("iPhone", 9))
    }

    #[test]
    fn a_form_takes_only_its_fields_each_of_its_type() {
        let good =
            serde_json::json!({ "b4.city": "Chiang Mai", "b4.late": true, "b4.room": "double" });
        let (sent, told) = send(&good).unwrap().unwrap();
        assert_eq!(
            (sent.line.as_str(), told.random_id.as_str()),
            ("Book", "act-4")
        );
        assert_eq!(sent.values["city"], "Chiang Mai");
        let long = "x".repeat(TEXT + 1);
        for (bad, field) in [
            (serde_json::json!({ "b4.late": "yes" }), "late"),
            (serde_json::json!({ "b4.room": "suite" }), "room"),
            (serde_json::json!({ "b4.city": long }), "city"),
            (serde_json::json!({ "b4.admin": true }), "admin"),
        ] {
            assert_eq!(send(&bad).unwrap_err(), Unsent::Value(field.into()));
        }
    }
}
