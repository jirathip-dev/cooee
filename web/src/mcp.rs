//! Settings › mcp servers (#56), HTML-first (#49) in #48's flat-list pattern: one row per server, `[stdio|http]
//! name · on/off · [edit on Mac] [remove]`; `+ server` (its card shows the exact command or URL, and approving needs
//! Face ID: `pair.ts`); a secret saved straight to the keychain; and each of the owner's profiles with a checkbox per
//! server and per tool. Every form posts and comes back here.

use std::collections::BTreeMap;

use cooee_core::{Profile, mcp::Server};
use topcoat::{Result, context::Cx, view::View, view::component, view::view};

use crate::document;

/// A server's tools (name, description), or why they couldn't be listed.
pub type Tools = Result<Vec<(String, String)>, String>;

/// What the page shows.
#[derive(Debug, Default)]
pub struct McpPage {
    /// The registry.
    pub servers: Vec<Server>,
    /// The owner's own profiles (a plugin's can't be changed here).
    pub profiles: Vec<Profile>,
    /// Each server's tools that are on.
    pub tools: BTreeMap<String, Tools>,
}

/// The page.
#[must_use]
pub fn mcp_page(cx: &Cx, page: McpPage) -> impl View {
    view! { cx => mcp(page: page) }
}

/// One server's row: `[stdio|http] name · on/off · [edit on Mac] [remove]`.
#[component]
async fn server_row(s: Server) -> Result<impl View> {
    let to = format!("/mcp/servers/{}", s.name);
    let (now, flip) = if s.enabled {
        ("on", "off")
    } else {
        ("off", "on")
    };
    Ok(view! {
        <div class="acr" data-server=(s.name.clone())>
            <span class="ptg">(if s.kind == "stdio" { "stdio" } else { "http" })</span>
            <span class="sk" title=(s.shown())>(s.name.clone())</span>
            <form method="post" action=(to.clone())>
                <input type="hidden" name="enabled" value=(flip)>
                <button
                    type="submit"
                    class="sec"
                    aria-pressed=(if s.enabled { "true" } else { "false" })
                    aria-label=(format!("{} is {now}: turn it {flip}", s.name))
                >
                    (now)
                </button>
            </form>
            <span class="st m">"edit on Mac"</span>
            <form method="post" action=(format!("{to}/remove"))>
                <button type="submit" class="sec" aria-label=(format!("remove {}", s.name))>"remove"</button>
            </form>
        </div>
    })
}

/// A profile's servers section: a checkbox per server that is on, and under it one per tool (none listed: the server's
/// whole set stays as it is).
#[component]
async fn picks(
    p: Profile,
    servers: Vec<Server>,
    tools: BTreeMap<String, Tools>,
) -> Result<impl View> {
    let on: Vec<Server> = servers.into_iter().filter(|s| s.enabled).collect();
    Ok(view! {
        <details class="thx">
            <summary>(p.label.clone())</summary>
            <form method="post" action=(format!("/profiles/{}/mcp", p.name)) class="mcp-p">
                for s in on {
                    <fieldset>
                        <legend>
                            <label>
                                <input
                                    type="checkbox"
                                    name=(format!("s:{}", s.name))
                                    checked=(p.mcp.iter().any(|k| k.server == s.name))
                                >
                                (s.name.clone())
                            </label>
                        </legend>
                        match tools.get(&s.name).cloned().unwrap_or_else(|| Err(String::new())) {
                            Ok(list) => {
                                for (t, about) in list {
                                    <label title=(about)>
                                        <input
                                            type="checkbox"
                                            name=(format!("t:{}:{t}", s.name))
                                            checked=(p.mcp.iter().any(|k| k.server == s.name && k.allows(&t)) || !p.mcp.iter().any(|k| k.server == s.name))
                                        >
                                        (t.clone())
                                    </label>
                                }
                            }
                            Err(why) => {
                                <p class="ds">(format!("tools not listed: {why}"))</p>
                            }
                        }
                    </fieldset>
                }
                <button type="submit">"save"</button>
            </form>
        </details>
    })
}

#[component]
async fn mcp(page: McpPage) -> Result<impl View> {
    Ok(view! {
        document(
            title: "mcp servers",
            <main class="sh set mcp" aria-labelledby="sht-mcp">
                <div class="hd pvh">
                    <a class="gl" href="/#settings" aria-label="back to settings">"‹"</a>
                    <b id="sht-mcp">"mcp servers"</b>
                </div>
                <div class="top">
                    for s in page.servers.clone() {
                        server_row(s: s)
                    }
                    <details class="thx" id="mcp-new">
                        <summary>"+ server"</summary>
                        <form method="post" action="/mcp/servers" id="mcp-add">
                            <input name="name" required=(true) aria-label="name" placeholder="name">
                            <select name="type" aria-label="type">
                                <option value="stdio">"stdio"</option>
                                <option value="http">"http"</option>
                                <option value="sse">"sse"</option>
                            </select>
                            <input name="target" required=(true) aria-label="command or URL" placeholder="command or URL">
                            <button type="submit">"review"</button>
                        </form>
                        <p class="ds">"you see the exact command or URL, then approve with Face ID; it is added off"</p>
                    </details>
                    <p class="ch">"secrets"</p>
                    <form method="post" action="/mcp/secret" class="mcp-s">
                        <input name="name" required=(true) aria-label="secret name" placeholder="NAME" autocomplete="off">
                        <input name="value" type="password" required=(true) aria-label="secret value" placeholder="value" autocomplete="off">
                        <button type="submit">"save to keychain"</button>
                    </form>
                    <p class="ds">"a server's env or headers name it as secret:NAME"</p>
                    <p class="ch">"agents"</p>
                    for p in page.profiles {
                        picks(p: p, servers: page.servers.clone(), tools: page.tools.clone())
                    }
                </div>
            </main>
        )
    })
}
