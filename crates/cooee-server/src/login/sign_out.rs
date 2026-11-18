use super::Logins;
use crate::hub::{HubError, lock};

impl Logins {
    /// Signs provider `name`'s account `account` out with the CLI's logout, then removes its folder (an added
    /// account's, even when the logout fails; the system default's is never removed).
    ///
    /// # Errors
    /// [`HubError::Refused`] for an unknown account, or with the logout's output when it fails.
    pub fn sign_out(&self, name: &str, account: &str) -> Result<(), HubError> {
        let provider = self.provider(name)?;
        let a = self.account(name, account)?;
        let out = provider.sign_out(a.dir.as_deref());
        let changed = out.is_ok() || a.dir.is_some();
        if changed {
            self.accounts
                .remove(name, account)
                .map_err(|e| HubError::Refused(e.to_string()))?;
        }
        let key = (name.to_owned(), account.to_owned());
        if let Some(c) = lock(&self.checks).get_mut(&key) {
            c.next = 0;
            if changed {
                c.status = None;
            }
        }
        let who = if account.is_empty() { name } else { account };
        out.map_err(|e| HubError::Refused(format!("{who} sign-out: {e}")))
    }
}
