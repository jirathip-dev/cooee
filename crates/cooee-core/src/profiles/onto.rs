use crate::agents::Agent;

/// Owner agent `own` as it starts (#802): on its `profile`, that profile's text and launch keys; an error if unserved.
#[expect(clippy::missing_errors_doc, reason = "the line names the error")]
pub fn onto(own: &Agent, all: &[Agent]) -> Result<Agent, String> {
    let Some(id) = &own.profile else {
        return Ok(own.clone());
    };
    let served = all.iter().find(|a| &a.id == id && a.lent.is_some());
    let p = served.ok_or_else(|| format!("{}: profile {id} is not served", own.id))?;
    if let Some(why) = p.lent.as_ref().and_then(|l| l.hidden.as_ref()) {
        return Err(format!("{}: profile {id} is hidden: {why}", own.id));
    }
    let p = p.clone();
    Ok(Agent {
        mode: Some(p.mode()),
        purpose: own.purpose.clone().or(p.purpose),
        instructions: p.instructions,
        agent: p.agent,
        settings: p.settings,
        env: p.env,
        mcp: p.mcp,
        policy: p.policy,
        fallback: p.fallback,
        ..own.clone()
    })
}
