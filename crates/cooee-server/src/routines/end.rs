use cooee_core::{
    Body,
    routines::{End, Ended, Paused, Routine},
};

use super::Runner;
use crate::{http::refused, hub::HubError};

impl Runner {
    /// `stop_routine` by thread `id`'s agent (`token`: its MCP token): routine `routine` ends now, when this chat
    /// made it; its chat gets the `Ended routine` line.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, no such routine (`NO_ROUTINE`) or one another chat made (`NOT_YOURS`).
    pub fn stop(
        &self,
        (id, token): (&str, &str),
        routine: &str,
        now: u64,
    ) -> Result<String, HubError> {
        let from = self.hub.token_of(id, token)?;
        let r = self.change(|b| {
            let r = b.routines.iter_mut().find(|r| r.id == routine);
            let r = r.ok_or_else(|| refused(format!("NO_ROUTINE: no routine {routine}")))?;
            if r.made_in != from.id {
                return Err(refused(format!(
                    "NOT_YOURS: routine {routine} was made in another chat"
                )));
            }
            r.paused = Some(Paused::Ended(Ended {
                at: now,
                by: End::StopRoutine,
            }));
            Ok(r.clone())
        })?;
        let text = format!("Ended routine {}", r.name);
        self.hub.say(id, Body::Note { text: text.clone() })?;
        Ok(format!(
            "{text}. The owner can resume or delete it in Settings › routines."
        ))
    }

    pub(crate) fn when(&self, r: &Routine, now: u64) -> String {
        let until = r.until(self.offset(now));
        if until.is_empty() {
            r.when()
        } else {
            format!("{} · {until}", r.when())
        }
    }
}
