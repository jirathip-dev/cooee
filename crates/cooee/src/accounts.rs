use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
    time::UNIX_EPOCH,
};

use cooee_core::{Accounts, Config, Provider, Providers};
use serde_json::{Map, Value, json};

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
        .and_then(|m| cooee_core::clock::now().duration_since(m).ok());
    Ok(age.map_or(0, |a| a.as_secs() / 86_400))
}

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
    let Some(data) = config.data_dir() else {
        return refuse(1, "no data folder: set `data` or HOME");
    };
    let shed = cooee_core::Shed::new(config.shed.clone(), config.shed_dir.clone());
    let providers = Providers::load(config.providers.as_deref(), &shed);
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
        ["ls"] => list(&all, &accounts, json),
        ["env", provider, name] => print_env(&providers, &all, (provider, name), require),
        _ => refuse(
            2,
            "usage: cooee accounts <ls [--json] | env <provider> <name> [--require-signed-in] \
             | add <provider> <name> --folder <path> | rm <provider> <name>>",
        ),
    }
}

fn refuse(code: u8, why: &str) -> ExitCode {
    cooee_core::note!("cooee: {why}");
    ExitCode::from(code)
}

fn list(all: &[Entry], accounts: &Accounts, json: bool) -> ExitCode {
    let rows: Vec<Value> = all.iter().map(|e| row(e, accounts)).collect();
    if json {
        cooee_core::say!("{}", Value::Array(rows));
    } else {
        for r in rows {
            let field = |k: &str| r.get(k).and_then(Value::as_str);
            let at = field("folder").or(field("token_file"));
            let state = if r.get("signed_in") == Some(&Value::Bool(true)) {
                "signed in"
            } else {
                "signed out"
            };
            let name = field("name").filter(|n| !n.is_empty());
            cooee_core::say!(
                "{}  {}  {state}  {}",
                field("provider").unwrap_or_default(),
                name.unwrap_or("(system default)"),
                at.unwrap_or("~")
            );
        }
    }
    ExitCode::SUCCESS
}

fn print_env(
    providers: &Providers,
    all: &[Entry],
    (provider, name): (&str, &str),
    require: bool,
) -> ExitCode {
    if providers.named(provider).is_none() {
        return refuse(1, &format!("no provider `{provider}`"));
    }
    let Some(e) = all
        .iter()
        .find(|e| e.provider.spec().name == provider && e.name == name)
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
        cooee_core::note!("cooee: {provider}/{name} is signed out");
    }
    cooee_core::say!("{}", Value::Object(env));
    ExitCode::SUCCESS
}

fn row(e: &Entry, accounts: &Accounts) -> Value {
    let (signed_in, error) = e.state();
    let now = cooee_core::env_var("COOEE_NOW")
        .and_then(|s| s.to_str()?.parse().ok())
        .unwrap_or_else(|| {
            cooee_core::clock::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs())
        });
    let usage = match &e.login {
        Login::Folder(dir) if signed_in => accounts.usage(e.provider, &e.name, dir.as_deref(), now),
        _ => None,
    };
    let usage = usage.map(|u| {
        let mut v = json!(u);
        if let Some(o) = v.as_object_mut() {
            o.insert("age_s".to_owned(), json!(now.saturating_sub(u.read_at)));
        }
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
