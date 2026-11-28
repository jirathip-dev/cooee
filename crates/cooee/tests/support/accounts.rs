use std::path::Path;

use cooee_core::shed_proto::{ErrorCode, RpcError};
use serde_json::{Value, json};

use super::FakeShed;

fn valid(name: &str) -> bool {
    let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
    !name.is_empty() && name != "default" && name.bytes().all(ok)
}

impl FakeShed {
    pub(super) fn account(&self, method: &str, p: &Value) -> Result<Value, RpcError> {
        let provider = p["provider"].as_str().unwrap_or_default().to_owned();
        let name = p["name"].as_str().unwrap_or_default().to_owned();
        let flag = self.dir.join(format!("signed-in-{provider}-{name}"));
        let key = (provider.clone(), name.clone());
        let mut s = self.shared.lock().unwrap();
        match method {
            "account.ls" => {
                let mut all: Vec<Value> = s.accounts.values().cloned().collect();
                all.retain(|a| p["provider"].is_null() || a["provider"] == p["provider"]);
                all.sort_by_key(|a| a["name"] != "default");
                Ok(json!(all))
            }
            "account.add" => {
                if !valid(&name) {
                    return Err(RpcError::new(ErrorCode::InvalidParams, &name));
                }
                let token = p["token_file"].as_str().map(|_| json!({ "saved_at": 1 }));
                let a = s.accounts.entry(key).or_insert_with(|| {
                    json!({ "provider": provider, "name": name, "folder": p["folder"],
                            "adopted": !p["folder"].is_null(), "state": "signed-out", "line": "" })
                });
                if let Some(t) = token {
                    a["token"] = t;
                }
                Ok(a.clone())
            }
            "account.login" => {
                let known = name == "default" || s.accounts.contains_key(&key);
                drop(s);
                if !known {
                    return Err(RpcError::new(ErrorCode::NoAccount, &name));
                }
                self.login(&provider, &name, &flag)
            }
            "account.status" => {
                let a = s.accounts.entry(key).or_insert_with(|| {
                    json!({ "provider": provider, "name": name, "state": "signed-out", "line": "" })
                });
                let ready = flag.exists() || !a["token"].is_null();
                a["state"] = json!(if ready { "ready" } else { "signed-out" });
                a["line"] = json!(if ready { "signed in" } else { "signed out" });
                a["who"] = if ready {
                    json!(format!("{name}@example.com"))
                } else {
                    Value::Null
                };
                Ok(a.clone())
            }
            "account.rm" if name == "default" => {
                Err(RpcError::new(ErrorCode::InvalidParams, "default"))
            }
            "account.rm" => match s.accounts.remove(&key) {
                Some(_) => Ok(json!({})),
                None => Err(RpcError::new(ErrorCode::NoAccount, &name)),
            },
            other => Err(RpcError::new(ErrorCode::MethodNotFound, other)),
        }
    }

    fn login(&self, provider: &str, name: &str, flag: &Path) -> Result<Value, RpcError> {
        let terminal = format!("login-{provider}-{name}");
        let script = self.dir.join(format!("login-{provider}"));
        let run = json!({ "name": terminal, "cwd": self.dir, "cmd": [script, flag] });
        self.run(&run)?;
        Ok(json!({ "terminal": terminal }))
    }

    /// Account `name` of `provider` as shed lists it.
    pub fn account_of(&self, provider: &str, name: &str) -> Option<Value> {
        let s = self.shared.lock().unwrap();
        s.accounts
            .get(&(provider.to_owned(), name.to_owned()))
            .cloned()
    }

    /// Puts `account` (shed's JSON) in this shed's list, as `account.ls` gives it.
    pub fn put_account(&self, account: Value) {
        let key = |k: &str| account[k].as_str().unwrap().to_owned();
        let key = (key("provider"), key("name"));
        self.shared.lock().unwrap().accounts.insert(key, account);
    }

    /// How many `method` requests shed got.
    pub fn calls_of(&self, method: &str) -> usize {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, _)| m == method)
            .count()
    }
}
