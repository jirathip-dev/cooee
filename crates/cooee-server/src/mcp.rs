//! MCP for agents (#56). Each server a thread's profile picks reaches its agent as `cooee mcp proxy <server>
//! --thread <id>` (stdio), which asks cooee for its grant (the command or URL, the variables and headers resolved in
//! memory, the allowed tools) with the thread's token, starts or connects to the real server, answers `tools/list`
//! with the allowed tools only and refuses a call to any other. `cooee mcp serve --thread <id>`, on every thread, is
//! cooee's own server: `propose_server`, which raises a card in the thread (nothing runs until the owner approves it
//! with Face ID), `remember`, a note in the thread's persona's memory, and `skill` (#218), its persona's skills. The token is in `COOEE_MCP` (`<cooee's URL> <token>`) and is good while the thread is
//! open.
mod grant;
mod own;
mod proxy;
mod rpc;
mod upstream;

pub use grant::*;
pub use own::*;
pub use proxy::*;
pub use rpc::*;
