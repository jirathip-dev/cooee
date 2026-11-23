use std::{collections::HashMap, sync::Arc, thread, time::Duration};

use blocks::{
    Accessory, Action, Block, Button, ButtonStyle, Confirm, Form, Label, List, Row, Screen,
    Section, Shown, Submit, Tap, Text, TextField, Time, Toast, Tone, Toned,
};
use cooee_core::shed_proto::{Account, AccountState, Window};
use cooee_machines::Machines;
use serde::Deserialize;
use serde_json::Value;
use topcoat::{
    Result,
    context::{Cx, app_context, request_context},
    router::{parse_query_params, path_param, response::Response, route},
};

use crate::{
    Hub, HubError, LoginView, Logins,
    http::{Answer, Name, Paired, blocking, manager, on_logins, refused},
    pairing, plugin,
};

const DROVER: Duration = Duration::from_secs(2);

fn list(id: &str, title: &str, rows: Vec<Row>) -> Block {
    let rows: Vec<Block> = rows.into_iter().map(|r| Block::Row(Box::new(r))).collect();
    let list = Block::List(Box::new(List::new(rows).id(id)));
    crate::http::core(Screen::default().id(id).title(title).blocks(vec![list]))
}

fn row(id: impl Into<String>, title: impl Into<String>, subtitle: impl Into<String>) -> Row {
    Row::new(title)
        .id(id)
        .subtitle(Toned::Text(subtitle.into()))
}

pub(super) async fn device_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let paired = pairing::devices(cx);
    let me = request_context::<Paired>(cx).0.id.clone();
    blocking(cx, move |_| {
        let list = paired.list().map_err(refused)?;
        let row = |d: cooee_core::Device| {
            let row = Row::new(d.label).id(d.id.clone());
            if d.id == me {
                return row.subtitle(Toned::Text("this device".to_owned()));
            }
            row
        };
        Ok(list.into_iter().map(row).collect())
    })
    .await
}

const SIGNINS: &str = "/screens/settings/signins";

const WARN_PCT: u32 = 80;

path_param!(machine);
path_param!(provider);

struct OnMachine {
    machine: String,
    local: bool,
    accounts: Option<Vec<Account>>,
}

async fn on_machines(cx: &Cx, only: Option<String>) -> Result<Vec<OnMachine>, HubError> {
    let hosts = Arc::clone(app_context::<Arc<Machines>>(cx));
    on_logins(cx, move |l| {
        let wanted = |m: &String| only.as_ref().is_none_or(|o| o == m);
        let here = hosts.local().to_owned();
        let mut all: Vec<OnMachine> = (wanted(&here).then(|| OnMachine {
            accounts: Some(local_accounts(l)),
            machine: here,
            local: true,
        }))
        .into_iter()
        .collect();
        let remote: Vec<String> = hosts.added().into_iter().filter(wanted).collect();
        let hosts = &hosts;
        thread::scope(|s| {
            let reads: Vec<_> = remote
                .into_iter()
                .map(|machine| {
                    s.spawn(move || {
                        let ls = hosts.on(&machine, &["account", "ls"]);
                        let accounts = ls.ok().and_then(|v| serde_json::from_value(v).ok());
                        OnMachine {
                            machine,
                            local: false,
                            accounts,
                        }
                    })
                })
                .collect();
            all.extend(reads.into_iter().filter_map(|r| r.join().ok()));
        });
        Ok(all)
    })
    .await
}

fn local_accounts(l: &Logins) -> Vec<Account> {
    l.sheets().into_iter().flat_map(|s| s.accounts).collect()
}

fn windows(a: &Account) -> &[Window] {
    a.usage.as_ref().map_or(&[], |u| &u.windows)
}

fn fullest(a: &Account) -> Option<&Window> {
    windows(a)
        .iter()
        .reduce(|top, w| if w.used_pct > top.used_pct { w } else { top })
}

fn title(a: &Account) -> String {
    let who = match (&a.who, a.name.as_str()) {
        (Some(who), _) => who.as_str(),
        (None, "default") => "main login",
        (None, name) => name,
    };
    format!("{} · {who}", a.provider)
}

fn time(since: Option<u64>, until: Option<u64>, label: &str) -> Shown {
    let iso = |at: u64| super::chat::iso(at.saturating_mul(1000));
    Shown::Time(Time {
        since: since.map(iso),
        until: until.map(iso),
        label: Some(label.to_owned()),
    })
}

fn state(a: &Account) -> Label {
    let (text, tone) = match a.state {
        AccountState::Ready => ("signed in", Tone::Ok),
        AccountState::Limited => ("at its limit", Tone::Warn),
        AccountState::SignedOut => ("signed out", Tone::Bad),
    };
    let resets = a
        .limited_until
        .or_else(|| fullest(a).and_then(|w| w.resets_at));
    let text = match resets {
        Some(at) if a.state == AccountState::Limited => {
            time(None, Some(at), "at its limit · resets in {rel}")
        }
        _ => text.into(),
    };
    Label {
        text,
        tone: Some(tone),
    }
}

fn account_row(machine: &str, a: &Account) -> Row {
    let id = format!("{machine}/{}/{}", a.provider, a.name);
    let hot = fullest(a).filter(|w| w.used_pct >= WARN_PCT);
    let figure = |w: &Window| format!("{} {}%", w.id, w.used_pct);
    let rest: Vec<String> = windows(a)
        .iter()
        .filter(|w| !hot.is_some_and(|h| std::ptr::eq(*w, h)))
        .map(figure)
        .collect();
    let mut row = Row::new(title(a))
        .id(id.clone())
        .subtitle(Toned::Label(state(a)))
        .accessory(Accessory::Chevron)
        .tap(Tap {
            sheet: Some(format!("{SIGNINS}/{id}")),
            ..Tap::default()
        });
    if let Some(h) = hot {
        row = row.badge(Label {
            text: figure(h).into(),
            tone: Some(Tone::Warn),
        });
    }
    if !rest.is_empty() {
        row = row.value(Shown::Text(rest.join(" · ")));
    }
    row
}

pub(super) async fn signin_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let here = Arc::clone(app_context::<Arc<Machines>>(cx))
        .local()
        .to_owned();
    on_logins(cx, move |l| {
        let all = local_accounts(l);
        Ok(all.iter().map(|a| account_row(&here, a)).collect())
    })
    .await
}

fn rows(id: impl Into<String>, rows: Vec<Row>) -> Block {
    let rows: Vec<Block> = rows.into_iter().map(|r| Block::Row(Box::new(r))).collect();
    Block::List(Box::new(List::new(rows).id(id)))
}

fn machine_section(s: &OnMachine) -> Block {
    let accounts = match &s.accounts {
        Some(all) => all.iter().map(|a| account_row(&s.machine, a)).collect(),
        None => vec![Row::new("offline").id(format!("{}/offline", s.machine))],
    };
    let header = if s.local {
        format!("{} · this machine", s.machine)
    } else {
        s.machine.clone()
    };
    let list = rows(format!("m/{}/accounts", s.machine), accounts);
    let section = Section::new(vec![list])
        .id(format!("m/{}", s.machine))
        .header(header);
    Block::Section(Box::new(section))
}

fn usage(a: &Account) -> Vec<Block> {
    let Some(u) = &a.usage else {
        return Vec::new();
    };
    let window = |w: &Window| {
        let tone = (w.used_pct >= WARN_PCT).then_some(Tone::Warn);
        let row = Row::new(w.id.clone())
            .id(format!("w/{}", w.id))
            .badge(Label {
                text: format!("{}%", w.used_pct).into(),
                tone,
            });
        match w.resets_at {
            Some(at) => row.subtitle(Toned::Label(Label {
                text: time(None, Some(at), "resets in {rel}"),
                tone: None,
            })),
            None => row,
        }
    };
    let read = Row::new("Read by shed")
        .id("read")
        .value(time(Some(u.read_at), None, "{} ago"));
    if u.windows.is_empty() {
        return vec![rows("read", vec![read])];
    }
    let mut all: Vec<Row> = u.windows.iter().map(window).collect();
    all.push(read);
    let section = Section::new(vec![rows("windows", all)])
        .id("usage")
        .header("Usage");
    vec![Block::Section(Box::new(section))]
}

fn detail(machine: &str, a: &Account) -> Block {
    let name = title(a);
    let who = rows(
        "who",
        vec![
            Row::new("State").id("state").badge(state(a)),
            Row::new("Machine")
                .id("machine")
                .value(Shown::Text(machine.to_owned())),
        ],
    );
    let values = crate::http::fields(serde_json::json!({
        "machine": machine, "provider": a.provider, "account": a.name,
    }));
    let again = Button::new("Sign in again")
        .id("again")
        .style(ButtonStyle::Plain)
        .action("signin.again")
        .values(values.clone());
    let out = Button::new("Sign out")
        .id("out")
        .style(ButtonStyle::Destructive)
        .action("signin.out")
        .values(values)
        .confirm(Confirm {
            title: format!("Sign out {name} on {machine}?"),
            message: Some(format!(
                "shed removes this sign-in from {machine}. To use it there again, sign in again."
            )),
            ok: Some("Sign out".to_owned()),
            destructive: Some(true),
        });
    let acts = Section::new(vec![
        Block::Button(Box::new(again)),
        Block::Button(Box::new(out)),
    ])
    .id("acts");
    let mut blocks = vec![who];
    blocks.extend(usage(a));
    blocks.push(Block::Section(Box::new(acts)));
    crate::http::core(
        Screen::default()
            .id("account")
            .title(name)
            .back("Sign-ins")
            .blocks(blocks),
    )
}

pub(super) async fn machine_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let all = Arc::clone(app_context::<Arc<Machines>>(cx));
    blocking(cx, move |hub| {
        let cards = all.poll(cooee_machines::unix_now());
        let mut tags = drover_tags(hub);
        let row = |c: cooee_machines::Card| {
            let state = if c.online { "online" } else { "offline" };
            let subtitle = match tags.remove(&c.name) {
                Some(t) => format!("{state} · {}", t.join(" · ")),
                None => state.to_owned(),
            };
            row(c.name.clone(), c.name, subtitle)
        };
        Ok(cards.into_iter().map(row).collect())
    })
    .await
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HostTab {
    hosts: Vec<HostCard>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HostCard {
    name: String,
    rows: Vec<Cell>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Cell {
    k: String,
    notes: Vec<String>,
}

fn drover_tags(hub: &Hub) -> HashMap<String, Vec<String>> {
    let drover = hub
        .plugins()
        .into_iter()
        .find(|p| hub.plugin_of(&p.name) == "drover");
    let Some(reply) =
        drover.and_then(|p| plugin::call(&p, "GET", "/api/v1/host", &[], &[], DROVER).ok())
    else {
        return HashMap::new();
    };
    let ok = reply.status == 200;
    let tab: HostTab = reply
        .bytes()
        .ok()
        .filter(|_| ok)
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let tags = |h: HostCard| {
        let row = h.rows.into_iter().find(|r| r.k == "tags")?;
        let tags: Vec<String> = row
            .notes
            .iter()
            .flat_map(|n| n.split_whitespace())
            .map(str::to_owned)
            .collect();
        (!tags.is_empty()).then_some((h.name, tags))
    };
    tab.hosts.into_iter().filter_map(tags).collect()
}

pub(super) async fn plugin_rows(cx: &Cx) -> Result<Vec<Row>, HubError> {
    let m = manager(cx);
    blocking(cx, move |_| {
        let rows = m.list().map_err(refused)?.into_iter().map(|r| {
            let l = r.locked;
            row(l.name.clone(), l.name, l.version)
        });
        Ok(rows.collect())
    })
    .await
}

#[route(GET "/screens/settings/devices")]
pub async fn devices(cx: &Cx) -> Result<Answer<Block>> {
    let rows = device_rows(cx).await;
    Ok(Answer(rows.map(|r| list("devices", "Devices", r))))
}

#[route(GET "/screens/settings/signins")]
pub async fn signins(cx: &Cx) -> Result<Answer<Block>> {
    let all = on_machines(cx, None).await;
    Ok(Answer(all.map(|all| {
        let screen = Screen::default().id("signins").title("Sign-ins");
        crate::http::core(screen.blocks(all.iter().map(machine_section).collect::<Vec<_>>()))
    })))
}

#[route(GET "/screens/settings/signins/{machine}/{provider}/{name}")]
pub async fn signin(cx: &Cx) -> Result<Answer<Block>> {
    let machine = path_param::<Machine>(cx).to_owned();
    let (provider, name) = (path_param::<Provider>(cx), path_param::<Name>(cx));
    let found = on_machines(cx, Some(machine.clone()))
        .await
        .and_then(|all| {
            let accounts = all.into_iter().find_map(|s| s.accounts);
            let accounts = accounts.ok_or_else(|| refused(format!("{machine} doesn't answer")))?;
            let account = accounts
                .into_iter()
                .find(|a| a.provider == provider && a.name == name);
            account.ok_or_else(|| refused(format!("no account {provider}/{name} on {machine}")))
        });
    Ok(Answer(found.map(|a| detail(&machine, &a))))
}

#[derive(Deserialize, Default)]
struct OnTerminal {
    terminal: Option<String>,
}

#[route(GET "/screens/settings/signins/{machine}/{provider}/{name}/login")]
pub async fn signin_login(cx: &Cx) -> Result<Answer<Block>> {
    let machine = path_param::<Machine>(cx).to_owned();
    let provider = path_param::<Provider>(cx).to_owned();
    let name = path_param::<Name>(cx).to_owned();
    let q: Option<OnTerminal> = parse_query_params(cx).ok();
    let terminal = q.and_then(|q| q.terminal).unwrap_or_default();
    let hosts = Arc::clone(app_context::<Arc<Machines>>(cx));
    let shown = on_logins(cx, move |l| {
        let view = if machine == hosts.local() {
            l.view(&provider)?
        } else {
            remote_view(&hosts, &machine, &terminal)
        };
        let values = crate::http::fields(serde_json::json!({
            "machine": machine, "provider": provider, "account": name, "terminal": terminal,
        }));
        Ok(login(&format!("{provider} · {name}"), &view, values))
    })
    .await;
    Ok(Answer(shown))
}

fn remote_view(hosts: &Machines, machine: &str, terminal: &str) -> LoginView {
    let (state, screen, why) = match hosts.on(machine, &["read", "--screen", "--", terminal]) {
        Ok(read) => {
            let screen = read
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            ("running", screen, None)
        }
        Err(e) => ("ended", String::new(), Some(e)),
    };
    LoginView {
        state,
        links: cooee_core::links(&screen),
        screen,
        why,
        account: None,
    }
}

fn login(back: &str, v: &LoginView, values: blocks::Fields) -> Block {
    let text = |id: &str, md: String| Block::Text(Box::new(Text::default().id(id).markdown(md)));
    let blocks = if v.state == "running" {
        let mut blocks = vec![text("screen", format!("```\n{}\n```", v.screen))];
        blocks.extend(v.links.iter().enumerate().map(|(i, link)| {
            let open = Tap {
                url: Some(link.clone()),
                ..Tap::default()
            };
            Block::Button(Box::new(
                Button::new(link.clone()).id(format!("link/{i}")).tap(open),
            ))
        }));
        let code = TextField::new("code").label("Code from the sign-in page");
        let form = Form::new(vec![code.into()]).id("code").submit(Submit {
            label: "Send".into(),
            action: "signin.code".into(),
            values: Some(values.clone()),
            ..Submit::default()
        });
        blocks.push(Block::Form(Box::new(form)));
        let cancel = Button::new("Cancel")
            .id("cancel")
            .action("signin.cancel")
            .values(values);
        blocks.push(Block::Button(Box::new(cancel)));
        blocks
    } else {
        let why = v
            .why
            .clone()
            .unwrap_or_else(|| "No sign-in is running.".to_owned());
        vec![text("why", why)]
    };
    crate::http::core(
        Screen::default()
            .id("login")
            .title("Sign in")
            .back(back)
            .refresh(true)
            .blocks(blocks),
    )
}

fn answer(toast: Option<String>, open: Option<String>, close: bool) -> blocks::Answer {
    blocks::Answer {
        toast: toast.map(Toast::new),
        open,
        close,
        ..blocks::Answer::default()
    }
}

pub(super) async fn act(cx: &Cx, a: &Action) -> Option<Result<Response>> {
    a.screen.strip_prefix(SIGNINS)?;
    let value = |k: &str| {
        a.values
            .get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let (machine, provider, name) = (value("machine"), value("provider"), value("account"));
    let (terminal, code) = (value("terminal"), value("code"));
    let (action, hosts) = (
        a.action.clone(),
        Arc::clone(app_context::<Arc<Machines>>(cx)),
    );
    let done = on_logins(cx, move |l| {
        let here = machine == hosts.local();
        let on = |args: &[&str]| hosts.on(&machine, args).map_err(refused);
        let login = format!("{SIGNINS}/{machine}/{provider}/{name}/login");
        let (sent, closed) = (
            || answer(Some("Code sent".to_owned()), None, false),
            || answer(None, None, true),
        );
        match (action.as_str(), here) {
            ("signin.again", true) => l
                .start(&provider, Some(&name), false)
                .map(|()| answer(None, Some(login), false)),
            ("signin.again", false) => {
                let started = on(&["account", "login", &provider, &name])?;
                let t = started
                    .get("terminal")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                Ok(answer(None, Some(format!("{login}?terminal={t}")), false))
            }
            ("signin.code", true) => l.send(&provider, &code).map(|()| sent()),
            ("signin.code", false) => {
                on(&["send", "--no-paste", "--", &terminal, code.trim()]).map(|_| sent())
            }
            ("signin.cancel", true) => l.cancel(&provider).map(|()| closed()),
            ("signin.cancel", false) => on(&["rm", "--", &terminal]).map(|_| closed()),
            ("signin.out", true) => l.sign_out(&provider, &name).map(|()| closed()),
            ("signin.out", false) => on(&["account", "rm", &provider, &name]).map(|_| closed()),
            (other, _) => Err(refused(format!("no action {other}"))),
        }
    })
    .await;
    Some(super::chat::answer(done))
}

#[route(GET "/screens/settings/machines")]
pub async fn machines(cx: &Cx) -> Result<Answer<Block>> {
    let rows = machine_rows(cx).await;
    Ok(Answer(rows.map(|r| list("machines", "Machines", r))))
}

#[route(GET "/screens/settings/plugins")]
pub async fn plugins(cx: &Cx) -> Result<Answer<Block>> {
    let rows = plugin_rows(cx).await;
    Ok(Answer(rows.map(|r| list("plugins", "Plugins", r))))
}

pub(super) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(devices)
        .route(signins)
        .route(signin)
        .route(signin_login)
        .route(machines)
        .route(plugins)
}
