use serde::{Serialize, Serializer};
use serde_json::{Value, json};

use crate::chat::{Body, Item};

impl Serialize for Item {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            id: u64,
            at: u64,
            seq: u64,
            #[serde(skip_serializing_if = "str::is_empty")]
            speaker: &'a str,
            #[serde(flatten)]
            body: &'a Body,
            #[serde(skip_serializing_if = "Option::is_none")]
            block: Option<Value>,
        }
        Wire {
            id: self.id,
            at: self.at,
            seq: self.seq,
            speaker: &self.speaker,
            body: &self.body,
            block: self.block(),
        }
        .serialize(s)
    }
}

impl Item {
    /// The card block an approval is drawn as: an `ask` (a choice, or a thread to pick), `config` (its diff with
    /// apply or discard), `secret` (one masked field) or `run` card. Its action is `answer`, sent back as
    /// `POST /threads/<id>/approve`; once settled it shows its line instead.
    #[must_use]
    pub fn block(&self) -> Option<Value> {
        let Body::Approval {
            tool,
            options,
            tapped,
            chosen,
            plugin,
            card,
        } = &self.body
        else {
            return None;
        };
        let (id, detail) = (self.id.to_string(), card.detail.as_deref().unwrap_or(""));
        let kind = match card.form.as_deref() {
            Some("config") => "config",
            Some("secret") => "secret",
            Some("run-card") => "run",
            _ => "ask",
        };
        let mut blocks = Vec::new();
        if kind == "config" {
            blocks.push(json!({ "type": "text", "markdown": format!("```\n{detail}\n```") }));
        }
        let answer = format!("{id}-answer");
        let open =
            card.until.is_some() || (tapped.is_none() && chosen.is_none() && card.line.is_none());
        blocks.extend(open.then(|| match kind {
            "secret" => json!({ "type": "secret", "id": answer, "name": detail, "action": "answer",
                "values": { "choice": "save" } }),
            "run" => json!({ "type": "run", "id": answer, "action": "answer", "command": detail,
                "sha": card.sha.as_deref().unwrap_or("") }),
            _ => {
                let recommended = options.iter().find(|o| o.kind == "recommended");
                let mut choice = json!({ "type": "choice", "id": answer, "action": "answer",
                    "options": options.iter().map(|o| json!({ "value": o.id, "label": o.name })).collect::<Vec<_>>() });
                if let Some(r) = recommended {
                    choice["recommended"] = json!(r.id);
                }
                if plugin.is_some() && card.form.is_none() {
                    choice["other"] = json!({ "placeholder": "other…" });
                }
                choice
            }
        }));
        let by = card
            .by
            .as_deref()
            .map(|b| format!(" · {b}"))
            .unwrap_or_default();
        let line = card.line.clone().or_else(|| {
            chosen
                .as_ref()
                .or(tapped.as_ref())
                .map(|c| format!("✓ {c}{by}"))
        });
        blocks.extend(line.map(|l| json!({ "type": "text", "style": "footnote", "markdown": l })));
        Some(json!({ "type": "card", "id": id, "kind": kind, "title": tool, "blocks": blocks }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::Card;

    fn conductors(chosen: Option<&str>, until: Option<u64>) -> Item {
        Item {
            id: 7,
            at: 1,
            seq: 0,
            speaker: String::new(),
            body: Body::Approval {
                tool: "Split #44?".to_owned(),
                options: Vec::new(),
                tapped: None,
                chosen: chosen.map(str::to_owned),
                plugin: Some("drover".to_owned()),
                card: Card {
                    by: Some("conductor-t6".to_owned()),
                    until,
                    ..Card::default()
                },
            },
        }
    }

    #[test]
    fn a_settled_card_asks_only_while_it_may_change_and_reads_back_without_its_block() {
        let types = |item: &Item| -> Vec<Value> {
            let block = item.block().unwrap();
            block["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .map(|b| b["type"].clone())
                .collect()
        };
        let open = conductors(Some("split"), Some(99));
        assert_eq!(types(&open), [json!("choice"), json!("text")]);
        assert_eq!(
            open.block().unwrap()["blocks"][1]["markdown"],
            "✓ split · conductor-t6"
        );
        assert_eq!(types(&conductors(Some("split"), None)), [json!("text")]);
        let stored = serde_json::to_string(&open).unwrap();
        assert!(stored.contains("\"block\""));
        assert_eq!(serde_json::from_str::<Item>(&stored).unwrap(), open);
    }
}
