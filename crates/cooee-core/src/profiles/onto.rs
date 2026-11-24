use crate::agents::Agent;

/// Owner agent `own` as it starts (#802): standing on a plugin's profile (`profile`), it takes the profile's
/// instructions, agent, settings, variables, MCP picks, policy, fallback and mode from `all`, and keeps its own name,
/// avatar, character, purpose (else the profile's), model and machine.
///
/// # Errors
/// When `all` serves no such profile, or it is hidden.
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
