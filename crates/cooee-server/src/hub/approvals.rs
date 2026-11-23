use std::{path::PathBuf, sync::Mutex};

use cooee_core::{
    Body, Calls as _, Card, Change, Item,
    action::{Action, Call},
    agents::{self, Agent, Home},
    rules::{Asked, Effect, Rule, Rules},
    shed_proto::{ApprovalOption, EventKind},
};
use serde_json::Value;

use super::{Hub, HubError, Live, lock, now};

impl Hub {
    pub(super) fn asked(&self, live: &mut Live, seq: u64, request: EventKind) -> Vec<Change> {
        let EventKind::ApprovalRequest {
            id,
            tool,
            options,
            call,
        } = request
        else {
            return Vec::new();
        };
        let call = call.unwrap_or_default();
        let field = |k: &str| call.get(k).and_then(Value::as_str);
        let at: Vec<PathBuf> = (call.get("locations").and_then(Value::as_array))
            .into_iter()
            .flatten()
            .filter_map(|l| l.get("path").and_then(Value::as_str).map(PathBuf::from))
            .collect();
        let read = Call {
            title: field("title").unwrap_or(&tool),
            kind: field("kind"),
            locations: &at,
            raw_input: call.get("rawInput"),
        };
        let action = self.providers.action(&live.meta.agent, &read);
        let persona = self.persona(&live.meta.profile);
        let folder = live.meta.folder.clone().unwrap_or_default();
        let unmatched = match persona.policy.as_deref() {
            Some("allow") => Effect::Allow,
            Some("deny") => Effect::Deny,
            _ => Effect::Ask,
        };
        let verdict = self.rules_of(&persona).decide(&action, &folder, unmatched);
        let asked = Asked {
            id,
            allow: once(&options, "allow_once", "allow"),
            deny: once(&options, "reject_once", "reject"),
            rule: Rule::proposed(&action, &folder),
        };
        if verdict.effect != Effect::Ask {
            let option = match verdict.effect {
                Effect::Allow => &asked.allow,
                _ => &asked.deny,
            };
            match self
                .shed
                .approve(&live.meta.terminal, option, Some(&asked.id))
            {
                Ok(()) => {
                    let row = field("toolCallId").zip(verdict.rule);
                    let row = row.and_then(|(c, r)| live.chat.ruled(c, r.words()));
                    return row.map(Change::Item).into_iter().collect();
                }
                Err(e) => cooee_core::note!("cooee: {}: a rule's answer: {e}", live.meta.id),
            }
        }
        let body = approval(action, asked);
        vec![Change::Item(live.chat.push(now(), seq, body))]
    }

    pub(super) fn rules_home(&self, persona: &Agent) -> Option<Home> {
        let config = self.config_dir().filter(|_| persona.lent.is_none())?;
        agents::home(config, &persona.id).ok()
    }

    pub(super) fn rules_of(&self, persona: &Agent) -> Rules {
        self.rules_home(persona).map_or_else(Rules::default, |h| {
            Rules::load(&h, persona.work_folder().as_deref())
        })
    }

    pub(super) fn shed_approve(
        &self,
        live: &Mutex<Live>,
        terminal: &str,
        tapped: Item,
        option: &str,
    ) -> Result<Option<Item>, HubError> {
        let asked = match &tapped.body {
            Body::Approval {
                card: Card {
                    request: Some(a), ..
                },
                ..
            } => Some(a.as_ref().clone()),
            _ => None,
        };
        let sent = match &asked {
            None => self
                .shed
                .approve(terminal, option, None)
                .map_err(Into::into),
            Some(a) => {
                let profile = lock(live).meta.profile.clone();
                self.answer(&profile, terminal, a, option)
            }
        };
        let mut live = lock(live);
        let done = live.chat.update(tapped.id, 0, |b| {
            if let Body::Approval {
                tapped,
                chosen,
                card,
                ..
            } = b
            {
                match &sent {
                    Ok(()) if asked.is_some() => *chosen = Some(option.to_owned()),
                    Ok(()) => {}
                    Err(_) => {
                        *tapped = None;
                        card.said = None;
                    }
                }
            }
        });
        if sent.is_err() || asked.is_some() {
            live.publish(self, done.clone().map(Change::Item).into_iter().collect())?;
        }
        sent.map(|()| done.or(Some(tapped)))
    }

    fn answer(
        &self,
        profile: &str,
        terminal: &str,
        a: &Asked,
        option: &str,
    ) -> Result<(), HubError> {
        let harness = match option {
            "allow" | "always" => &a.allow,
            "deny" => &a.deny,
            o => {
                return Err(HubError::Refused(format!(
                    "no option {o}: allow, always or deny"
                )));
            }
        };
        if option == "always" {
            let persona = self.persona(profile);
            let home = self.rules_home(&persona).ok_or_else(|| {
                HubError::Refused("this persona has no folder to keep rules in".to_owned())
            })?;
            let rule = Rule {
                by: "card".to_owned(),
                at: now(),
                ..a.rule.clone()
            };
            Rules::add(&home, persona.work_folder().as_deref(), rule)
                .map_err(|e| HubError::Refused(format!("the rule wasn't saved: {e}")))?;
        }
        Ok(self.shed.approve(terminal, harness, Some(&a.id))?)
    }

    pub(super) fn denied_tool(
        &self,
        persona: &Agent,
        folder: &std::path::Path,
        (server, tool): (&str, &str),
    ) -> Result<(), HubError> {
        let action = Action::tool(server, tool);
        let verdict = self.rules_of(persona).decide(&action, folder, Effect::Ask);
        match verdict.rule.filter(|_| verdict.effect == Effect::Deny) {
            Some(r) => Err(HubError::Refused(format!("{server}/{tool}: {}", r.words()))),
            None => Ok(()),
        }
    }
}

fn once(options: &[ApprovalOption], kind: &str, prefix: &str) -> String {
    let o = options.iter().find(|o| o.kind == kind);
    let o = o.or_else(|| options.iter().find(|o| o.kind.starts_with(prefix)));
    let o = o.or_else(|| options.iter().find(|o| o.id.starts_with(prefix)));
    o.map(|o| o.id.clone()).unwrap_or_default()
}

fn approval(action: Action, asked: Asked) -> Body {
    let option = |id: &str, name: String, kind: &str| ApprovalOption {
        id: id.to_owned(),
        name,
        kind: kind.to_owned(),
    };
    let options = vec![
        option("allow", "Allow".to_owned(), "allow_once"),
        option(
            "always",
            format!("Always allow {}", asked.rule.what()),
            "allow_always",
        ),
        option("deny", "Deny".to_owned(), "reject_once"),
    ];
    let card = Card {
        action: Some(action.kind),
        request: Some(Box::new(asked)),
        ..Card::default()
    };
    Body::Approval {
        tool: action.subject,
        options,
        tapped: None,
        chosen: None,
        plugin: None,
        card,
    }
}
