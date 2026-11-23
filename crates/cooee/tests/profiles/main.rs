//! Agent profiles end to end with the fake shed (tests/support), whose CLI runs the adapter command a start gives
//! it: Plain starts the agent with nothing added; a file profile's account, settings, env (its secret through the
//! adapter's shell), policy, its provider's session setup (#503) and instructions are applied at spawn; a plugin's
//! profile starts while the plugin is installed and is refused once it is removed; a guard its agent can't
//! run refuses the start; no secret reaches cooee's config, files or log, or what it tells shed.

#[path = "../support/mod.rs"]
mod support;

mod file;
mod persona;
mod plugin;
mod setup;
