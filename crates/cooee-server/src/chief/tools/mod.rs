mod access;
mod config;
mod look;
mod machine;
mod mcp_server;
mod persona;
mod room;
mod route;
mod routine;
mod stored_secret;

use crate::chief::Tool;

pub(crate) const ALL: &[Tool] = &[
    access::TOOL,
    config::TOOL,
    look::TOOL,
    machine::TOOL,
    mcp_server::TOOL,
    persona::TOOL,
    room::TOOL,
    route::TOOL,
    routine::TOOL,
    stored_secret::TOOL,
];
