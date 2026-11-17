use std::{io, path::Path};

use cooee_core::{
    Body, Card, Change, Config, Item, Launch, Thread,
    agents::{self, Agent, Home},
    browser, history,
    library::{self, Source, Update},
    mcp::{self, COOEE, Server},
    memory,
    shed_proto::ApprovalOption,
    skills::{self, Skill},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::mcp::Grant;

use super::{Hub, HubError, Recipe, lock, now, refused};

/// An agent's `propose_server` (#56): a server for the registry, and why.
#[derive(Debug, Deserialize)]
pub struct Proposal {
    /// Its name.
    pub name: String,
    /// `stdio`, `http` or `sse`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// A stdio server's program.
    #[serde(default)]
    pub command: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// An http or sse server's URL.
    #[serde(default)]
    pub url: String,
    /// What the owner gets from it.
    #[serde(default)]
    pub why: String,
}

pub(super) enum Proposed {
    Server(Server),
    Skill {
        persona: String,
        skill: Skill,
        replaces: Option<String>,
    },
    Later(Box<dyn FnOnce() -> Result<String, String> + Send>),
}

/// An agent's `skill` proposal (#218): the skill, and the one it edits.
#[derive(Debug, Deserialize)]
pub struct SkillProposal {
    /// The skill.
    #[serde(flatten)]
    pub skill: Skill,
    /// The name of the skill it edits; none: a new one.
    #[serde(default)]
    pub replaces: Option<String>,
}

impl Proposal {
    /// The server it would add: switched off, with no variables or headers (the Mac adds those).
    #[must_use]
    pub fn server(self) -> Server {
        Server {
            name: self.name,
            kind: self.kind,
            command: self.command,
            args: self.args,
            url: self.url,
            enabled: false,
            ..Server::default()
        }
    }
}

impl Hub {
    /// Agents' MCP proxies reach cooee at `url` (`cooee serve`, once it listens).
    pub fn listening(&self, url: String) {
        self.url.set(url).ok();
    }

    /// The registry, read from the config file now.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the config can't be read.
    pub fn mcp(&self) -> Result<Vec<Server>, HubError> {
        let Some(c) = &self.config else {
            return Ok(Vec::new());
        };
        Config::load(c)
            .map(|c| c.mcp)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    /// Changes the registry with `f` (its sentence on success), checks and saves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] with `f`'s or the check's reason, or no config file.
    pub fn edit_mcp(
        &self,
        f: impl FnOnce(&mut Vec<Server>) -> Result<String, String>,
    ) -> Result<String, HubError> {
        let _held = lock(&self.registry);
        let Some(c) = &self.config else {
            return Err(HubError::Refused(
                "there is no config file to save MCP servers in".to_owned(),
            ));
        };
        let mut list = self.mcp()?;
        let said = f(&mut list).map_err(HubError::Refused)?;
        mcp::check(&list).map_err(HubError::Refused)?;
        mcp::save(c, &list).map_err(|e| refused(c, &e))?;
        Ok(said)
    }

    pub(super) fn config_dir(&self) -> Option<&Path> {
        self.config.as_deref().and_then(Path::parent)
    }

    pub(super) fn persona(&self, id: &str) -> Agent {
        let list = agents::all(self.config_dir(), &self.data, |_| None);
        list.into_iter().find(|a| a.id == id).unwrap_or_default()
    }

    fn servers(&self) -> Result<Vec<Server>, HubError> {
        let mut list = self.mcp()?;
        if !list.iter().any(|s| s.name == browser::SERVER) {
            list.push(browser::server());
        }
        Ok(list)
    }

    fn chromium(&self) -> Result<std::path::PathBuf, HubError> {
        let Some(c) = &self.config else {
            return Ok(browser::CHROMIUM.into());
        };
        Config::load(c)
            .map(|c| c.chromium)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    fn grant_of(&self, s: Server, tools: Option<Vec<String>>) -> Result<Grant, String> {
        let (env, headers) = s.resolved(&self.data)?;
        Ok(Grant {
            kind: s.kind,
            command: s.command,
            args: s.args,
            url: s.url,
            env,
            headers,
            tools,
        })
    }

    pub(super) fn mcp_launch(&self, id: &str, recipe: &Recipe) -> Result<Launch, HubError> {
        let mut launch = recipe.launch.clone();
        let (Some(url), Ok(exe)) = (self.url.get(), std::env::current_exe()) else {
            return Ok(launch);
        };
        let servers = self.servers()?;
        let picked: Vec<&Server> = servers
            .iter()
            .filter(|s| recipe.mcp.contains(&s.name))
            .collect();
        for s in &picked {
            if !s.enabled {
                return Err(HubError::Refused(format!(
                    "MCP server {} is off: settings › mcp servers",
                    s.name
                )));
            }
            s.resolved(&self.data).map_err(HubError::Refused)?;
        }
        let var = json!([{ "name": crate::mcp::VAR, "value": format!("{url} {}", crate::mcp::token(&self.mcp_key, id)) }]);
        let exe = exe.to_string_lossy();
        let entry = |name: &str, args: &[&str]| json!({ "name": name, "command": exe, "args": args, "env": var });
        let mut all = vec![entry(COOEE, &["mcp", "serve", "--thread", id])];
        all.extend(
            picked
                .iter()
                .map(|s| entry(&s.name, &["mcp", "proxy", &s.name, "--thread", id])),
        );
        let file = self
            .data
            .join("secrets")
            .join(format!(".mcp-{id}-{:x}.json", now()));
        launch.mcp = Some((file, all));
        Ok(launch)
    }

    pub(super) fn unconnected(&self, recipe: &Recipe) -> Result<String, HubError> {
        let servers = self.servers()?;
        let missing: Vec<&str> = recipe
            .mcp
            .iter()
            .filter(|n| !servers.iter().any(|s| &s.name == *n))
            .map(String::as_str)
            .collect();
        Ok(if missing.is_empty() {
            String::new()
        } else {
            format!(
                "These MCP servers you expect aren't connected yet: {}. Before anything else, your first reply tells \
                 the owner to connect each one by that name in Settings › MCP servers (or `cooee mcp add`); until \
                 then you can't use it.",
                missing.join(", ")
            )
        })
    }

    /// Thread `id`, if `token` is its MCP token and it is open.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token or a closed thread.
    pub fn token_of(&self, id: &str, token: &str) -> Result<Thread, HubError> {
        let no = || {
            HubError::Refused(format!(
                "this MCP token isn't thread {id}'s, or the thread is closed"
            ))
        };
        if token != crate::mcp::token(&self.mcp_key, id) {
            return Err(no());
        }
        let meta = self.thread(id).map_err(|_| no())?.0;
        if meta.closed {
            return Err(no());
        }
        Ok(meta)
    }

    /// The grant of thread `id`'s proxy for `server` (`token`: the thread's): its pick from its profile, the server
    /// on, resolved now.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a server it doesn't pick, one off, or a reference
    /// that doesn't resolve.
    pub fn grant(&self, id: &str, token: &str, server: &str) -> Result<Grant, HubError> {
        let (persona, pick, s) = self.granted(id, token, server)?;
        let mut g = self.grant_of(s, pick.tools).map_err(HubError::Refused)?;
        if server == browser::SERVER {
            let profile = browser::profile(&self.data, &persona.id);
            let cdp = browser::endpoint(&self.chromium()?, &profile)
                .map_err(|e| HubError::Refused(format!("{}'s browser: {e}", persona.id)))?;
            g.args.extend(["--cdp-endpoint".to_owned(), cdp]);
        }
        Ok(g)
    }

    fn granted(
        &self,
        id: &str,
        token: &str,
        server: &str,
    ) -> Result<(Agent, mcp::Pick, Server), HubError> {
        let meta = self.token_of(id, token)?;
        let persona = self.persona(&meta.profile);
        let pick = persona.mcp.iter().find(|p| p.server == server).cloned();
        let pick = pick.ok_or_else(|| {
            HubError::Refused(format!(
                "thread {id}'s profile doesn't pick MCP server {server}"
            ))
        })?;
        let s = self
            .servers()?
            .into_iter()
            .find(|s| s.name == server && s.enabled);
        let s =
            s.ok_or_else(|| HubError::Refused(format!("MCP server {server} is off or gone")))?;
        Ok((persona, pick, s))
    }

    /// Thread `id`'s proxy's `tools/call` of `tool` on `server` with `args`, checked again: the grant still holds, the
    /// tool is allowed and, on the browser, a `url` is on the persona's site allowlist; each browser call waits while
    /// the owner holds control, is logged, and answers the owner's hand-back note.
    ///
    /// # Errors
    /// [`HubError::Refused`] as [`Hub::grant`], for a tool not allowed, or a site not on the allowlist.
    pub fn call(
        &self,
        (id, token): (&str, &str),
        (server, tool): (&str, &str),
        args: &Value,
    ) -> Result<Option<String>, HubError> {
        let (persona, pick, _) = self.granted(id, token, server)?;
        if !pick.allows(tool) {
            return Err(HubError::Refused(format!(
                "tool {tool} isn't allowed in this thread (its profile's tools for {server})"
            )));
        }
        if server != browser::SERVER {
            return Ok(None);
        }
        let note = self.wait_turn(&persona.id);
        let url = args.get("url").and_then(Value::as_str);
        let off = url.filter(|u| !browser::allows(&persona.sites, u));
        let line = json!({ "at": now(), "thread": id, "tool": tool, "arguments": args, "refused": off.is_some() });
        let profile = browser::profile(&self.data, &persona.id);
        browser::log(&profile, &line).map_err(|e| refused(&profile, &e))?;
        match off {
            Some(u) => Err(HubError::Refused(format!(
                "{u} isn't on {}'s site allowlist",
                persona.id
            ))),
            None => Ok(note),
        }
    }

    /// An agent's `remember` in thread `id` (#126, #216): an edit of its persona's notes, for its next sessions; the
    /// answer names the note's id and the characters used. In a routine's run (#219) it only raises a card, and the
    /// edit is made when the owner approves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or an edit refused.
    pub fn remember(&self, id: &str, token: &str, e: &memory::Edit) -> Result<String, HubError> {
        let home = self.home_of(id, token)?;
        let meta = self.token_of(id, token)?;
        if meta.routine.is_some() {
            let what = match (&e.text, &e.replaces) {
                (Some(_), Some(r)) => format!("replace {}'s note [{r}]", meta.profile),
                (Some(_), None) => format!("add a note to {}'s memory", meta.profile),
                (None, _) => format!("forget {}'s notes {}", meta.profile, e.forget.join(", ")),
            };
            let (detail, e) = (e.text.clone().unwrap_or_default(), e.clone());
            let then = Box::new(move || {
                memory::remember(&home, "persona", &e, now())
                    .map(|n| {
                        n.map_or_else(|| "forgotten".to_owned(), |n| format!("kept as [{}]", n.id))
                    })
                    .map_err(|e| e.to_string())
            });
            self.raise(id, (what, "memory", detail), Proposed::Later(then))?;
            return Ok(
                "Proposed. The owner sees a card; nothing is saved until they approve it."
                    .to_owned(),
            );
        }
        let kept = memory::remember(&home, "persona", e, now())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let used = memory::used(&memory::notes(&home));
        let what = kept.map_or_else(|| "Forgotten".to_owned(), |n| format!("Kept as [{}]", n.id));
        Ok(format!(
            "{what}; {used} of {} characters used.",
            memory::CAP
        ))
    }

    /// What persona `persona`'s threads said after line `mark` (#219), routine runs left out: none when the owner
    /// said nothing since.
    ///
    /// # Errors
    /// [`HubError::Refused`] when a thread can't be read.
    pub fn news(&self, persona: &str, mark: u64) -> Result<Option<(String, u64)>, HubError> {
        let mine: Vec<String> = self
            .all()
            .iter()
            .map(|l| lock(l).meta.clone())
            .filter(|m| m.profile == persona && m.routine.is_none())
            .map(|m| m.id)
            .collect();
        history::since(&self.store, &mine, mark).map_err(|e| HubError::Refused(e.to_string()))
    }

    /// An agent's `history` in thread `id` (#217): a search of its persona's threads (`query`, at most `limit`
    /// hits), or a page of one of them (`thread`, before item `before`).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a thread of another persona.
    pub fn history(
        &self,
        (id, token): (&str, &str),
        (query, limit): (&str, usize),
        thread: &str,
        before: Option<u64>,
    ) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let persona = agents::find(self.config_dir(), &self.data, &meta.profile)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))?;
        let refused = |e: cooee_core::StoreError| HubError::Refused(e.to_string());
        let run = meta
            .routine
            .as_ref()
            .is_some_and(|r| r.id == agents::CONSOLIDATE);
        if run && thread.is_empty() && query.trim().is_empty() {
            let key = format!("{}-{}", agents::CONSOLIDATE, persona.id);
            let read = history::watermark(&self.data, &key, |w| {
                let news = self.news(&persona.id, w.line);
                if let Ok(Some((_, newest))) = &news {
                    w.line = *newest;
                }
                news
            });
            return read
                .map_err(|e| HubError::Refused(e.to_string()))?
                .map(|n| {
                    n.map_or_else(
                        || "nothing new since the last run".to_owned(),
                        |(text, _)| text,
                    )
                });
        }
        if thread.is_empty() && query.trim().is_empty() {
            return Err(HubError::Refused("give a query or a thread".to_owned()));
        }
        if thread.is_empty() {
            let mine: Vec<String> = self
                .all()
                .iter()
                .map(|l| lock(l).meta.clone())
                .filter(|m| m.profile == persona.id)
                .map(|m| m.id)
                .collect();
            return cooee_core::history::search(&self.store, &mine, query, limit).map_err(refused);
        }
        match self.thread(thread) {
            Ok((t, _)) if t.profile == persona.id => {
                cooee_core::history::read(&self.store, thread, before).map_err(refused)
            }
            _ => Err(HubError::Refused(format!(
                "thread {thread} isn't one of {}'s",
                persona.id
            ))),
        }
    }

    /// An agent's `propose_server` in thread `id`: a card with the exact command or URL; nothing runs, and the
    /// server is added (off) only when the owner approves it with Face ID.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a server that breaks a rule or a name taken.
    pub fn propose(&self, id: &str, token: &str, p: Proposal) -> Result<Item, HubError> {
        self.token_of(id, token)?;
        let why: String = p.why.trim().chars().take(300).collect();
        let s = p.server();
        s.check().map_err(HubError::Refused)?;
        if self.mcp()?.iter().any(|o| o.name == s.name) {
            return Err(HubError::Refused(format!(
                "there is an MCP server {} already",
                s.name
            )));
        }
        let tool = format!("add MCP server {} ({}): {why}", s.name, s.kind);
        self.raise(id, (tool, "mcp", s.shown()), Proposed::Server(s))
    }

    /// An agent's `skill` proposal in thread `id` (#218): a card with the skill's file, or an edit's diff; nothing
    /// is saved until the owner approves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a skill [`skills::check`]
    /// refuses.
    pub fn propose_skill(
        &self,
        (id, token): (&str, &str),
        p: SkillProposal,
    ) -> Result<Item, HubError> {
        let home = self.home_of(id, token)?;
        let persona = self.token_of(id, token)?.profile;
        let old = skills::check(&home, &p.skill, p.replaces.as_deref())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let verb = if p.replaces.is_some() { "edit" } else { "add" };
        let tool = format!(
            "{verb} {persona}'s skill {}: {}",
            p.skill.name, p.skill.description
        );
        let detail = skills::diff(&old, &p.skill.file());
        let pending = Proposed::Skill {
            persona,
            skill: p.skill,
            replaces: p.replaces,
        };
        self.raise(id, (tool, "skill", detail), pending)
    }

    /// Brings thread `id`'s persona's library skills up to `sources` (#496), fetching in `work`: an unedited one is
    /// replaced, an edited one kept and its update raised as a `skill` diff card in the thread. Says what it did.
    ///
    /// # Errors
    /// [`HubError::NoThread`]; [`HubError::Refused`] for a thread with no persona or an update that fails.
    pub fn update_skills(
        &self,
        id: &str,
        sources: &[Source],
        work: &Path,
    ) -> Result<Vec<String>, HubError> {
        let thread = self.list().into_iter().find(|t| t.id == id);
        let persona = thread
            .ok_or_else(|| HubError::NoThread(id.to_owned()))?
            .persona;
        let home = self
            .config_dir()
            .ok_or_else(|| io::Error::other("no config folder"))
            .and_then(|c| agents::home(c, &persona))
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))?;
        let done =
            library::update(&home, sources, work).map_err(|e| HubError::Refused(e.to_string()))?;
        let mut said = Vec::new();
        for (name, u) in done {
            said.push(match u {
                Update::Current => format!("{name}: up to date"),
                Update::Updated => format!("{name}: updated"),
                Update::Kept { new, old } => {
                    let tool = format!("update {persona}'s skill {name}: you edited it, so it stays until you approve");
                    let detail = skills::diff(&old, &new.file());
                    let pending = Proposed::Skill {
                        persona: persona.clone(),
                        skill: new,
                        replaces: Some(name.clone()),
                    };
                    self.raise(id, (tool, "skill", detail), pending)?;
                    format!("{name}: you edited it, so it is kept; the card shows the update")
                }
            });
        }
        Ok(said)
    }

    /// An agent's `skill` read in thread `id` (#218): its persona's skill `name`'s body, at most 8,000 characters.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or no such skill of its.
    pub fn read_skill(&self, (id, token): (&str, &str), name: &str) -> Result<String, HubError> {
        let home = self.home_of(id, token)?;
        let s = skills::read(&home, name).map_err(|e| {
            let persona = home
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            HubError::Refused(format!("{e} for {persona}"))
        })?;
        Ok(s.body.chars().take(8000).collect())
    }

    fn home_of(&self, id: &str, token: &str) -> Result<Home, HubError> {
        let meta = self.token_of(id, token)?;
        let config = self
            .config_dir()
            .ok_or_else(|| HubError::Refused("no config folder".to_owned()))?;
        agents::home(config, &meta.profile)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))
    }

    fn raise(
        &self,
        id: &str,
        (tool, form, detail): (String, &str, String),
        p: Proposed,
    ) -> Result<Item, HubError> {
        let option = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let body = Body::Approval {
            tool,
            options: vec![option("approve"), option("decline")],
            tapped: None,
            chosen: None,
            plugin: None,
            card: Card {
                form: Some(form.to_owned()),
                detail: Some(detail),
                ..Card::default()
            },
        };
        let live = self.live(id)?;
        let mut live = lock(&live);
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        lock(&self.proposals).insert((id.to_owned(), item.id), p);
        Ok(item)
    }

    /// A card in thread `id` saying `tool` and `detail`; approving it (with Face ID) runs `then` (#261).
    ///
    /// # Errors
    /// [`HubError::NoThread`] for no such thread.
    pub fn propose_later(
        &self,
        id: &str,
        (tool, detail): (String, String),
        then: Box<dyn FnOnce() -> Result<String, String> + Send>,
    ) -> Result<Item, HubError> {
        self.raise(id, (tool, "routine", detail), Proposed::Later(then))
    }

    /// Whether item `item` of thread `id` is an MCP server's or a routine's proposal card (approving it needs Face
    /// ID).
    #[must_use]
    pub fn is_proposal(&self, id: &str, item: u64) -> bool {
        let Ok(live) = self.live(id) else {
            return false;
        };
        let live = lock(&live);
        live.chat.items.iter().any(|i| {
            i.id == item && matches!(&i.body, Body::Approval { card, .. } if matches!(card.form.as_deref(), Some("mcp" | "routine")))
        })
    }

    pub(super) fn proposal_answer(
        &self,
        id: &str,
        item: u64,
        option: &str,
    ) -> Result<Option<String>, String> {
        let p = lock(&self.proposals).remove(&(id.to_owned(), item));
        if option != "approve" {
            return Ok(Some("declined".to_owned()));
        }
        match p.ok_or("this proposal ran out (cooee restarted): ask the agent again")? {
            Proposed::Server(s) => self.add_mcp(s).map(Some).map_err(|e| e.to_string()),
            Proposed::Skill {
                persona,
                skill,
                replaces,
            } => self
                .config_dir()
                .ok_or_else(|| io::Error::other("no config folder"))
                .and_then(|c| agents::home(c, &persona))
                .and_then(|h| skills::save(&h, skill, replaces.as_deref()))
                .map(|s| Some(format!("saved skill {}", s.name)))
                .map_err(|e| e.to_string()),
            Proposed::Later(then) => then().map(Some),
        }
    }

    /// Adds `s` to the registry, off (an agent's proposal, or the phone's `+ server`, both after Face ID).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a name taken or a server that breaks a rule.
    pub fn add_mcp(&self, s: Server) -> Result<String, HubError> {
        self.edit_mcp(|list| {
            if list.iter().any(|o| o.name == s.name) {
                return Err(format!(
                    "there is an MCP server {} already: change it on the Mac",
                    s.name
                ));
            }
            let said = format!(
                "added {}, off: turn it on in settings › mcp servers",
                s.name
            );
            list.push(Server {
                enabled: false,
                ..s
            });
            Ok(said)
        })
    }
}
