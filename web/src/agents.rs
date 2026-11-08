//! Chat-first (#86, docs/design/chat-first): the sidebar is your agents, each a profile (#40) or Plain on an
//! installed ACP agent, with a name, an avatar and one ongoing chat, its side threads under it; a new chat needs
//! nothing picked (it runs in a scratch folder of its own) and "Use a folder…" attaches a root's folder at any time.
//! From 1000 px the sidebar sits beside the pane; on a phone it is the home screen and a chat its own screen.

use std::path::{Path, PathBuf};

use cooee_core::{AGENTS_FIX, Agent, Profile, ROOTS_FIX, Sheet, Summary, Thread, Turn};
use topcoat::{
    Result,
    context::Cx,
    view::{Unescaped, View, component, view},
};

use super::{age, document, header, hint, named, pick, short, shown, state};

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
        for r in self
            .rows
            .iter()
            .filter(|r| r.workflow.is_none() && !r.archived)
        {
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

/// How many side threads show under the agent on show; the older fold.
const SIDE: usize = 5;

/// A side thread's row: its title, age and unread count (#43).
#[component]
async fn side_row(c: Summary, open: String) -> Result<impl View> {
    Ok(view! {
        <a
            class=(match (c.id == open, c.unread) {
                (true, _) => "str on",
                (false, true) => "str ud",
                _ => "str",
            })
            href=(format!("/t/{}", c.id))
            data-id=(c.id.clone())
        >
            <span class="tt">(c.title)</span>
            <span class="ta">
                (age(c.at))
                badge(n: c.unseen)
            </span>
        </a>
    })
}

/// The count of messages not seen yet (#43), when there are any.
#[component]
async fn badge(n: usize) -> Result<impl View> {
    Ok(view! {
        if n > 0 {
            <b class="bd" aria-label=(format!("{n} unread"))>(n.to_string())</b>
        }
    })
}

/// Where a tap on an agent goes: its ongoing chat, else a new one.
fn href(m: &Mate) -> String {
    m.chats
        .first()
        .map_or(format!("/new?{}", m.key), |c| format!("/t/{}", c.id))
}

/// The sidebar: each agent (its avatar, name, age, state and last line; the current one with its side threads and
/// `+ new chat`), then `+ new agent`. `current` is the agent on show, `open` the chat; `fresh`: a new chat with it,
/// marked as its own `New chat · now` row.
#[component]
async fn sidebar(
    cx: &Cx,
    mates: Vec<Mate>,
    profiles: Vec<Profile>,
    archived: Vec<Summary>,
    current: String,
    open: String,
    fresh: bool,
) -> Result<impl View> {
    let look = shown(cx);
    // A new agent needs one to run: none installed, none to add.
    let can_add = !mates.is_empty();
    // An agent's row says its latest: its newest chat's state, line and age; bold while any chat is unread.
    let rows: Vec<(usize, Mate, Option<Summary>, bool, bool)> = mates
        .into_iter()
        .enumerate()
        .map(|(n, m)| {
            let latest = m.chats.iter().max_by_key(|c| c.at).cloned();
            let unread = m.chats.iter().any(|c| c.unread);
            let on = m.key == current;
            (n, m, latest, unread, on)
        })
        .collect();
    Ok(view! {
        <nav class="ags" aria-label="agents">
            if look.scene() {
                herd(rows: rows, open: open.clone())
            } else {
                <p class="lb">"agents"</p>
                for (n, m, latest, unread, on) in rows {
                    <a
                        class=(match (on && !fresh, unread) {
                            (true, _) => "agr on",
                            (false, true) => "agr ud",
                            _ => "agr",
                        })
                        href=(href(&m))
                        data-id=(m.chats.first().map(|c| c.id.clone()))
                        aria-current=((on
                                && m.chats.first().is_some_and(|c| c.id == open)).then(
                            || "page".to_owned(),
                        ))
                    >
                        avatar(name: m.name.clone(), n: n, big: false)
                        <span class="tt">(m.name.clone())</span>
                        <span class="ta">
                            (latest.as_ref().map(|c| age(c.at)).unwrap_or_default())
                            badge(n: m.chats.iter().map(|c| c.unseen).sum())
                        </span>
                        <span class="tp">
                            <b
                                data-st=(state(
                                    latest.as_ref().map_or("", |c| c.state.as_str()),
                                ))
                            >
                                (Unescaped::new_unchecked(
                                    look.st(
                                        state(latest.as_ref().map_or("", |c| c.state.as_str())),
                                        false,
                                    ),
                                ))
                            </b>
                            (latest.map(|c| c.last).unwrap_or_default())
                        </span>
                    </a>
                    if on {
                        <div class="sth">
                            for c in m.chats.iter().skip(1).take(SIDE).cloned() {
                                side_row(c: c, open: open.clone())
                            }
                            if m.chats.len() > SIDE + 1 {
                                <details>
                                    <summary>
                                        (format!("{} older chats", m.chats.len() - SIDE - 1))
                                    </summary>
                                    for c in m.chats.iter().skip(SIDE + 1).cloned() {
                                        side_row(c: c, open: open.clone())
                                    }
                                </details>
                            }
                            if fresh {
                                <a
                                    class="str on"
                                    href=(format!("/new?{}", m.key))
                                    aria-current="page"
                                >
                                    <span class="tt">"New chat"</span>
                                    <span class="ta">"now"</span>
                                </a>
                            }
                            <a class="str nw" href=(format!("/new?{}", m.key))>
                                "+ new chat"
                            </a>
                        </div>
                    }
                }
            }
            super::shelf(rows: archived)
            if can_add {
                new_agent(profiles: profiles)
            }
        </nav>
    })
}

/// `+ new agent`: a name and a profile (Plain by default), posted to `/agents`.
#[component]
async fn new_agent(profiles: Vec<Profile>) -> Result<impl View> {
    Ok(view! {
        <details class="nwa">
            <summary>"+ new agent"</summary>
            <form method="post" action="/agents">
                <input
                    name="name"
                    aria-label="name"
                    placeholder="name"
                    required=(true)
                    autocomplete="off"
                >
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
    })
}

/// A state's pose in Scene (#34): its 132 × 100 sprite.
fn pose(st: &str) -> &'static str {
    match st {
        "working" => "edit",
        "needs" => "needs",
        "done" => "done",
        "fail" => "fail",
        "ended" => "merged",
        _ => "idle",
    }
}

/// The sidebar in Scene (#34, design §1): one page of the theme's scene, each agent a sprite acting out its ongoing
/// chat's state over its name card (name, `profile · age`, the state's word), the current one ringed; then the
/// current agent's chats as chips. `chat.ts` switches the strip to night by the device's clock.
#[component]
async fn herd(
    cx: &Cx,
    rows: Vec<(usize, Mate, Option<Summary>, bool, bool)>,
    open: String,
) -> Result<impl View> {
    let look = shown(cx);
    let art = format!("/look/{}/scene.svg", look.name());
    let count = format!("{} agents", rows.len());
    let current = rows.iter().find(|r| r.4).map(|r| r.1.clone());
    let sprites: Vec<(Mate, String, String, String, String, bool)> = rows
        .into_iter()
        .map(|(_, m, latest, unread, on)| {
            let st = state(m.chats.first().map_or("", |c| c.state.as_str()));
            let key = if st.is_empty() { "idle" } else { st };
            let meta = match latest {
                Some(c) => format!("{} · {}", m.profile, age(c.at)),
                None => m.profile.clone(),
            };
            let class = if unread { "sb" } else { "sb rd" };
            (m, class.to_owned(), st.to_owned(), key.to_owned(), meta, on)
        })
        .collect();
    Ok(view! {
        <div class="scn" id="scene" data-art=(art.clone())>
            <section class="sp" aria-label="agents">
                <img class="sky" src=(art) alt="">
                <h2 class="fh">
                    <b>"agents"</b>
                    <span class="ds">(count)</span>
                </h2>
                <div class="fld">
                    for (m, class, st, key, meta, on) in sprites {
                        <a
                            class=(class)
                            href=(href(&m))
                            aria-current=(on.then(|| "true".to_owned()))
                        >
                            <span class="sf" data-s=(pose(&st))>
                                (Unescaped::new_unchecked(look.s(pose(&st))))
                            </span>
                            <span class="sn">
                                <b>(m.name.clone())</b>
                                <span>(meta)</span>
                                <span class=(format!("sw {st}")) data-w=(key.clone())>
                                    (Unescaped::new_unchecked(look.w(&key, false)))
                                </span>
                            </span>
                        </a>
                    }
                </div>
            </section>
        </div>
        if let Some(m) = current {
            <nav class="scc" aria-label=(format!("{}'s chats", m.name))>
                for (i, c) in m.chats.iter().enumerate() {
                    <a
                        class="chip"
                        href=(format!("/t/{}", c.id))
                        aria-current=((c.id == open).then(|| "page".to_owned()))
                    >
                        (if i == 0 { m.name.clone() } else { c.title.clone() })
                    </a>
                }
                <a class="chip" href=(format!("/new?{}", m.key))>"+ new chat"</a>
            </nav>
        }
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
    let name = |f: &Path| {
        f.file_name()
            .map_or(short(f), |n| n.to_string_lossy().into_owned())
    };
    let label = now.as_deref().map_or("scratch".to_owned(), name);
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
    let list_empty = list.is_empty();
    Ok(view! {
        <button type="button" class="fo" popovertarget="ufs">
            (format!("folder: {label} ▾"))
        </button>
        <div
            class="ufs"
            id="ufs"
            popover="auto"
            role="dialog"
            aria-label="Use a folder"
        >
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
                if list.is_empty() {
                    hint(
                        why: "no folders set · add",
                        fix: ROOTS_FIX,
                        then: "to config.toml"
                    )
                } else {
                    <input
                        id="fq"
                        type="search"
                        aria-label="search folders"
                        placeholder="search folders"
                        autocomplete="off"
                    >
                }
                <label>
                    <input
                        type="radio"
                        name="folder"
                        value=""
                        checked=(scratch)
                        disabled=(!scratch)
                    >
                    "scratch"
                    <span class="ds">"this chat's own"</span>
                </label>
                // A root heads its group (`~/Projects`); the folders in it show by name.
                for (f, depth, n) in list {
                    <label
                        data-d=(depth.to_string())
                        hidden=(depth > 1)
                        title=(short(&f))
                    >
                        <input
                            type="radio"
                            name="folder"
                            value=(f.display().to_string())
                            checked=(now.as_ref() == Some(&f))
                        >
                        (if depth == 0 { short(&f) } else { name(&f) })
                        <span class="ds">(n)</span>
                    </label>
                }
            </fieldset>
            <p class="ds">
                "Later turns run in the folder you pick. What the chat made in scratch stays there."
            </p>
            if !list_empty {
                <button type="button" id="mkdir">"+ new folder"</button>
            }
        </div>
    })
}

/// The folders a chat may use (with their depth), the threads (to count the chats in each), and the chat's folder
/// (none: its scratch folder).
pub(crate) type Places = (Vec<(PathBuf, u8)>, Vec<Summary>, Option<PathBuf>);

/// The composer's chips: the folder (`places`: the folders, the threads and the chat's folder; none for a
/// Workflow session, whose folder is its tool's), then the model and effort (`now` picked).
#[component]
pub(crate) async fn chips(
    places: Option<Places>,
    choices: (Vec<String>, Vec<String>),
    now: (String, String),
) -> Result<impl View> {
    Ok(view! {
        <div class="mc">
            if let Some((list, rows, folder)) = places {
                folder_chip(list: list, rows: rows, now: folder)
            }
            pick(id: "model".to_owned(), name: "model", list: choices.0, now: now.0)
            pick(id: "effort".to_owned(), name: "effort", list: choices.1, now: now.1)
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
            providers: providers.clone(),
            <div class="app" data-show=(show)>
                header(home: home, tabs: tabs, current: String::new(), new: new)
                <main class="cfm">
                    <div class="sdb">
                        sidebar(
                            mates: mates.clone(),
                            profiles: profiles,
                            archived: rows
                                .iter()
                                .filter(|r| r.archived && r.workflow.is_none())
                                .cloned()
                                .collect(),
                            current: if home { String::new() } else { current },
                            open: open.clone(),
                            fresh: matches!(pane, Pane::New { .. })
                        )
                    </div>
                    match pane {
                        Pane::Chat { thread, turn, profile, folder, choices } => {
                            super::chat(
                                thread: *thread,
                                turn: turn,
                                providers: &providers,
                                profile: profile,
                                mate: mate.map(
                                    |(n, m)| {
                                        let ongoing = m.chats.first().is_some_and(|c| c.id == open);
                                        (n, m.name, ongoing)
                                    },
                                ),
                                // Every thread's id, so a ref to one in the chat opens it (#84).
                                ids: rows.iter().map(|r| r.id.clone()).collect(),
                                places: (list, rows, folder),
                                choices: choices
                            )
                        }
                        _ => {
                            hello(
                                home: home,
                                mate: mate,
                                agents: agents,
                                providers: providers,
                                places: (list, rows)
                            )
                        }
                    }
                </main>
            </div>
        )
    })
}

/// A new chat's bar: `‹` back to the agents, the agent, `New chat`, its folder, and (not on the home page) `×`, back
/// to the agent's ongoing chat, else the agents.
#[component]
async fn bar(home: bool, n: usize, m: Mate) -> Result<impl View> {
    let close = m
        .chats
        .first()
        .map_or("/?list".to_owned(), |c| format!("/t/{}", c.id));
    Ok(view! {
        <div class="th">
            <a class="gl bk" href="/?list" aria-label="back to agents">"‹"</a>
            avatar(name: m.name.clone(), n: n, big: false)
            <div class="tb">
                // On the home page the header's "cooee" is the page's heading.
                if home {
                    <h2>"New chat"</h2>
                } else {
                    <h1>"New chat"</h1>
                }
                <span class="ds">
                    <i class="dt" data-sync="on"></i>
                    (format!("{} · {} · ", m.name, m.profile))
                    <span id="bf">"scratch"</span>
                </span>
            </div>
            if !home {
                <a class="gl" href=(close.clone()) aria-label="close the chat">"×"</a>
            }
        </div>
    })
}

/// A new chat with `mate`: its hello, then the composer with the folder chip and the model and effort chips. No
/// agent installed (or `shed agents` failing): what to run instead.
#[component]
async fn hello(
    home: bool,
    mate: Option<(usize, Mate)>,
    agents: Result<Vec<Agent>, String>,
    providers: Vec<Sheet>,
    places: (Vec<(PathBuf, u8)>, Vec<Summary>),
) -> Result<impl View> {
    let none = agents.err();
    let provider = mate
        .as_ref()
        .and_then(|(_, m)| providers.iter().find(|p| p.provider.agent == m.agent));
    let signed_out = provider.is_some_and(|p| {
        p.accounts
            .iter()
            .any(|(a, s)| a.name.is_empty() && !s.signed_in)
    });
    let choices = provider.map_or_else(Default::default, |p| {
        (p.provider.models.clone(), p.provider.efforts.clone())
    });
    Ok(view! {
        <div class="chat hip">
            match mate {
                None => {
                    if !home {
                        <h1 class="vh">"New chat"</h1>
                    }
                    <div class="tc">
                        match none {
                            Some(e) => {
                                <p class="err">(e)</p>
                            }
                            None => {
                                hint(
                                    why: "no agents installed · on the Mac:",
                                    fix: AGENTS_FIX,
                                    then: ""
                                )
                            }
                        }
                    </div>
                }
                Some((n, m)) => {
                    bar(home: home, n: n, m: m.clone())
                    <div class="tc">
                        <div class="hi">
                            avatar(name: m.name.clone(), n: n, big: true)
                            <b>(format!("New chat with {}", m.name))</b>
                            <p class="ds">
                                "It starts in a scratch folder of its own. Use a folder… whenever this should work in a repo."
                            </p>
                            if signed_out {
                                <button
                                    type="button"
                                    class="so"
                                    popovertarget="sh-providers"
                                >
                                    (format!("sign in {}", named(&m.agent, &providers)))
                                </button>
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
                        chips(
                            places: Some((places.0, places.1, None)),
                            choices: choices,
                            now: (String::new(), String::new())
                        )
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
        </div>
    })
}
