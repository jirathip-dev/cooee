use std::{io, path::Path};

use cooee_core::connectors::{HttpsUrl, form};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::net::{Reply, fetch};

#[derive(Deserialize)]
#[serde(transparent)]
pub(super) struct Secret(String);

impl Secret {
    pub(super) const fn new(v: String) -> Self {
        Self(v)
    }

    pub(super) fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(super) enum Credential {
    Token {
        value: Secret,
    },
    #[serde(rename = "oauth")]
    OAuth {
        access: Secret,
        refresh: Option<Secret>,
        expires: u64,
    },
}

#[derive(Clone, Copy)]
pub(crate) struct Vault {
    pub(crate) get: fn(&Path, &str) -> io::Result<Option<String>>,
    pub(crate) set: fn(&Path, &str, &str) -> io::Result<()>,
    pub(crate) remove: fn(&Path, &str) -> io::Result<()>,
}

pub(crate) const KEYCHAIN: Vault = Vault {
    get: cooee_core::secrets::get,
    set: cooee_core::secrets::set,
    remove: cooee_core::secrets::remove,
};

pub(super) fn item(connector: &str, account: &str) -> String {
    format!("connector.{connector}.{account}")
}

pub(super) fn client_item(issuer: &str, redirect: &str) -> String {
    let hash = cooee_core::sha256(format!("{issuer} {redirect}").as_bytes());
    format!("connector-client.{}", hash.get(..16).unwrap_or_default())
}

pub(super) fn keep(v: Vault, data: &Path, name: &str, c: &Credential) -> io::Result<()> {
    let line = match c {
        Credential::Token { value } => json!({ "kind": "token", "value": value.expose() }),
        Credential::OAuth {
            access,
            refresh,
            expires,
        } => json!({
            "kind": "oauth",
            "access": access.expose(),
            "refresh": refresh.as_ref().map(Secret::expose),
            "expires": expires,
        }),
    };
    (v.set)(data, name, &line.to_string())
}

pub(super) fn read(v: Vault, data: &Path, name: &str) -> io::Result<Option<Credential>> {
    let Some(line) = (v.get)(data, name)? else {
        return Ok(None);
    };
    serde_json::from_str(&line)
        .map(Some)
        .map_err(|_| io::Error::other(format!("{name} isn't a credential")))
}

pub(super) struct Client {
    pub(super) id: String,
    pub(super) secret: Option<Secret>,
}

impl Client {
    fn pairs<'a>(&'a self, mut pairs: Vec<(&'a str, &'a str)>) -> String {
        pairs.push(("client_id", &self.id));
        if let Some(s) = &self.secret {
            pairs.push(("client_secret", s.expose()));
        }
        form(&pairs)
    }
}

pub(super) enum Refused {
    Service(String),
    Net(String),
}

fn post(url: &HttpsUrl, body: &str) -> Result<Reply, Refused> {
    let headers = [
        "Content-Type: application/x-www-form-urlencoded".to_owned(),
        "Accept: application/json".to_owned(),
    ];
    fetch("POST", url.as_str(), &headers, Some(body)).map_err(Refused::Net)
}

fn granted(r: &Reply, now: u64, old: Option<Secret>) -> Result<Credential, Refused> {
    let doc = r.json();
    let access = doc
        .get("access_token")
        .and_then(Value::as_str)
        .filter(|_| r.ok());
    let Some(access) = access else {
        let why = doc
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("no access_token");
        return Err(Refused::Service(format!(
            "the sign-in server refused ({}): {why}",
            r.status
        )));
    };
    let refresh = doc
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(|t| Secret(t.to_owned()));
    let expires = doc
        .get("expires_in")
        .and_then(Value::as_u64)
        .map_or(u64::MAX, |s| now.saturating_add(s.saturating_mul(1000)));
    Ok(Credential::OAuth {
        access: Secret(access.to_owned()),
        refresh: refresh.or(old),
        expires,
    })
}

pub(super) fn exchange(
    token: &HttpsUrl,
    client: &Client,
    (code, verifier, redirect, resource): (&str, &Secret, &str, &str),
    now: u64,
) -> Result<Credential, Refused> {
    let body = client.pairs(vec![
        ("grant_type", "authorization_code"),
        ("code", code),
        ("code_verifier", verifier.expose()),
        ("redirect_uri", redirect),
        ("resource", resource),
    ]);
    granted(&post(token, &body)?, now, None)
}

pub(super) fn refresh(
    token: &HttpsUrl,
    client: &Client,
    (old, resource): (Secret, &str),
    now: u64,
) -> Result<Credential, Refused> {
    let body = client.pairs(vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", old.expose()),
        ("resource", resource),
    ]);
    granted(&post(token, &body)?, now, Some(old))
}

pub(super) fn revoke(url: &HttpsUrl, client: &Client, c: &Credential) -> Result<(), Refused> {
    let Credential::OAuth {
        access, refresh, ..
    } = c
    else {
        return Ok(());
    };
    let (token, hint) = refresh
        .as_ref()
        .map_or((access, "access_token"), |r| (r, "refresh_token"));
    let body = client.pairs(vec![("token", token.expose()), ("token_type_hint", hint)]);
    let r = post(url, &body)?;
    if r.ok() {
        Ok(())
    } else {
        Err(Refused::Service(format!("revoke answered {}", r.status)))
    }
}
