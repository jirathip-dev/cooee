//! `cooee accounts ls [--json]`, `cooee accounts env <provider> <name> [--require-signed-in]`: the credential
//! helper herdsman calls for a lane's login environment (docs/accounts.md), and `cooee accounts add <provider> <name>
//! --folder <path>` / `rm <provider> <name>`, which adopt an existing config folder and drop one (#183). A folder account's environment is its
//! provider's variable and folder; its token stays the CLI's. A provider with a token variable may instead have
//! hand-placed token files, `<data>/accounts/<provider>/<name>.token` (0600 or tighter), whose value goes in it.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cooee_core::{Accounts, Config, Provider, Providers};
use serde_json::{Map, Value, json};

/// How an account logs in: a config folder (none: the system default) or a token file.
enum Login {
    Folder(Option<PathBuf>),
    Token(PathBuf, &'static str),
}

struct Entry<'a> {
    provider: &'a dyn Provider,
    name: String,
    login: Login,
}

impl Entry<'_> {
    /// Whether it is signed in (a folder: its provider's status command; a token file: a fit one), and why not.
    fn state(&self) -> (bool, Option<String>) {
        match &self.login {
            Login::Folder(dir) => (self.provider.check(dir.as_deref()).signed_in, None),
            Login::Token(path, _) => match token_age(path) {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e)),
            },
        }
    }
}

/// A token file's age in days, once it is a file no looser than 0600.
fn token_age(path: &Path) -> Result<u64, String> {
    let meta = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mode = meta.permissions().mode() & 0o777;
    if !meta.is_file() || mode & 0o077 != 0 {
        return Err(format!(
            "{}: a token file must be 0600 or tighter, not {mode:o}",
            path.display()
        ));
    }
    let age = meta
        .modified()
        .ok()
        .and_then(|m| SystemTime::now().duration_since(m).ok());
    Ok(age.map_or(0, |a| a.as_secs() / 86_400))
}

/// Every provider's accounts: its folder accounts, the system default first, then its token files (a token file
/// replaces a folder of the same name).
fn entries<'a>(providers: &'a Providers, data: &Path) -> Vec<Entry<'a>> {
    let accounts = Accounts::new(data);
    let mut all = Vec::new();
    for provider in providers.iter() {
        let provider = provider.as_ref();
        let name = &provider.spec().name;
        let mut list: Vec<Entry> = accounts
            .list(name)
            .into_iter()
            .map(|a| Entry {
                provider,
                name: a.name,
                login: Login::Folder(a.dir),
            })
            .collect();
        if let Some(var) = provider.token_var() {
            let dir = data.join("accounts").join(name);
            let mut tokens: Vec<(String, PathBuf)> = fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().into_string().ok()?;
                    let name = name.strip_suffix(".token")?.to_owned();
                    let safe = !name.is_empty()
                        && name
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-');
                    safe.then(|| (name, e.path()))
                })
                .collect();
            tokens.sort();
            for (name, path) in tokens {
                list.retain(|e| e.name != name);
                list.push(Entry {
                    provider,
                    name,
                    login: Login::Token(path, var),
                });
            }
        }
        all.extend(list);
    }
    all
}

/// Runs `cooee accounts …`: 0 done, 1 refused (stderr says why), 2 usage, 3 signed out with `--require-signed-in`.
pub fn run(config: &Config, args: &[&str], json: bool, require: bool) -> ExitCode {
    let refuse = |code: u8, why: &str| {
        eprintln!("cooee: {why}");
        ExitCode::from(code)
    };
    let Some(data) = config.data_dir() else {
        return refuse(1, "no data folder: set `data` or HOME");
    };
    let providers = Providers::new(&config.providers);
    let all = entries(&providers, &data);
    let accounts = Accounts::new(&data);
    match args {
        ["add", p, name, "--folder", folder] => {
            if providers.named(p).is_none_or(|p| p.spec().home.is_empty()) {
                return refuse(1, &format!("no provider `{p}` with account folders"));
            }
            match accounts.add(p, name, Path::new(folder)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => refuse(1, &e.to_string()),
            }
        }
        ["rm", p, name] => {
            if name.is_empty() || accounts.get(p, name).is_none() {
                return refuse(1, &format!("no account `{name}` of provider `{p}`"));
            }
            match accounts.remove(p, name) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => refuse(1, &e.to_string()),
            }
        }
        ["ls"] => {
            let rows: Vec<Value> = all.iter().map(|e| row(e, &accounts)).collect();
            if json {
                println!("{}", Value::Array(rows));
            } else {
                for r in rows {
                    let at = r["folder"].as_str().or(r["token_file"].as_str());
                    let state = if r["signed_in"] == true {
                        "signed in"
                    } else {
                        "signed out"
                    };
                    let name = r["name"].as_str().filter(|n| !n.is_empty());
                    println!(
                        "{}  {}  {state}  {}",
                        r["provider"].as_str().unwrap_or_default(),
                        name.unwrap_or("(system default)"),
                        at.unwrap_or("~")
                    );
                }
            }
            ExitCode::SUCCESS
        }
        ["env", provider, name] => {
            if providers.named(provider).is_none() {
                return refuse(1, &format!("no provider `{provider}`"));
            }
            let Some(e) = all
                .iter()
                .find(|e| e.provider.spec().name == *provider && e.name == *name)
            else {
                return refuse(1, &format!("no account `{name}` of provider `{provider}`"));
            };
            let (signed_in, why) = e.state();
            if let Some(why) = why {
                return refuse(1, &why);
            }
            let mut env = Map::new();
            match &e.login {
                Login::Folder(dir) => {
                    if let Some((var, dir)) = e.provider.env(dir.as_deref()) {
                        env.insert(var.to_owned(), json!(dir));
                    }
                }
                Login::Token(path, var) => match fs::read_to_string(path) {
                    Ok(t) if !t.trim().is_empty() => {
                        env.insert((*var).to_owned(), json!(t.trim()));
                    }
                    _ => return refuse(1, &format!("{}: empty or unreadable", path.display())),
                },
            }
            if !signed_in {
                if require {
                    return refuse(3, &format!("{provider}/{name} is signed out"));
                }
                eprintln!("cooee: {provider}/{name} is signed out");
            }
            println!("{}", Value::Object(env));
            ExitCode::SUCCESS
        }
        _ => refuse(
            2,
            "usage: cooee accounts <ls [--json] | env <provider> <name> [--require-signed-in] \
             | add <provider> <name> --folder <path> | rm <provider> <name>>",
        ),
    }
}

/// An account as `accounts ls --json` lists it (never its token), with a signed-in folder account's usage and the
/// reading's age in seconds.
fn row(e: &Entry, accounts: &Accounts) -> Value {
    let (signed_in, error) = e.state();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let usage = match &e.login {
        Login::Folder(dir) if signed_in => accounts.usage(e.provider, &e.name, dir.as_deref(), now),
        _ => None,
    };
    let usage = usage.map(|u| {
        let mut v = json!(u);
        v["age_s"] = json!(now.saturating_sub(u.read_at));
        v
    });
    let (folder, token) = match &e.login {
        Login::Folder(dir) => (dir.as_deref(), None),
        Login::Token(path, _) => (None, Some(path)),
    };
    json!({
        "provider": e.provider.spec().name,
        "name": e.name,
        "folder": folder,
        "signed_in": signed_in,
        "token_file": token,
        "token_age_days": token.and_then(|p| token_age(p).ok()),
        "error": error,
        "usage": usage,
    })
}
