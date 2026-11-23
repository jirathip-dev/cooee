use cooee_core::{
    Config, Kind,
    agents::Role,
    chief::{self, Carded, Change},
    disk, mcp, secrets,
};

use super::{Hub, HubError, proposals::Proposed};

pub struct Me(String);

impl Me {
    pub(crate) fn thread(&self) -> &str {
        &self.0
    }
}

impl Hub {
    pub(crate) fn chief_of(&self, id: &str, token: &str) -> Result<Me, HubError> {
        let t = self.token_of(id, token)?;
        let no = |why: &str| Err(HubError::Refused(why.to_owned()));
        if t.profile != chief::ID || self.persona(&t.profile).role != Role::Chief {
            return no("NOT_CHIEF: only Chief has this tool");
        }
        let (asked, run) = (
            !t.asked_by.is_empty(),
            t.routine.is_some() || t.workflow.is_some(),
        );
        if t.kind != Kind::Chat || !t.room.is_empty() || asked || run {
            return no("NOT_HERE: Chief's tools work only in its own chat with the owner");
        }
        Ok(Me(id.to_owned()))
    }

    pub(crate) fn propose_change(
        &self,
        me: &Me,
        c: Result<Carded, String>,
    ) -> Result<String, HubError> {
        let c = c.map_err(HubError::Refused)?;
        let secret = matches!(&c.change, Change::SecretSet(_));
        let form = if secret { "secret" } else { "change" };
        let card = (c.title.clone(), form, c.detail.clone());
        self.raise(me.thread(), card, Proposed::Change(Box::new(c)))?;
        Ok("Asked. The owner sees a card with the exact change; nothing changes before they approve it with Face ID.".to_owned())
    }

    pub(super) fn change_answer(
        &self,
        (id, item): (&str, u64),
        c: &Carded,
        approved: bool,
        value: Option<&str>,
    ) -> Result<String, String> {
        let done = if approved {
            self.apply_change(&c.change, value)
        } else {
            Ok("declined".to_owned())
        };
        let line = match &done {
            Ok(said) if approved => format!("[cooee] approved: {}: {said}", c.title),
            Ok(_) => format!("[cooee] declined: {}", c.title),
            Err(why) => format!("[cooee] failed: {}: {why}", c.title),
        };
        self.queue(id, &line, &format!("change-{item}"), &[], None)
            .ok();
        done
    }

    fn apply_change(&self, c: &Change, value: Option<&str>) -> Result<String, String> {
        let (data, e) = (&self.data, |e: std::io::Error| e.to_string());
        match c {
            Change::McpRemove(name) => self.edit_mcp(|list| {
                list.retain(|s| &s.name != name);
                Ok(format!("MCP server {name} is removed"))
            }),
            Change::McpSwitch(name, on) => self.edit_mcp(|list| {
                let s = list.iter_mut().find(|s| &s.name == name);
                s.ok_or(format!("MCP server {name} is gone"))?.enabled = *on;
                Ok(format!(
                    "MCP server {name} is {}",
                    if *on { "on" } else { "off" }
                ))
            }),
            Change::SecretSet(name) => {
                let v = value.filter(|v| !v.is_empty()).ok_or("no value was sent")?;
                mcp::set_secret(data, name, v).map_err(e)?;
                return Ok(format!("secret {name} is saved"));
            }
            Change::SecretRemove(name) => {
                secrets::remove(data, name).map_err(e)?;
                return Ok(format!("secret {name} is removed"));
            }
            Change::File {
                path,
                version,
                text,
            } => {
                if disk::version(&[path]) != *version {
                    return Err(
                        "changed on disk since the card was made: nothing was written, ask again"
                            .to_owned(),
                    );
                }
                cooee_core::write_private(path, text.as_bytes()).map_err(e)?;
                return Ok(format!("{} is saved", path.display()));
            }
            Change::MachineSetup(machine) => {
                let config = Config::load(self.config.as_deref().ok_or("no config file")?);
                crate::machine::run(&config.map_err(|e| e.to_string())?, machine, true)?;
                return Ok(format!("{machine} is set up"));
            }
        }
        .map_err(|e| e.to_string())
    }
}
