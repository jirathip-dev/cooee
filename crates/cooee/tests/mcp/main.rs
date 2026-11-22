//! MCP servers (#56) end to end with the fake shed (tests/support), whose CLI keeps the `--mcp` file a start hands
//! it, so each test plays the agent: it starts every server in that file as Claude's and Codex's ACP adapters do
//! (their own `initialize`, id styles and order) and talks MCP to it. A fake stdio server offers tools a, b and c.
//! The registry refuses a literal token; a profile's allowlist is all the agent sees and may call; references
//! resolve when the proxy starts and their values reach no file, log or event; a closed thread's token stops
//! working; `propose_server` only raises a card, which needs Face ID and adds the server off. A `secret` (#163) the owner
//! drops with Face ID is typed into its terminal once, or saved for the persona, and reaches no transcript, event,
//! file, log or model input; the agent only reads `secret NAME entered`. A persona's `mail` (#220) sends only on its
//! approved card or a rule allowing the recipient, and a read's verification code is typed with `secret MAIL_CODE`,
//! never in what the agent reads or the thread keeps.

#[path = "../support/mod.rs"]
mod support;

mod browser;
mod desk;
mod http;
mod mail;
mod propose;
mod proxy;
mod registry;
mod secrets;
mod setup;
