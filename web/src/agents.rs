//! Chat-first (#86, docs/design/chat-first): the sidebar is your agents, each a profile (#40) or Plain on an
//! installed ACP agent, with a name, an avatar and one ongoing chat, its side threads under it; a new chat needs
//! nothing picked (it runs in a scratch folder of its own) and "Use a folder…" attaches a root's folder at any time.
//! From 1000 px the sidebar sits beside the pane; on a phone it is the home screen and a chat its own screen.

use std::path::PathBuf;

use cooee_core::{AGENTS_FIX, Agent, Profile, ROOTS_FIX, Sheet, Summary, Thread, Turn};
use topcoat::{
    Result,
    context::Cx,
    view::{Unescaped, View, component, view},
};

use super::{age, document, header, hint, pick, short, shown, signins, state};

/// One agent in the sidebar.
#[derive(Debug, Clone)]
pub struct Mate {
    /// What `/new` starts its chat with: `profile=<id>` or `agent=<ACP id>` (Plain).
    pub key: String,
    /// Its name.
    pub name: String,
    /// Its profile's id, or `Plain`.
    pub profile: String,
    /// The ACP agent a Plain chat runs.
    pub agent: String,
    /// Its chats: the ongoing one (its oldest open, else its newest) first, then its side threads, newest first.
    pub chats: Vec<Summary>,
}

/// What the sidebar and a new chat are drawn from: the threads (newest first), the profiles, the installed agents,
/// the providers (names, sign-ins, models), the folders a chat may use, and the plugin tabs.
pub struct Side {
    /// `Hub::list`.
    pub rows: Vec<Summary>,
    /// The profiles to pick from.
    pub profiles: Vec<Profile>,
    /// `shed agents`, or why it failed.
    pub agents: Result<Vec<Agent>, String>,
    /// Each provider's sheet.
    pub providers: Vec<Sheet>,
    /// The roots and the folders inside them, with their depth.
    pub folders: Vec<(PathBuf, u8)>,
    /// The plugin tabs: name, label, dot.
    pub tabs: Vec<(String, String, bool)>,
}

impl Side {
    /// The agents: Plain on each installed agent (named after its provider), then each profile; a thread whose
    /// agent or profile is gone keeps one of its own. Workflow sessions (#33) are drover's, never listed here.
    #[must_use]
    pub fn mates(&self) -> Vec<Mate> {
        let installed = self.agents.as_ref().map(Vec::as_slice).unwrap_or_default();
        let plain = installed.iter().map(|a| {
            let provider = self.providers.iter().find(|p| p.provider.agent == a.id);
            let name = provider.map_or(a.id.clone(), |p| {
                let mut c = p.provider.name.chars();
                c.next()
                    .map_or(String::new(), |f| f.to_uppercase().chain(c).collect())
            });
            mate(format!("agent={}", a.id), name, "Plain", &a.id)
        });
        let profiles = self.profiles.iter().map(|p| {
            mate(
                format!("profile={}", p.id()),
                p.label.clone(),
                &p.id(),
                &p.agent,
            )
        });
        let mut mates: Vec<Mate> = plain.chain(profiles).collect();
        for r in self.rows.iter().filter(|r| r.workflow.is_none()) {
            let key = key(&r.agent, &r.profile);
            let at = if let Some(at) = mates.iter().position(|m| m.key == key) {
                at
            } else {
                let (name, profile) = if r.profile.is_empty() {
                    (r.agent.clone(), "Plain")
                } else {
                    (r.profile.clone(), r.profile.as_str())
                };
                mates.push(mate(key, name, profile, &r.agent));
                mates.len() - 1
            };
            mates[at].chats.push(r.clone());
        }
        for m in &mut mates {
            let ongoing = m
                .chats
                .iter()
                .rposition(|c| c.state != "ended")
                .unwrap_or(0);
            if !m.chats.is_empty() {
                let first = m.chats.remove(ongoing);
                m.chats.insert(0, first);
            }
        }
        mates
    }
}

fn mate(key: String, name: String, profile: &str, agent: &str) -> Mate {
    Mate {
        key,
        name,
        profile: profile.to_owned(),
        agent: agent.to_owned(),
        chats: Vec::new(),
    }
}

/// The agent a thread of `agent` and `profile` belongs to, as [`Mate::key`].
fn key(agent: &str, profile: &str) -> String {
    if profile.is_empty() {
        format!("agent={agent}")
    } else {
        format!("profile={profile}")
    }
}

/// A name's initials: the first letters of its first two words, else its first two letters.
fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let two: String = match words.as_slice() {
        [a, b, ..] => a.chars().take(1).chain(b.chars().take(1)).collect(),
        _ => name.chars().take(2).collect(),
    };
    two.to_uppercase()
}

/// An agent's avatar: its initials in the `n`th scheme colour.
#[component]
pub(crate) async fn avatar(name: String, n: usize, big: bool) -> Result<impl View> {
    let colour = ["--l", "--link", "--g", "--p"][n % 4];
    Ok(view! {
        <i
            class=(if big { "av big-av" } else { "av" })
            style=(format!("--av:var({colour})"))
            aria-hidden="true"
        >
            (initials(&name))
        </i>
    })
}

/// Where a tap on an agent goes: its ongoing chat, else a new one.
fn href(m: &Mate) -> String {
    m.chats
        .first()
        .map_or(format!("/new?{}", m.key), |c| format!("/t/{}", c.id))
}

/// The sidebar: each agent (its avatar, name, age, state and last line; the current one with its side threads and
/// `+ new chat`), then `+ new agent`. `current` is the agent on show, `open` the chat.
#[component]
async fn sidebar(
    cx: &Cx,
    mates: Vec<Mate>,
    profiles: Vec<Profile>,
    current: String,
    open: String,
) -> Result<impl View> {
    let look = shown(cx);
    // A new agent needs one to run: none installed, none to add.
    let can_add = !mates.is_empty();
    let rows: Vec<(usize, Mate, String, String, bool)> = mates
        .into_iter()
        .enumerate()
        .map(|(n, m)| {
            let first = m.chats.first();
            let st = state(first.map_or("", |c| c.state.as_str()));
            let mark = look.st(st, false);
            let on = m.key == current;
            (n, m, st.to_owned(), mark, on)
        })
        .collect();
    Ok(view! {
        <nav class="ags" aria-label="agents">
            <p class="lb">"agents"</p>
            for (n, m, st, mark, on) in rows {
                <a
                    class=(match (on, m.chats.first().is_some_and(|c| c.unread)) {
                        (true, _) => "agr on",
                        (false, true) => "agr ud",
                        _ => "agr",
                    })
                    href=(href(&m))
                    aria-current=((on && m.chats.first().is_some_and(|c| c.id == open))
                        .then(|| "page".to_owned()))
                >
                    avatar(name: m.name.clone(), n: n, big: false)
                    <span class="tt">(m.name.clone())</span>
                    <span class="ta">(m.chats.first().map(|c| age(c.at)).unwrap_or_default())</span>
                    <span class="tp">
                        <b data-st=(st)>(Unescaped::new_unchecked(mark))</b>
                        (m.chats.first().map(|c| c.last.clone()).unwrap_or_default())
                    </span>
                </a>
                if on {
                    <div class="sth">
                        for c in m.chats.iter().skip(1).cloned() {
                            <a
                                class=(if c.id == open { "str on" } else { "str" })
                                href=(format!("/t/{}", c.id))
                            >
                                <span class="tt">(c.title)</span>
                                <span class="ta">(age(c.at))</span>
                            </a>
                        }
                        <a class="str nw" href=(format!("/new?{}", m.key))>"+ new chat"</a>
                    </div>
                }
            }
            if can_add {
                <details class="nwa">
                    <summary>"+ new agent"</summary>
                    <form method="post" action="/agents">
                        <input name="name" aria-label="name" placeholder="name" required=(true) autocomplete="off">
                        <label class="pk">
                            <span class="lb">"profile"</span>
                            <select name="profile">
                                <option value="">"Plain"</option>
                                for p in profiles {
                                    <option value=(p.id())>(p.label.clone())</option>
                                }
                            </select>
                        </label>
                        <button type="submit">"add"</button>
                    </form>
                </details>
            }
        </nav>
    })
}

/// The folder chip (`folder: scratch ▾`) and its "Use a folder…" panel: `scratch` (this chat's own; on a chat that
/// already moved, no way back), then each root's folders with how many chats work there. A pick attaches it
/// (`chat.ts`); in a new chat it is the form's `folder`.
#[component]
pub(crate) async fn folder_chip(
    list: Vec<(PathBuf, u8)>,
    rows: Vec<Summary>,
    now: Option<PathBuf>,
) -> Result<impl View> {
    let label = now.as_deref().map_or("scratch".to_owned(), |f| {
        f.file_name()
            .map_or(short(f), |n| n.to_string_lossy().into_owned())
    });
    let count = |f: &PathBuf| {
        let f = f.canonicalize().unwrap_or_else(|_| f.clone());
        match rows.iter().filter(|r| r.folder == f).count() {
            0 => String::new(),
            1 => "1 chat".to_owned(),
            n => format!("{n} chats"),
        }
    };
    let list: Vec<(PathBuf, u8, String)> = list
        .iter()
        .map(|(f, d)| (f.clone(), *d, count(f)))
        .collect();
    let scratch = now.is_none();
    Ok(view! {
        <button type="button" class="fo" popovertarget="ufs">(format!("folder: {label} ▾"))</button>
        <div class="ufs" id="ufs" popover="auto" role="dialog" aria-label="Use a folder">
            <div class="fh">
                <b>"Use a folder…"</b>
                <button
                    type="button"
                    class="gl"
                    popovertarget="ufs"
                    popovertargetaction="hide"
                    aria-label="close"
                >
                    "×"
                </button>
            </div>
            <fieldset class="fl">
                <legend class="vh">"folder"</legend>
                <label data-d="0">
                    <input type="radio" name="folder" value="" checked=(scratch) disabled=(!scratch)>
                    "scratch"
                    <span class="ds">"this chat's own"</span>
                </label>
                if list.is_empty() {
                    hint(why: "no folders set · add", fix: ROOTS_FIX, then: "to config.toml")
                } else {
                    <input
                        id="fq"
                        type="search"
                        aria-label="search folders"
                        placeholder="search folders"
                        autocomplete="off"
                    >
                }
                for (f, depth, n) in list {
                    <label data-d=(depth.to_string()) hidden=(depth > 1)><input type="radio" name="folder" value=(f.display().to_string()) checked=(now.as_ref() == Some(&f))>(short(&f))<span class="ds">(n)</span></label>
                }
            </fieldset>
            <p class="ds">
                "Later turns run in the folder you pick. What the chat made in scratch stays there."
            </p>
        </div>
    })
}

/// The pages: the agents (the home screen), a new chat with one agent, or one chat.
pub enum Pane {
    /// `/`: the list, and from 1000 px a new chat with the first agent (on a phone too, before any chat).
    Home {
        /// Keep a phone on the list (`?list`, the `‹` of a chat).
        list: bool,
    },
    /// `/new?<key>`.
    New {
        /// The agent's [`Mate::key`].
        key: String,
    },
    /// `/t/{id}`.
    Chat {
        /// The thread.
        thread: Box<Thread>,
        /// Its turn.
        turn: Turn,
        /// Its profile's label.
        profile: String,
        /// Its folder (none: its scratch folder).
        folder: Option<PathBuf>,
        /// Its provider's models and efforts.
        choices: (Vec<String>, Vec<String>),
    },
}

/// A page of the app: the header, the sidebar and the pane (`chat.ts` drives a new chat's form and a chat).
#[must_use]
pub fn page(cx: &Cx, side: Side, pane: Pane) -> impl View {
    view! { cx => app(side: side, pane: pane) }
}

#[component]
async fn app(side: Side, pane: Pane) -> Result<impl View> {
    let mates = side.mates();
    let own = side.rows.iter().any(|r| r.workflow.is_none());
    let find = |key: &str| mates.iter().position(|m| m.key == key);
    let (at, show, title) = match &pane {
        Pane::Home { list } => (
            (!mates.is_empty()).then_some(0),
            if *list || own { "list" } else { "pane" },
            "cooee".to_owned(),
        ),
        Pane::New { key } => (find(key).or(Some(0)), "pane", "New chat".to_owned()),
        Pane::Chat { thread, .. } => (
            thread
                .workflow
                .is_none()
                .then(|| find(&key(&thread.agent, &thread.profile)))
                .flatten(),
            "pane",
            thread.title.clone(),
        ),
    };
    let at = at.filter(|&n| n < mates.len());
    let mate = at.map(|n| (n, mates[n].clone()));
    let current = mate
        .as_ref()
        .map(|(_, m)| m.key.clone())
        .unwrap_or_default();
    let open = match &pane {
        Pane::Chat { thread, .. } => thread.id.clone(),
        _ => String::new(),
    };
    let new = format!("/new?{current}");
    let Side {
        rows,
        profiles,
        agents,
        providers,
        folders: list,
        tabs,
    } = side;
    let home = matches!(pane, Pane::Home { .. });
    Ok(view! {
        document(
            title: title.as_str(),
            <div class="app">
                header(home: home, tabs: tabs, current: String::new(), new: new)
                <div class="cfm" data-show=(show)>
                    <div class="sd">
                        if home {
                            signins(providers: providers.clone())
                        }
                        sidebar(
                            mates: mates.clone(),
                            profiles: profiles,
                            current: if home { String::new() } else { current },
                            open: open.clone()
                        )
                    </div>
                    match pane {
                        Pane::Chat { thread, turn, profile, folder, choices } => {
                            super::chat(
                                thread: *thread,
                                turn: turn,
                                profile: profile,
                                mate: mate.map(|(n, m)| {
                                    let ongoing = m.chats.first().is_some_and(|c| c.id == open);
                                    (n, m.name, ongoing)
                                }),
                                places: (list, rows, folder),
                                choices: choices
                            )
                        }
                        _ => {
                            hello(mate: mate, agents: agents, providers: providers, places: (list, rows))
                        }
                    }
                </div>
            </div>
        )
    })
}

/// A new chat with `mate`: its hello, then the composer with the folder chip and the model and effort chips. No
/// agent installed (or `shed agents` failing): what to run instead.
#[component]
async fn hello(
    mate: Option<(usize, Mate)>,
    agents: Result<Vec<Agent>, String>,
    providers: Vec<Sheet>,
    places: (Vec<(PathBuf, u8)>, Vec<Summary>),
) -> Result<impl View> {
    let none = agents.err();
    let provider = mate
        .as_ref()
        .and_then(|(_, m)| providers.iter().find(|p| p.provider.agent == m.agent));
    let signed_out =
        provider.is_some_and(|p| p.accounts.iter().any(|(a, s)| a.default && !s.signed_in));
    let choices = provider.map_or_else(Default::default, |p| {
        (p.provider.models.clone(), p.provider.efforts.clone())
    });
    Ok(view! {
        <main class="chat hip">
            match mate {
                None => {
                    <div class="tc">
                        match none {
                            Some(e) => {
                                <p class="err">(e)</p>
                            }
                            None => {
                                hint(why: "no agents installed · on the Mac:", fix: AGENTS_FIX, then: "")
                            }
                        }
                    </div>
                }
                Some((n, m)) => {
                    <div class="th">
                        <a class="gl bk" href="/?list" aria-label="back to agents">"‹"</a>
                        avatar(name: m.name.clone(), n: n, big: false)
                        <div class="tb">
                            <h1>"New chat"</h1>
                            <span class="ds">
                                <i class="dt" data-sync="on"></i>
                                (format!("{} · {} · scratch", m.name, m.profile))
                            </span>
                        </div>
                    </div>
                    <div class="tc">
                        <div class="hi">
                            avatar(name: m.name.clone(), n: n, big: true)
                            <b>(format!("New chat with {}", m.name))</b>
                            <p class="ds">
                                "It starts in a scratch folder of its own. Use a folder… whenever this should work in a repo."
                            </p>
                            if signed_out {
                                <a class="so" href="/#signins">(format!("sign in {}", m.agent))</a>
                            }
                        </div>
                    </div>
                    <form id="new">
                        match m.key.split_once('=') {
                            Some(("profile", id)) => {
                                <input type="hidden" name="profile" value=(id.to_owned())>
                            }
                            _ => {
                                <input type="hidden" name="agent" value=(m.agent.clone())>
                            }
                        }
                        <div class="mc">
                            folder_chip(list: places.0, rows: places.1, now: None)
                            pick(id: "model".to_owned(), name: "model", list: choices.0, now: String::new())
                            pick(id: "effort".to_owned(), name: "effort", list: choices.1, now: String::new())
                        </div>
                        <div class="tw">
                            <textarea
                                name="text"
                                rows="1"
                                aria-label="message"
                                placeholder=(format!("message {}", m.name))
                            ></textarea>
                            <button type="submit">"send"</button>
                        </div>
                        <p class="err" id="new-err" hidden=(true)></p>
                    </form>
                }
            }
        </main>
    })
}
