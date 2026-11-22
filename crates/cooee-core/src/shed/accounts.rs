use serde_json::{Value, json};
use shed_proto::{Account, AccountAdd, AccountName, AccountsParams, Login, Method};

use super::{Shed, ShedError, client::CALL};

impl Shed {
    /// # Errors [`ShedError`] when shed's `account.ls` (of `provider` or all, `default` first) fails.
    pub fn accounts(&self, provider: Option<&str>) -> Result<Vec<Account>, ShedError> {
        let params = AccountsParams {
            provider: provider.map(str::to_owned),
        };
        self.call(Method::AccountLs, &json!(params))
    }

    /// # Errors [`ShedError`] when shed refuses `account.add` (makes an account or adopts a folder; twice is once).
    pub fn account_add(&self, params: &AccountAdd) -> Result<Account, ShedError> {
        self.call(Method::AccountAdd, &json!(params))
    }

    /// # Errors [`ShedError`] when shed refuses `account.login` (the login in a redacted terminal, its name).
    pub fn account_login(&self, provider: &str, name: &str) -> Result<String, ShedError> {
        let login: Login = self.call(Method::AccountLogin, &json!(named(provider, name)))?;
        Ok(login.terminal)
    }

    /// # Errors [`ShedError`] when `account.status` (the account's status and usage read now) fails.
    pub fn account_status(&self, provider: &str, name: &str) -> Result<Account, ShedError> {
        let mut conn = self.connect_for(CALL * 3)?;
        conn.call(Method::AccountStatus, json!(named(provider, name)))
    }

    /// # Errors [`ShedError`] when shed refuses `account.rm` (logs the account out and removes it).
    pub fn account_rm(&self, provider: &str, name: &str) -> Result<(), ShedError> {
        let mut conn = self.connect_for(CALL * 3)?;
        conn.call::<Value>(Method::AccountRm, json!(named(provider, name)))
            .map(drop)
    }
}

fn named(provider: &str, name: &str) -> AccountName {
    AccountName {
        provider: provider.to_owned(),
        name: name.to_owned(),
    }
}
