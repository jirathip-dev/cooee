use std::{io, path::PathBuf};

use cooee_core::{
    agents,
    rules::{Rule, Rules},
};
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    router::{RouterBuilder, content::Json, path_param, route},
};

use crate::http::{Answer, Name, now, on_personas, thread_id};

#[derive(Serialize)]
struct Row {
    #[serde(flatten)]
    rule: Rule,
    words: String,
}

impl From<Rule> for Row {
    fn from(rule: Rule) -> Self {
        Self {
            words: rule.words(),
            rule,
        }
    }
}

#[derive(Deserialize)]
struct NewRule {
    effect: String,
    kind: String,
    pattern: String,
    folder: Option<PathBuf>,
}

fn home(
    data: &std::path::Path,
    config: &std::path::Path,
    id: &str,
) -> io::Result<(agents::Home, Option<PathBuf>)> {
    let folder = agents::find(Some(config), data, id)?.work_folder();
    Ok((agents::home(config, id)?, folder))
}

#[route(GET "/personas/{id}/rules")]
async fn rule_list(cx: &Cx) -> Result<Answer<Vec<Row>>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        let (home, folder) = home(d, c, &id)?;
        let rules = Rules::load(&home, folder.as_deref());
        Ok(rules.list().iter().cloned().map(Row::from).collect())
    })
    .await
}

#[route(POST "/personas/{id}/rules")]
async fn rule_add(cx: &Cx, Json(r): Json<NewRule>) -> Result<Answer<Row>> {
    let id = thread_id(cx);
    on_personas(cx, move |d, c| {
        let (home, folder) = home(d, c, &id)?;
        let rule = Rule::parse(&r.effect, &r.kind, &r.pattern, r.folder.as_deref())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let rule = Rule {
            by: "settings".to_owned(),
            at: now(),
            ..rule
        };
        Rules::add(&home, folder.as_deref(), rule).map(Row::from)
    })
    .await
}

#[route(DELETE "/personas/{id}/rules/{name}")]
async fn rule_remove(cx: &Cx) -> Result<Answer<()>> {
    let (id, rule) = (thread_id(cx), path_param::<Name>(cx).to_owned());
    on_personas(cx, move |d, c| {
        let (home, folder) = home(d, c, &id)?;
        Rules::remove(&home, folder.as_deref(), &rule)
    })
    .await
}

pub(super) fn routes(b: RouterBuilder) -> RouterBuilder {
    b.route(rule_list).route(rule_add).route(rule_remove)
}
