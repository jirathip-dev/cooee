use std::path::Path;

use cooee_core::{
    agents::Agent,
    mcp::Server,
    provider::{Channel, Gap, Setup, Want},
};

use super::{HubError, Recipe};
use crate::login::Logins;

pub fn persona(
    logins: &Logins,
    a: &Agent,
    account: Option<&str>,
    (data, config, registry): (&Path, Option<&Path>, &[Server]),
) -> Result<Recipe, HubError> {
    let brief = a.brief(config);
    let off = a.off(registry);
    let run = data.join("run").join(&a.id);
    let want = Want {
        brief: &brief,
        off: &off,
        ask_all: true,
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
    launch.policy = Some("ask".to_owned());
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
