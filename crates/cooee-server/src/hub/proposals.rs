//! Proposal cards (#56, #218, #261): an agent's MCP server, skill or GitHub routine waits on a card in its
//! thread; approving it (with Face ID for a server or a routine) does what it proposed, declining does nothing.

use cooee_core::{Body, Card, Change, Item, personas, shed_proto::ApprovalOption, skills};

use super::{Hub, HubError, Proposal, Proposed, SkillProposal, lock, now};

impl Hub {
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
        let persona = self.persona_of(id, token)?;
        let old = skills::check(&self.data, &persona, &p.skill, p.replaces.as_deref())
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

    /// An agent's `skill` read in thread `id` (#218): its persona's skill `name`'s body, at most 8,000 characters.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or no such skill of its.
    pub fn read_skill(&self, (id, token): (&str, &str), name: &str) -> Result<String, HubError> {
        let persona = self.persona_of(id, token)?;
        let s = skills::read(&self.data, &persona, name)
            .map_err(|e| HubError::Refused(format!("{e} for {persona}")))?;
        Ok(s.body.chars().take(8000).collect())
    }

    /// The id of thread `id`'s persona, for its token.
    fn persona_of(&self, id: &str, token: &str) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        personas::find(self.config_dir(), &self.data, &meta.profile)
            .map(|p| p.id)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))
    }

    /// A proposal's card in thread `id`: what it does, its form and its detail; `p` waits for the owner's answer.
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

    /// The owner's answer to a proposal: `approve` adds a server, off, or saves a skill.
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
            } => skills::save(&self.data, &persona, skill, replaces.as_deref())
                .map(|s| Some(format!("saved skill {}", s.name)))
                .map_err(|e| e.to_string()),
            Proposed::Later(then) => then().map(Some),
        }
    }
}
