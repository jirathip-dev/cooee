use serde_json::{Value, json};
use shed_proto::{Account, AccountAdd, AccountName, AccountsParams, Login, Method};

use super::{Shed, ShedError, client::CALL};

impl Shed {
    /// shed's accounts (`account.ls`), of `provider` or all, per provider `default` first.
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn accounts(&self, provider: Option<&str>) -> Result<Vec<Account>, ShedError> {
        let params = AccountsParams {
            provider: provider.map(str::to_owned),
        };
        self.call(Method::AccountLs, &json!(params))
    }

    /// Makes an account, or adopts an existing folder or token file for it (`account.add`); twice is once.
    ///
    /// # Errors
    /// [`ShedError`] when shed refuses it.
    pub fn account_add(&self, params: &AccountAdd) -> Result<Account, ShedError> {
        self.call(Method::AccountAdd, &json!(params))
    }

    /// Starts account `name` of `provider`'s login in a terminal shed redacts (`account.login`); its name.
    ///
    /// # Errors
    /// [`ShedError`] when shed refuses it.
    pub fn account_login(&self, provider: &str, name: &str) -> Result<String, ShedError> {
        let login: Login = self.call(Method::AccountLogin, &json!(named(provider, name)))?;
        Ok(login.terminal)
    }

    /// Account `name` of `provider` as its status and usage read now (`account.status`).
    ///
    /// # Errors
    /// [`ShedError`] when the call fails.
    pub fn account_status(&self, provider: &str, name: &str) -> Result<Account, ShedError> {
        let mut conn = self.connect_for(CALL * 3)?;
        conn.call(Method::AccountStatus, json!(named(provider, name)))
    }

    /// Logs account `name` of `provider` out and removes it (`account.rm`).
    ///
    /// # Errors
    /// [`ShedError`] when shed refuses it.
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
