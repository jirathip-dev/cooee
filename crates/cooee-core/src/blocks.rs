use crate::shed_proto::ApprovalOption;
use blocks::{
    Block, Card, CardKind, Choice, ChoiceOption, Fallback, Fields, Island, Other, Run, Secret,
    Text, TextStyle,
};
use serde::{Serialize, Serializer};
use serde_json::Value;

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
            block: Option<Block>,
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
    /// The `device-key` island: the device signs `values` with its own key; a client without it draws `instead`.
    #[must_use]
    pub fn device_key(id: &str, mode: &str, send: &str, values: Fields, instead: &str) -> Block {
        let props = Fields::from_iter([
            ("mode".to_owned(), Value::from(mode)),
            ("send".to_owned(), Value::from(send)),
            ("values".to_owned(), Value::Object(values)),
        ]);
        let fallback = Fallback {
            title: "This app keeps no device key".into(),
            subtitle: Some(instead.into()),
            ..Fallback::default()
        };
        let island = Island::new("device-key", fallback)
            .id(id)
            .props(props)
            .events(vec![send.to_owned()]);
        Block::Island(Box::new(island))
    }

    /// Whether approving this card needs the device key: cooee's own proposals.
    #[must_use]
    pub fn needs_key(&self) -> bool {
        matches!(&self.body, Body::Approval { card, plugin: None, .. }
            if matches!(card.form.as_deref(), Some("mcp" | "routine" | "secret" | "change")))
    }

    /// The card block an approval is drawn as: an `ask` (a choice, or a thread to pick), `config` (its diff with
    /// apply or discard), `secret` (one masked field) or `run` card. Its action is `answer`, sent back as
    /// `POST /threads/<id>/approve`; once settled it shows its line instead.
    #[must_use]
    pub fn block(&self) -> Option<Block> {
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
            Some("config") => CardKind::Config,
            Some("secret") => CardKind::Secret,
            Some("run-card") => CardKind::Run,
            Some("mcp" | "routine") => CardKind::Proposal,
            _ => CardKind::Ask,
        };
        let mut blocks = Vec::new();
        if matches!(kind, CardKind::Config | CardKind::Proposal) {
            let lang = if matches!(kind, CardKind::Config) {
                "diff"
            } else {
                ""
            };
            let diff = Text::default().markdown(format!("```{lang}\n{detail}\n```"));
            blocks.push(Block::Text(Box::new(diff)));
        }
        let answer = format!("{id}-answer");
        let open =
            card.until.is_some() || (tapped.is_none() && chosen.is_none() && card.line.is_none());
        blocks.extend(open.then(|| match kind {
            CardKind::Secret => {
                let save = options.first().map_or("save", |o| o.id.as_str());
                let values = Fields::from_iter([("choice".to_owned(), Value::from(save))]);
                let secret = Secret::new(detail, "answer").id(answer).values(values);
                Block::Secret(Box::new(secret))
            }
            CardKind::Run => {
                let sha = card.sha.as_deref().unwrap_or("");
                Block::Run(Box::new(Run::new("answer", detail, sha).id(answer)))
            }
            _ => self.choice(options, plugin.is_some() && card.form.is_none(), answer),
        }));
        let keyed = open && self.needs_key() && options.iter().any(|o| o.id == "approve");
        blocks.extend(keyed.then(|| self.key(&id)));
        let line = settled_line(card, chosen.as_ref().or(tapped.as_ref()));
        blocks.extend(line.into_iter().chain(card.said.clone()).map(|l| {
            Block::Text(Box::new(
                Text::default().style(TextStyle::Footnote).markdown(l),
            ))
        }));
        let card = Card::default().id(id).kind(kind).title(tool).blocks(blocks);
        Some(Block::Card(Box::new(card)))
    }

    fn key(&self, id: &str) -> Block {
        let values = Fields::from_iter([
            ("item".to_owned(), Value::from(self.id)),
            ("choice".to_owned(), Value::from("approve")),
        ]);
        let instead = "Approve it on a device with a passkey";
        Self::device_key(&format!("{id}-key"), "approve", "answer", values, instead)
    }

    fn choice(&self, options: &[ApprovalOption], other: bool, answer: String) -> Block {
        let mut shown: Vec<_> = options
            .iter()
            .map(|o| ChoiceOption {
                value: o.id.clone(),
                label: o.name.clone(),
                ..ChoiceOption::default()
            })
            .collect();
        shown.extend(self.sayable().map(|_| ChoiceOption {
            value: Item::SAY.to_owned(),
            label: "Deny and say…".to_owned(),
            ..ChoiceOption::default()
        }));
        let mut choice = Choice::new("answer", shown).id(answer);
        let recommended = options.iter().find(|o| o.kind == "recommended");
        choice.recommended = recommended.map(|r| r.id.clone());
        if other {
            choice = choice.other(Other {
                placeholder: Some("other…".to_owned()),
                ..Other::default()
            });
        }
        Block::Choice(Box::new(choice))
    }
}

fn settled_line(card: &crate::chat::Card, picked: Option<&String>) -> Option<String> {
    let by = card
        .by
        .as_deref()
        .map(|b| format!(" · {b}"))
        .unwrap_or_default();
    card.line
        .clone()
        .or_else(|| picked.map(|c| format!("✓ {c}{by}")))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

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
            let block = serde_json::to_value(item.block()).unwrap();
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
            serde_json::to_value(open.block()).unwrap()["blocks"][1]["markdown"],
            "✓ split · conductor-t6"
        );
        assert_eq!(types(&conductors(Some("split"), None)), [json!("text")]);
        let stored = serde_json::to_string(&open).unwrap();
        assert!(stored.contains("\"block\""));
        assert_eq!(serde_json::from_str::<Item>(&stored).unwrap(), open);
    }
}
