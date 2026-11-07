//! The new thread (#48): a composer with one row of chips, `[<provider> · <account> ▾] [<folder> ▾]`, each opening
//! its sheet (a popover of radios in the form, so it opens with no script). The agent chip starts at the pick last
//! used in the chosen folder, from the thread store. Also the sidebar (from 1024 px) and Settings › providers, the
//! flat list of accounts.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use cooee_core::{AGENTS_FIX, Account, Agent, ROOTS_FIX, Sheet, Status, Summary};
use topcoat::{
    Result,
    context::Cx,
    view::{View, component, view},
};

use crate::{age, short, state};

/// A provider's name for ACP agent `agent` (`claude` for `claude-acp`): never the agent's id on screen.
#[must_use]
pub fn named(agent: &str, providers: &[Sheet]) -> String {
    providers
        .iter()
        .find(|s| s.provider.agent == agent)
        .map_or_else(
            || agent.strip_suffix("-acp").unwrap_or(agent).to_owned(),
            |s| s.provider.name.clone(),
        )
}

/// Who thread or pick `account` of `agent` runs as (an email, an account's name or `main login`); empty for an
/// agent no provider signs in.
#[must_use]
pub fn who(agent: &str, account: &str, providers: &[Sheet]) -> String {
    let Some(sheet) = providers.iter().find(|s| s.provider.agent == agent) else {
        return String::new();
    };
    let found = sheet.accounts.iter().find(|(a, _)| a.name == account);
    match found {
        Some((a, s)) => a.shown(s),
        None if account.is_empty() => "main login".to_owned(),
        None => account.to_owned(),
    }
}

/// `<provider> · <account>`, or the provider alone.
fn said(name: &str, who: &str) -> String {
    if who.is_empty() {
        name.to_owned()
    } else {
        format!("{name} · {who}")
    }
}

/// What a new thread may run with: an installed agent and one of its provider's accounts.
#[derive(Clone)]
struct Choice {
    agent: String,
    account: String,
    name: String,
    who: String,
    signed_in: bool,
}

impl Choice {
    fn key(&self) -> String {
        format!("{}/{}", self.agent, self.account)
    }
}

/// Every installed agent's accounts, by agent: an agent whose provider's CLI isn't installed is left out, one no
/// provider signs in is one pick.
fn choices(agents: &[Agent], providers: &[Sheet]) -> Vec<Choice> {
    let mut out = Vec::new();
    for agent in agents {
        let name = named(&agent.id, providers);
        match providers.iter().find(|s| s.provider.agent == agent.id) {
            Some(s) if s.accounts.iter().any(|(_, st)| st.install.is_some()) => {}
            Some(s) => out.extend(s.accounts.iter().map(|(a, st)| Choice {
                agent: agent.id.clone(),
                account: a.name.clone(),
                name: name.clone(),
                who: a.shown(st),
                signed_in: st.signed_in,
            })),
            None => out.push(Choice {
                agent: agent.id.clone(),
                account: String::new(),
                name,
                who: String::new(),
                signed_in: true,
            }),
        }
    }
    out
}

/// A folder as the thread store and the pickers compare it.
fn same(folder: &Path) -> PathBuf {
    folder.canonicalize().unwrap_or_else(|_| folder.to_owned())
}

/// The pick last used in each folder (`rows` newest first) and overall, among `choices` signed in; with none used,
/// the first signed in (else the first).
fn last_used(rows: &[Summary], choices: &[Choice]) -> (BTreeMap<PathBuf, String>, Option<String>) {
    let mut by = BTreeMap::new();
    let mut overall = None;
    for r in rows {
        let ok = choices
            .iter()
            .any(|c| c.signed_in && c.agent == r.agent && c.account == r.account);
        if ok {
            let key = format!("{}/{}", r.agent, r.account);
            by.entry(same(&r.folder)).or_insert_with(|| key.clone());
            overall.get_or_insert(key);
        }
    }
    let first = choices.iter().find(|c| c.signed_in).or(choices.first());
    (by, overall.or_else(|| first.map(Choice::key)))
}

/// What the composer needs: the installed agents (or why shed couldn't list them) and the folders with their depth.
pub struct New {
    /// `shed agents`, as last checked.
    pub agents: Result<Vec<Agent>, String>,
    /// The roots and the folders inside them (depth 0 to 2).
    pub folders: Vec<(PathBuf, u8)>,
}

/// One folder in the folder sheet: its path, as shown, its depth (recent: 0), the pick last used in it and its
/// label.
struct Spot {
    path: PathBuf,
    depth: u8,
    last: Option<String>,
    said: String,
}

/// The folder sheet's folders: recent first (the folders with threads, newest first, as the folder list spells them),
/// then the rest; and how many are recent.
fn spots(
    rows: &[Summary],
    folders: Vec<(PathBuf, u8)>,
    by: &BTreeMap<PathBuf, String>,
    picks: &[Choice],
) -> (Vec<Spot>, usize) {
    let spot = |path: PathBuf, depth: u8, key: &PathBuf| {
        let last = by.get(key).cloned();
        let c = last
            .as_ref()
            .and_then(|l| picks.iter().find(|c| c.key() == *l));
        Spot {
            path,
            depth,
            last,
            said: c.map(|c| said(&c.name, &c.who)).unwrap_or_default(),
        }
    };
    let mut rest: Vec<(PathBuf, u8, PathBuf)> = folders
        .into_iter()
        .map(|(f, d)| {
            let key = same(&f);
            (f, d, key)
        })
        .collect();
    let mut out = Vec::new();
    for r in rows {
        let key = same(&r.folder);
        if let Some(i) = rest.iter().position(|(_, _, k)| *k == key) {
            let (path, _, key) = rest.remove(i);
            out.push(spot(path, 0, &key));
        }
    }
    let recent = out.len();
    out.extend(rest.into_iter().map(|(p, d, k)| spot(p, d, &k)));
    (out, recent)
}

/// An empty picker's row instead: why, and the line that fixes it with its copy button (`chat.ts`).
#[component]
pub async fn hint(why: &str, fix: &str, then: &str) -> Result<impl View> {
    Ok(view! {
        <p class="hn">
            (why)
            <code>(fix)</code>
            (then)
            <button type="button" data-copy=(fix) aria-label=(format!("copy {fix}"))>
                "copy"
            </button>
        </p>
    })
}

/// A provider's sign-in terminal (drawn by `chat.ts`), opened under the row that started it.
#[component]
pub async fn login_panel(provider: String, command: String) -> Result<impl View> {
    Ok(view! {
        <div class="lg" data-provider=(provider.clone()) hidden=(true)>
            <p class="ds">(format!("{command} · shed terminal"))</p>
            <pre class="out"></pre>
            <div class="lgf">
                <input
                    aria-label=(format!("the code from the {provider} sign-in page"))
                    placeholder="paste the code"
                    autocomplete="off"
                >
                <button type="button" class="lgs">"send"</button>
                <button type="button" class="sec cn">"cancel"</button>
            </div>
        </div>
    })
}

/// A sheet's header in a form: its title and ×.
#[component]
async fn pop_head(which: &str) -> Result<impl View> {
    Ok(view! {
        <div class="hd">
            <b id=(format!("sht-{which}"))>(which)</b>
            <button
                type="button"
                class="gl"
                popovertarget=(format!("sh-{which}"))
                popovertargetaction="hide"
                aria-label="close"
            >
                "×"
            </button>
        </div>
    })
}

/// The new-thread composer (`/new`, and `/` from 1024 px): the chip row, the box and [send], and the chips'
/// sheets. No agents or no folders: the line that fixes it, and send waits.
#[component]
pub async fn composer(rows: Vec<Summary>, new: New, providers: Vec<Sheet>) -> Result<impl View> {
    let New { agents, folders } = new;
    let (installed, err) = match agents {
        Ok(found) => (found, None),
        Err(e) => (Vec::new(), Some(e)),
    };
    let none = installed.is_empty() && err.is_none();
    let picks = choices(&installed, &providers);
    let (by, def) = last_used(&rows, &picks);
    let (spots, recent) = spots(&rows, folders, &by, &picks);
    let at = spots.first().map(|s| s.path.clone());
    let pick = spots.first().and_then(|s| s.last.clone()).or(def.clone());
    let chosen = picks.iter().find(|c| Some(c.key()) == pick);
    let chip = chosen.map_or("no agent".to_owned(), |c| said(&c.name, &c.who));
    let to = chosen.map_or(String::new(), |c| c.name.clone());
    let can = chosen.is_some() && at.is_some();
    let place = at.as_deref().map(short).unwrap_or_default();
    let base = at
        .as_deref()
        .and_then(Path::file_name)
        .map_or_else(|| place.clone(), |n| n.to_string_lossy().into_owned());
    Ok(view! {
        <form class="tw nc" id="nc" data-def=(def.unwrap_or_default())>
            if none {
                hint(
                    why: "no agents installed · on the Mac:",
                    fix: AGENTS_FIX,
                    then: ""
                )
            }
            if let Some(e) = err {
                <p class="err">(e)</p>
            }
            if at.is_none() {
                hint(
                    why: "no folders set · add",
                    fix: ROOTS_FIX,
                    then: "to config.toml"
                )
            }
            <div class="chips">
                <button
                    type="button"
                    class="chip"
                    id="chip-agent"
                    popovertarget="sh-agent"
                    aria-label=(format!("agent: {chip}"))
                >
                    <span>(chip.clone())</span>
                    <i aria-hidden="true">"▾"</i>
                </button>
                <button
                    type="button"
                    class="chip"
                    id="chip-folder"
                    popovertarget="sh-folder"
                    aria-label=(format!("folder: {place}"))
                    title=(place.clone())
                >
                    <span>(base)</span>
                    <i aria-hidden="true">"▾"</i>
                </button>
            </div>
            <textarea
                name="text"
                rows="1"
                aria-label="first message"
                placeholder=(format!("message {to}"))
            ></textarea>
            <button type="submit" disabled=(!can)>"send"</button>
            <p class="err" id="new-err" hidden=(true)></p>
            agent_sheet(picks: picks, providers: providers, pick: pick)
            folder_sheet(spots: spots, recent: recent)
        </form>
    })
}

/// The agent sheet: each installed provider's accounts (a signed-out one muted, with [sign in]), `pick` checked,
/// and their sign-in terminals.
#[component]
async fn agent_sheet(
    picks: Vec<Choice>,
    providers: Vec<Sheet>,
    pick: Option<String>,
) -> Result<impl View> {
    // The accounts by provider, in the agents' order.
    let mut sections: Vec<(String, String, Vec<Choice>)> = Vec::new();
    for c in picks {
        match sections.last_mut() {
            Some((agent, _, them)) if *agent == c.agent => them.push(c),
            _ => sections.push((c.agent.clone(), c.name.clone(), vec![c])),
        }
    }
    let panels: Vec<(String, String)> = providers
        .iter()
        .filter(|s| sections.iter().any(|(a, _, _)| *a == s.provider.agent))
        .map(|s| (s.provider.name.clone(), s.provider.login.join(" ")))
        .collect();
    Ok(view! {
        <div
            class="sh pop"
            id="sh-agent"
            popover="auto"
            role="dialog"
            aria-labelledby="sht-agent"
        >
            pop_head(which: "agent")
            <fieldset class="top">
                <legend class="vh">"provider and account"</legend>
                for (_, name, them) in sections {
                    <p class="ch">(name)</p>
                    for c in them {
                        <div class=(if c.signed_in { "sr pk" } else { "sr pk so" })>
                            <label>
                                <input
                                    type="radio"
                                    name="pick"
                                    value=(c.key())
                                    data-agent=(c.agent.clone())
                                    data-account=(c.account.clone())
                                    data-name=(c.name.clone())
                                    data-label=(said(&c.name, &c.who))
                                    checked=(Some(c.key()) == pick)
                                    disabled=(!c.signed_in)
                                >
                                <span class="sk">
                                    (if c.who.is_empty() {
                                        c.name.clone()
                                    } else {
                                        c.who.clone()
                                    })
                                </span>
                            </label>
                            if !c.signed_in {
                                <span class="sv">"signed out"</span>
                                <button
                                    type="button"
                                    class="si"
                                    data-provider=(c.name.clone())
                                    data-account=(c.account.clone())
                                    data-pick=(c.key())
                                >
                                    "sign in"
                                </button>
                            }
                            <i class="tk" aria-hidden="true">"✓"</i>
                        </div>
                    }
                }
                for (p, cmd) in panels {
                    login_panel(provider: p, command: cmd)
                }
                <p class="lft">
                    <a href="#settings/providers">
                        "add an account or a provider in settings ›"
                    </a>
                </p>
            </fieldset>
        </div>
    })
}

/// The folder sheet: search and [+ new folder], then the recent folders (each with the pick last used there) and
/// the roots and one level in (two levels when searching), the first checked.
#[component]
async fn folder_sheet(spots: Vec<Spot>, recent: usize) -> Result<impl View> {
    Ok(view! {
        <div
            class="sh pop"
            id="sh-folder"
            popover="auto"
            role="dialog"
            aria-labelledby="sht-folder"
        >
            pop_head(which: "folder")
            <div class="top">
                <div class="fq">
                    <input
                        id="fq"
                        type="search"
                        aria-label="search folders"
                        placeholder="search folders"
                        autocomplete="off"
                    >
                    <button type="button" class="sec" id="mkdir">"+ new folder"</button>
                </div>
                <fieldset class="fl">
                    <legend class="vh">"folder"</legend>
                    for (i, s) in spots.into_iter().enumerate() {
                        if i == 0 && recent > 0 {
                            <p class="ch">"recent"</p>
                        }
                        if i == recent {
                            <p class="ch">"all folders"</p>
                        }
                        <label
                            class="sr pk"
                            data-d=(s.depth.to_string())
                            hidden=(s.depth > 1)
                        >
                            <input
                                type="radio"
                                name="folder"
                                value=(s.path.display().to_string())
                                data-last=(s.last.unwrap_or_default())
                                checked=(i == 0)
                            >
                            <span class="sk">(short(&s.path))</span>
                            if !s.said.is_empty() {
                                <span class="sv">(s.said)</span>
                            }
                            <i class="tk" aria-hidden="true">"✓"</i>
                        </label>
                    }
                </fieldset>
            </div>
        </div>
    })
}

/// The sidebar (from 1024 px): [+ new thread] (`cur` `new`: the composer's own page), then a folder per project,
/// the folders by their newest thread, each foldable: its rows (the state dot, the title, the age), the open
/// thread's (`cur`) marked; under the device's chip (#33), as the list is.
#[component]
pub async fn side(cx: &Cx, rows: Vec<Summary>, cur: String) -> Result<impl View> {
    let list = crate::chip(cx);
    let mut groups: Vec<(PathBuf, Vec<Summary>)> = Vec::new();
    for r in rows {
        if (list == "chats" && r.workflow.is_some()) || (list == "workflow" && r.workflow.is_none())
        {
            continue;
        }
        match groups.iter_mut().find(|(f, _)| *f == r.folder) {
            Some((_, list)) => list.push(r),
            None => groups.push((r.folder.clone(), vec![r])),
        }
    }
    Ok(view! {
        <aside class="side" aria-label="threads">
            <a
                class="nw"
                href="/new"
                aria-current=(if cur == "new" { Some("page") } else { None })
            >
                "+ new thread"
            </a>
            for (folder, list) in groups {
                <details class="fg" open=(true) data-f=(folder.display().to_string())>
                    <summary title=(short(&folder))>
                        <b>(short(&folder))</b>
                        <span class="ds">(list.len().to_string())</span>
                    </summary>
                    for r in list {
                        <a
                            class=(if r.unread { "sb" } else { "sb rd" })
                            href=(format!("/t/{}", r.id))
                            aria-current=(if r.id == cur { Some("page") } else { None })
                        >
                            <i class="sd" data-sb=(state(&r.state))></i>
                            <span class="tt">(r.title.clone())</span>
                            <span class="ta">(age(r.at))</span>
                        </a>
                    }
                </details>
            }
        </aside>
    })
}

/// One account in Settings › providers: `[provider] <who> ✓ signed in [sign out]`, or `signed out [sign in]`.
#[component]
async fn account_row(provider: String, a: Account, s: Status) -> Result<impl View> {
    Ok(view! {
        <div class="acr" data-provider=(provider.clone()) data-account=(a.name.clone())>
            <span class="ptg">(provider.clone())</span>
            <span class="sk">(a.shown(&s))</span>
            if s.signed_in {
                <span class="st g">"✓ signed in"</span>
                <button type="button" class="sec sgo">"sign out"</button>
            } else {
                <span class="st m">"signed out"</span>
                if a.dir.is_some() {
                    <button type="button" class="sec sgo">"sign out"</button>
                }
                <button
                    type="button"
                    class="si"
                    data-provider=(provider)
                    data-account=(a.name.clone())
                >
                    "sign in"
                </button>
            }
        </div>
    })
}

/// Settings › providers (#48): one row per account, `[provider] <who> ✓ signed in [sign out]` or `signed out [sign
/// in]`; `[+ account]` per installed provider; one row per provider that isn't installed, `[copy install]`. A
/// sign-in's terminal opens under its row.
#[component]
pub async fn providers_sheet(providers: Vec<Sheet>) -> Result<impl View> {
    let (out, mut installed): (Vec<Sheet>, Vec<Sheet>) = providers
        .into_iter()
        .partition(|s| s.accounts.iter().any(|(_, st)| st.install.is_some()));
    installed.retain(|s| !s.accounts.is_empty());
    Ok(view! {
        <div
            class="sh set"
            id="sh-providers"
            popover="auto"
            role="dialog"
            aria-modal="true"
            aria-labelledby="sht-providers"
        >
            <div class="hd pvh">
                <button
                    type="button"
                    class="gl"
                    data-open="settings"
                    aria-label="back to settings"
                >
                    "‹"
                </button>
                <b id="sht-providers">"providers"</b>
                <button
                    type="button"
                    class="gl"
                    data-close=(true)
                    popovertarget="sh-providers"
                    popovertargetaction="hide"
                    aria-label="close"
                >
                    "×"
                </button>
            </div>
            <div class="top">
                for sheet in installed {
                    for (a, s) in sheet.accounts.clone() {
                        account_row(provider: sheet.provider.name.clone(), a: a, s: s)
                    }
                    if !sheet.provider.home.is_empty() {
                        <div
                            class="acr add"
                            data-provider=(sheet.provider.name.clone())
                        >
                            <span class="ptg">(sheet.provider.name.clone())</span>
                            <span class="sk"></span>
                            <button type="button" class="sec ad">"+ account"</button>
                        </div>
                    }
                    // How its last sign-in ended, when it didn't simply finish.
                    if let Some(why) = sheet
                        .why
                        .clone()
                        .filter(|w| w != "the login command finished") {
                        <p class="ds wy">(why)</p>
                    }
                    login_panel(
                        provider: sheet.provider.name.clone(),
                        command: sheet.provider.login.join(" ")
                    )
                }
                for sheet in out {
                    <div class="acr ni">
                        <span class="ptg">(sheet.provider.name.clone())</span>
                        <span class="sk">"not installed"</span>
                        if !sheet.provider.install.is_empty() {
                            <button
                                type="button"
                                class="sec"
                                data-copy=(sheet.provider.install.clone())
                                aria-label=(format!("copy {}", sheet.provider.install))
                            >
                                "copy install"
                            </button>
                        }
                    </div>
                }
            </div>
        </div>
    })
}
