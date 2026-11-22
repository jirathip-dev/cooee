//! Personas over the API (#126) with the fake shed (tests/support): one is created, edited and deleted; an agent
//! folder's launch keys are read-only over the API and it runs with them (#359); a note by the persona (its `remember`
//! tool through cooee's MCP server) and one by the owner both list and reach its next session; clearing its memory,
//! or deleting it, leaves its chats. Plugins attach to a persona and detach (#129); an installed drover goes with every
//! orchestrator persona, by its role (#159); the first start moves an old conductor profile or persona into an
//! orchestrator's folder (#359).

#[path = "../support/mod.rs"]
mod support;

mod edit;
mod notes;
mod orchestrators;
mod plugins;
mod setup;
mod update;
mod voice;
