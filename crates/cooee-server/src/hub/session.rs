use std::path::Path;

use cooee_core::{
    agents::Agent,
    provider::{Channel, Gap, Setup, Want},
};

use super::{HubError, Recipe};
use crate::login::Logins;

pub(crate) fn persona(
    logins: &Logins,
    a: &Agent,
    account: Option<&str>,
    (data, config): (&Path, Option<&Path>),
) -> Result<Recipe, HubError> {
    let brief = a.brief(config);
    let (off, run) = (a.off(), data.join("run").join(&a.id));
    let want = Want {
        brief: &brief,
        off: &off,
        ask_all: false,
        run: &run,
    };
    let setup = logins.session(&a.agent, &want);
    let signed = setup.is_some();
    let setup = setup.unwrap_or_else(|| Setup::plain(&want));
    let (account, mut launch) = logins.agent(a, account, data)?;
    for (file, text) in &setup.files {
        let file = run.join(file);
        cooee_core::write_private(&file, text.as_bytes())
            .map_err(|e| HubError::Refused(format!("{}: {e}", file.display())))?;
    }
    launch.meta = setup.meta;
    launch.env.extend(setup.env);
    launch.args.extend(setup.args);
    let own = signed && account.is_empty();
    let gaps = setup.gaps.into_iter();
    Ok(Recipe {
        account,
        profile: a.id.clone(),
        launch,
        instructions: if setup.brief == Channel::System {
            String::new()
        } else {
            brief
        },
        gaps: gaps.filter(|g| own || *g != Gap::PersonalFiles).collect(),
        ..Recipe::default()
    })
}
