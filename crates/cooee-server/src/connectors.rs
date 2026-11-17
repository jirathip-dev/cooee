#[cfg(test)]
mod tests;
mod tokens;

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError},
};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD as B64};
use cooee_core::{
    agents::Agent,
    connectors::{self as core, Auth, Client as Kind, Endpoints, HttpsUrl, Name, Target},
    mcp::{Server, Source},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokens::{Client, Credential, Refused, Secret};
pub(crate) use tokens::{KEYCHAIN, Vault};

use crate::net::{Reply, fetch};

const TEN_MINUTES: u64 = 600_000;
const EARLY: u64 = 60_000;

#[derive(Serialize, Deserialize)]
struct File {
    format: u32,
    clients: Vec<Registration>,
    accounts: Vec<Account>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Registration {
    issuer: String,
    redirect: String,
    client_id: String,
    has_secret: bool,
}

#[derive(Clone, Serialize, Deserialize)]
struct Account {
    connector: String,
    name: String,
    #[serde(flatten)]
    state: State,
    issuer: String,
    client: String,
    scopes: Vec<String>,
    since: u64,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
enum State {
    SignedIn,
    NeedsSignIn { why: String },
}

struct Started {
    state: Secret,
    verifier: Secret,
    redirect: String,
    issuer: String,
    token: HttpsUrl,
    client: Registration,
    resource: String,
    scopes: Vec<String>,
}

enum Pending {
    OAuth(Box<Started>),
    Token,
    Taken,
}

struct Flow {
    id: String,
    connector: String,
    account: String,
    made: u64,
    ended: Option<Result<(), String>>,
    pending: Pending,
}

pub(crate) struct Connectors {
    data: PathBuf,
    vault: Vault,
    file: Mutex<()>,
    flows: Mutex<Vec<Flow>>,
    refreshing: Mutex<()>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn said(r: Refused) -> String {
    match r {
        Refused::Service(s) | Refused::Net(s) => s,
    }
}

fn random(n: usize) -> Result<String, String> {
    cooee_core::random(n).map(|b| B64.encode(b)).map_err(text)
}

fn target(s: &Server) -> Result<(HttpsUrl, Auth), String> {
    match s.source()? {
        Source::Catalogue(e) => match &e.target {
            Target::Url { url, .. } => Ok((url.clone(), e.auth.clone())),
            Target::Builtin(b) => match *b {},
        },
        Source::Typed { url, auth, .. } => Ok((url, auth)),
        Source::Command { .. } => Err(format!("{} runs on the Mac: it has no sign-in", s.name)),
    }
}

fn redirect(origin: &str, host: &str) -> Result<String, String> {
    let local = ["localhost", "127.0.0.1", "[::1]"]
        .iter()
        .any(|h| host == *h || host.starts_with(&format!("{h}:")));
    if origin == format!("https://{host}") || (local && origin == format!("http://{host}")) {
        Ok(format!("{origin}/connectors/callback"))
    } else {
        Err(
            "sign-in needs HTTPS: reach cooee through `tailscale serve`, at the Mac's tailnet name"
                .to_owned(),
        )
    }
}

fn first_ok(urls: &[String]) -> Option<Reply> {
    urls.iter()
        .find_map(|u| fetch("GET", u, &[], None).ok().filter(Reply::ok))
}

fn endpoints(issuer: &HttpsUrl) -> Result<Endpoints, String> {
    let doc = first_ok(&core::issuer_metadata(issuer))
        .ok_or_else(|| format!("{} has no sign-in metadata", issuer.as_str()))?;
    Endpoints::read(issuer, &doc.json())
}

fn discover(url: &HttpsUrl, pinned: Option<&HttpsUrl>) -> Result<(HttpsUrl, Endpoints), String> {
    let named = first_ok(&core::resource_metadata(url)).map(|r| core::issuer_of(&r.json()));
    let same = |a: &HttpsUrl, b: &HttpsUrl| {
        a.as_str().trim_end_matches('/') == b.as_str().trim_end_matches('/')
    };
    let issuer = match (named, pinned) {
        (Some(Ok(i)), Some(p)) if !same(&i, p) => {
            return Err(format!(
                "the server now names the sign-in server {}, not its reviewed {}: refused until its entry is reviewed again",
                i.as_str(),
                p.as_str()
            ));
        }
        (Some(Ok(i)), _) => i,
        (_, Some(p)) => p.clone(),
        (Some(Err(e)), None) => return Err(e),
        (None, None) => return Err("the server has no sign-in metadata".to_owned()),
    };
    let ends = endpoints(&issuer)?;
    Ok((issuer, ends))
}

impl Connectors {
    pub(crate) fn new(data: &Path, vault: Vault) -> Self {
        Self {
            data: data.to_owned(),
            vault,
            file: Mutex::new(()),
            flows: Mutex::new(Vec::new()),
            refreshing: Mutex::new(()),
        }
    }

    fn load(&self) -> Result<File, String> {
        match fs::read(self.data.join("connectors.json")) {
            Ok(b) => serde_json::from_slice(&b).map_err(text),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(File {
                format: 1,
                clients: Vec::new(),
                accounts: Vec::new(),
            }),
            Err(e) => Err(text(e)),
        }
    }

    fn change<T>(&self, f: impl FnOnce(&mut File) -> T) -> Result<T, String> {
        let _held = lock(&self.file);
        let mut file = self.load()?;
        let done = f(&mut file);
        let bytes = serde_json::to_vec_pretty(&file).map_err(text)?;
        cooee_core::write_private(&self.data.join("connectors.json"), &bytes).map_err(text)?;
        Ok(done)
    }

    fn client_of(&self, r: &Registration) -> Result<Client, String> {
        let secret = if r.has_secret {
            let name = tokens::client_item(&r.issuer, &r.redirect);
            (self.vault.get)(&self.data, &name).map_err(text)?
        } else {
            None
        };
        Ok(Client {
            id: r.client_id.clone(),
            secret: secret.map(Secret::new),
        })
    }

    fn registered(&self, issuer: &str, redirect: &str) -> Result<Option<Registration>, String> {
        let file = self.load()?;
        Ok(file
            .clients
            .into_iter()
            .find(|r| r.issuer == issuer && (r.redirect == redirect || r.redirect.is_empty())))
    }

    fn save_client(&self, r: Registration, secret: Option<&str>) -> Result<Registration, String> {
        if let Some(s) = secret {
            let name = tokens::client_item(&r.issuer, &r.redirect);
            (self.vault.set)(&self.data, &name, s).map_err(text)?;
        }
        let kept = r.clone();
        self.change(move |f| {
            f.clients
                .retain(|o| (&o.issuer, &o.redirect) != (&r.issuer, &r.redirect));
            f.clients.push(r);
        })?;
        Ok(kept)
    }

    fn register(
        &self,
        url: &HttpsUrl,
        issuer: &str,
        redirect: &str,
    ) -> Result<Registration, String> {
        let body = json!({
            "client_name": "cooee",
            "redirect_uris": [redirect],
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
        });
        let headers = ["Content-Type: application/json".to_owned()];
        let r = fetch("POST", url.as_str(), &headers, Some(&body.to_string()))?;
        let doc = r.json();
        let id = doc
            .get("client_id")
            .and_then(Value::as_str)
            .filter(|_| r.ok());
        let id = id.ok_or_else(|| {
            format!(
                "the sign-in server refused cooee's registration ({})",
                r.status
            )
        })?;
        let secret = doc.get("client_secret").and_then(Value::as_str);
        let reg = Registration {
            issuer: issuer.to_owned(),
            redirect: redirect.to_owned(),
            client_id: id.to_owned(),
            has_secret: secret.is_some(),
        };
        self.save_client(reg, secret)
    }

    pub(crate) fn add_client(
        &self,
        s: &Server,
        id: &str,
        secret: Option<&str>,
    ) -> Result<String, String> {
        let (url, auth) = target(s)?;
        let Auth::OAuth { issuer, .. } = auth else {
            return Err(format!("{} doesn't sign in with OAuth", s.name));
        };
        if id.is_empty() || id.contains(char::is_control) {
            return Err("a client id is one line".to_owned());
        }
        let (issuer, _) = discover(&url, issuer.as_ref())?;
        let reg = Registration {
            issuer: issuer.as_str().to_owned(),
            redirect: String::new(),
            client_id: id.to_owned(),
            has_secret: secret.is_some(),
        };
        self.save_client(reg, secret)?;
        Ok(format!("Saved the client for {}.", issuer.as_str()))
    }

    fn start(&self, flow: Flow, now: u64) {
        let mut flows = lock(&self.flows);
        flows.retain(|f| now.saturating_sub(f.made) <= 2 * TEN_MINUTES);
        flows.push(flow);
    }

    pub(crate) fn signin(
        &self,
        s: &Server,
        account: &str,
        (origin, host): (&str, &str),
        now: u64,
    ) -> Result<Value, String> {
        let account = Name::try_from(account)?.as_str().to_owned();
        let (url, auth) = target(s)?;
        let id = random(16)?;
        let flow = |pending| Flow {
            id: id.clone(),
            connector: s.name.clone(),
            account: account.clone(),
            made: now,
            ended: None,
            pending,
        };
        let (issuer, scopes, kind) = match auth {
            Auth::None => return Err(format!("{} needs no sign-in", s.name)),
            Auth::Token { help, .. } => {
                self.start(flow(Pending::Token), now);
                return Ok(json!({ "flow": id, "help": help }));
            }
            Auth::OAuth {
                issuer,
                scopes,
                client,
            } => (issuer, scopes, client),
        };
        let redirect = redirect(origin, host)?;
        let (issuer, ends) = discover(&url, issuer.as_ref())?;
        let client = match (
            self.registered(issuer.as_str(), &redirect)?,
            kind,
            &ends.register,
        ) {
            (Some(r), ..) => r,
            (None, Kind::Registered, Some(at)) => self.register(at, issuer.as_str(), &redirect)?,
            _ => {
                return Err(format!(
                    "{} needs a sign-in client made once on the service: add it under connectors › {}",
                    issuer.as_str(),
                    s.name
                ));
            }
        };
        let (verifier, challenge) = core::pkce(&cooee_core::random(32).map_err(text)?);
        let state = random(24)?;
        let open = core::authorize_url(
            &ends.authorize,
            (&client.client_id, &redirect),
            (&state, &challenge),
            &scopes,
            url.as_str(),
        );
        self.start(
            flow(Pending::OAuth(Box::new(Started {
                state: Secret::new(state),
                verifier: Secret::new(verifier),
                redirect,
                issuer: issuer.as_str().to_owned(),
                token: ends.token,
                client,
                resource: url.as_str().to_owned(),
                scopes,
            }))),
            now,
        );
        Ok(json!({ "flow": id, "open": open, "until": "cooee" }))
    }

    fn signed_in(
        &self,
        (connector, account): (&str, &str),
        c: &Credential,
        (issuer, client, scopes): (&str, &str, Vec<String>),
        now: u64,
    ) -> Result<(), String> {
        tokens::keep(self.vault, &self.data, &tokens::item(connector, account), c).map_err(text)?;
        let row = Account {
            connector: connector.to_owned(),
            name: account.to_owned(),
            state: State::SignedIn,
            issuer: issuer.to_owned(),
            client: client.to_owned(),
            scopes,
            since: now,
        };
        self.change(|f| {
            f.accounts
                .retain(|a| (a.connector.as_str(), a.name.as_str()) != (connector, account));
            f.accounts.push(row);
        })
    }

    pub(crate) fn paste(
        &self,
        connector: &str,
        flow: &str,
        value: &str,
        now: u64,
    ) -> Result<String, String> {
        if value.trim().is_empty() || value.contains(char::is_control) {
            return Err("a token is one line".to_owned());
        }
        let account = {
            let mut flows = lock(&self.flows);
            let f = flows.iter_mut().find(|f| {
                f.id == flow
                    && f.connector == connector
                    && f.ended.is_none()
                    && matches!(f.pending, Pending::Token)
            });
            let f = f.filter(|f| now.saturating_sub(f.made) <= TEN_MINUTES);
            let f = f.ok_or("no such sign-in in the last 10 minutes: start it again")?;
            f.ended = Some(Ok(()));
            f.account.clone()
        };
        let c = Credential::Token {
            value: Secret::new(value.trim().to_owned()),
        };
        let done = self.signed_in((connector, &account), &c, ("", "", Vec::new()), now);
        if let Err(e) = &done {
            self.end(flow, Err(e.clone()));
        }
        done.map(|()| "Saved in the Mac's keychain.".to_owned())
    }

    fn end(&self, id: &str, how: Result<(), String>) {
        if let Some(f) = lock(&self.flows).iter_mut().find(|f| f.id == id) {
            f.ended = Some(how);
        }
    }

    pub(crate) fn flow(&self, id: &str, now: u64) -> Option<Value> {
        let flows = lock(&self.flows);
        let f = flows.iter().find(|f| f.id == id)?;
        let (state, why) = match &f.ended {
            Some(Ok(())) => ("signed_in", None),
            Some(Err(why)) => ("failed", Some(why.as_str())),
            None if now.saturating_sub(f.made) > TEN_MINUTES => {
                ("failed", Some("the sign-in took over 10 minutes"))
            }
            None => ("waiting", None),
        };
        Some(json!({ "state": state, "connector": f.connector, "account": f.account, "why": why }))
    }

    pub(crate) fn callback(
        &self,
        state: &str,
        outcome: Result<&str, &str>,
        now: u64,
    ) -> Option<String> {
        let (id, connector, account, pending) = {
            let mut flows = lock(&self.flows);
            let f = flows.iter_mut().find(|f| {
                f.ended.is_none()
                    && matches!(&f.pending, Pending::OAuth(s) if s.state.expose() == state)
            })?;
            if now.saturating_sub(f.made) > TEN_MINUTES {
                f.ended = Some(Err("the sign-in took over 10 minutes".to_owned()));
                return None;
            }
            f.ended = Some(Err("signing in".to_owned()));
            let pending = std::mem::replace(&mut f.pending, Pending::Taken);
            (
                f.id.clone(),
                f.connector.clone(),
                f.account.clone(),
                pending,
            )
        };
        let Pending::OAuth(started) = pending else {
            return None;
        };
        let Started {
            verifier,
            redirect,
            issuer,
            token,
            client,
            resource,
            scopes,
            ..
        } = *started;
        let done = outcome
            .map_err(|e| format!("the service said: {e}"))
            .and_then(|code| {
                let c = self.client_of(&client)?;
                let cred =
                    tokens::exchange(&token, &c, (code, &verifier, &redirect, &resource), now)
                        .map_err(said)?;
                self.signed_in(
                    (&connector, &account),
                    &cred,
                    (&issuer, &client.client_id, scopes),
                    now,
                )
            });
        self.end(&id, done);
        Some(id)
    }

    fn account(&self, connector: &str, name: &str) -> Result<Option<Account>, String> {
        Ok(self
            .load()?
            .accounts
            .into_iter()
            .find(|a| a.connector == connector && a.name == name))
    }

    fn client_for(&self, a: &Account) -> Result<Client, String> {
        let file = self.load()?;
        let r = file
            .clients
            .iter()
            .find(|r| r.issuer == a.issuer && r.client_id == a.client);
        self.client_of(r.ok_or("its sign-in client is gone: sign in again")?)
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "#497 slice B (#521) sends each call with it")
    )]
    pub(crate) fn credential(
        &self,
        s: &Server,
        account: &str,
        now: u64,
    ) -> Result<(String, String), String> {
        let (url, auth) = target(s)?;
        let item = tokens::item(&s.name, account);
        let needs = || format!("{} ({account}) needs sign-in", s.name);
        let read = || {
            tokens::read(self.vault, &self.data, &item)
                .map_err(text)?
                .ok_or_else(needs)
        };
        let bearer = |a: &Secret| ("Authorization".to_owned(), format!("Bearer {}", a.expose()));
        match (read()?, auth) {
            (Credential::Token { value }, Auth::Token { header, prefix, .. }) => {
                return Ok((header, format!("{prefix}{}", value.expose())));
            }
            (
                Credential::OAuth {
                    access, expires, ..
                },
                _,
            ) if expires > now.saturating_add(EARLY) => {
                return Ok(bearer(&access));
            }
            (Credential::OAuth { .. }, _) => {}
            _ => return Err(needs()),
        }
        let _one = lock(&self.refreshing);
        let old = match read()? {
            Credential::OAuth {
                access, expires, ..
            } if expires > now.saturating_add(EARLY) => return Ok(bearer(&access)),
            Credential::OAuth {
                refresh: Some(old), ..
            } => old,
            _ => return Err(needs()),
        };
        let a = self.account(&s.name, account)?.ok_or_else(needs)?;
        let issuer = HttpsUrl::try_from(a.issuer.clone())?;
        let fresh = tokens::refresh(
            &endpoints(&issuer)?.token,
            &self.client_for(&a)?,
            (old, url.as_str()),
            now,
        );
        match fresh {
            Ok(c) => {
                tokens::keep(self.vault, &self.data, &item, &c).map_err(text)?;
                match c {
                    Credential::OAuth { access, .. } => Ok(bearer(&access)),
                    Credential::Token { .. } => Err(needs()),
                }
            }
            Err(Refused::Service(why)) => {
                self.change(|f| {
                    for a in f
                        .accounts
                        .iter_mut()
                        .filter(|a| a.connector == s.name && a.name == account)
                    {
                        a.state = State::NeedsSignIn { why: why.clone() };
                    }
                })?;
                Err(format!("{}: {why}", needs()))
            }
            Err(Refused::Net(e)) => Err(e),
        }
    }

    pub(crate) fn signout(&self, connector: &str, account: &str) -> Result<String, String> {
        let item = tokens::item(connector, account);
        let cred = tokens::read(self.vault, &self.data, &item).map_err(text)?;
        let row = self.account(connector, account)?;
        if cred.is_none() && row.is_none() {
            return Ok(format!("{connector} ({account}) isn't signed in."));
        }
        let revoked = match (&cred, &row) {
            (Some(c @ Credential::OAuth { .. }), Some(a)) => HttpsUrl::try_from(a.issuer.clone())
                .and_then(|i| endpoints(&i))
                .ok()
                .and_then(|e| e.revoke)
                .is_some_and(|at| {
                    self.client_for(a)
                        .is_ok_and(|cl| tokens::revoke(&at, &cl, c).is_ok())
                }),
            _ => false,
        };
        (self.vault.remove)(&self.data, &item).map_err(text)?;
        self.change(|f| {
            f.accounts
                .retain(|a| !(a.connector == connector && a.name == account));
        })?;
        Ok(if revoked {
            format!("Signed {connector} ({account}) out; the service revoked cooee's access.")
        } else {
            format!(
                "Signed {connector} ({account}) out. The service wasn't told: remove cooee in its own settings too."
            )
        })
    }

    pub(crate) fn forget(&self, connector: &str) -> Result<(), String> {
        let names: Vec<String> = self
            .load()?
            .accounts
            .into_iter()
            .filter(|a| a.connector == connector)
            .map(|a| a.name)
            .collect();
        for n in names {
            self.signout(connector, &n)?;
        }
        Ok(())
    }

    pub(crate) fn list(&self, servers: &[Server], personas: &[Agent]) -> Result<Value, String> {
        let file = self.load()?;
        let kind = |a: &Auth| match a {
            Auth::None => "none",
            Auth::Token { .. } => "token",
            Auth::OAuth { .. } => "oauth",
        };
        let catalogue: Vec<Value> = core::catalogue()
            .iter()
            .map(|e| json!({
                "name": e.name.as_str(), "label": e.label, "publisher": e.publisher, "auth": kind(&e.auth),
                "leaves": e.leaves,
                "tools": { "read": e.tools.read.len(), "write": e.tools.write.len(), "never": e.tools.never.len() },
            }))
            .collect();
        let mut connectors = Vec::new();
        for s in servers {
            let (label, publisher, reviewed, url, auth, leaves, tools) = match s.source() {
                Ok(Source::Catalogue(e)) => {
                    let url = match &e.target {
                        Target::Url { url, .. } => url.as_str().to_owned(),
                        Target::Builtin(b) => match *b {},
                    };
                    (
                        e.label.as_str(),
                        e.publisher.as_str(),
                        true,
                        url,
                        kind(&e.auth),
                        e.leaves.as_str(),
                        e.tools.clone(),
                    )
                }
                Ok(Source::Typed {
                    url, auth, read, ..
                }) => {
                    let tools = core::Tools {
                        read: read.to_vec(),
                        ..core::Tools::default()
                    };
                    (
                        s.name.as_str(),
                        "",
                        false,
                        url.as_str().to_owned(),
                        kind(&auth),
                        "",
                        tools,
                    )
                }
                Ok(Source::Command { .. }) | Err(_) => (
                    s.name.as_str(),
                    "this Mac",
                    false,
                    String::new(),
                    "none",
                    "",
                    core::Tools::default(),
                ),
            };
            let host = url
                .split_once("://")
                .map_or("", |(_, r)| r.split('/').next().unwrap_or_default());
            let accounts: Vec<Value> = file
                .accounts
                .iter()
                .filter(|a| a.connector == s.name)
                .map(|a| {
                    let (state, why) = match &a.state {
                        State::SignedIn => ("signed_in", None),
                        State::NeedsSignIn { why } => ("needs_sign_in", Some(why)),
                    };
                    json!({ "name": a.name, "state": state, "why": why, "scopes": a.scopes, "since": a.since })
                })
                .collect();
            let users: Vec<&str> = personas
                .iter()
                .filter(|p| p.mcp.iter().any(|m| m.server == s.name))
                .map(|p| p.name.as_str())
                .collect();
            connectors.push(json!({
                "name": s.name, "label": label, "publisher": publisher, "reviewed": reviewed, "host": host,
                "leaves": leaves, "auth": auth, "enabled": s.enabled,
                "tools": { "read": tools.read, "write": tools.write, "never": tools.never },
                "accounts": accounts, "personas": users,
            }));
        }
        Ok(json!({ "catalogue": catalogue, "connectors": connectors }))
    }
}
