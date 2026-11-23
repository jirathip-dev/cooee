mod ask_persona;
mod create_routine;
mod desk;
mod handoff;
mod history;
mod mail;
mod propose_server;
mod remember;
mod secret;
mod skill;
mod stop_routine;
mod update_persona;

use crate::mcp::Own;

pub(crate) const ALL: &[Own] = &[
    ask_persona::TOOL,
    create_routine::TOOL,
    desk::TOOL,
    handoff::TOOL,
    history::TOOL,
    mail::TOOL,
    propose_server::TOOL,
    remember::TOOL,
    secret::TOOL,
    skill::TOOL,
    stop_routine::TOOL,
    update_persona::TOOL,
];
