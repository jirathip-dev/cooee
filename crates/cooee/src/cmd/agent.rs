use std::path::Path;

use cooee_core::agents::is_id;

use crate::cli::{Failed, Flag, Invocation};

pub(super) const USAGE: &str = "agent add --from <sample> [--id <id>] | agent add --list";
pub(super) const FLAGS: &[Flag] = &[
    Flag::Value("--from"),
    Flag::Value("--id"),
    Flag::Switch("--list"),
];

macro_rules! sample {
    ($id:literal $(, $extra:literal)*) => {
        ($id, &[
            ("agent.toml", include_str!(concat!("../../../../examples/agents/", $id, "/agent.toml"))),
            ("CHARACTER.md", include_str!(concat!("../../../../examples/agents/", $id, "/CHARACTER.md"))),
            ("INSTRUCTIONS.md", include_str!(concat!("../../../../examples/agents/", $id, "/INSTRUCTIONS.md"))),
            ("memory.md", include_str!(concat!("../../../../examples/agents/", $id, "/memory.md"))),
            $(($extra, include_str!(concat!("../../../../examples/agents/", $id, "/", $extra))),)*
        ])
    };
}

type Sample = (&'static str, &'static [(&'static str, &'static str)]);

const SAMPLES: &[Sample] = &[
    sample!("analyst"),
    sample!("assistant"),
    sample!("errands"),
    sample!("finance", "routine.toml"),
    sample!("inbox", "routine.toml"),
    sample!("morning-brief", "routine.toml"),
    sample!("orchestrator"),
    sample!("planner", "skills/spec/SKILL.md"),
    sample!("researcher", "skills/cite/SKILL.md"),
    sample!("travel"),
    sample!("tutor", "routine.toml"),
    sample!("writer"),
];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    let config = inv.path().and_then(Path::parent);
    match (inv.rest().as_slice(), inv.value("--from"), inv.on("--list")) {
        (["add"], None, true) => {
            for (id, files) in SAMPLES {
                let character = files.iter().find(|(n, _)| *n == "CHARACTER.md");
                let line = character.and_then(|(_, t)| t.lines().next());
                cooee_core::say!("{id:<14} {}", line.unwrap_or_default());
            }
            Ok(())
        }
        (["add"], Some(from), false) => {
            let config = config.ok_or("no config folder: run `cooee setup` first".to_owned())?;
            Ok(add(config, from, inv.value("--id").unwrap_or(from))?)
        }
        _ => Err(Failed::Usage),
    }
}

fn add(config: &Path, from: &str, id: &str) -> Result<(), String> {
    let (_, files) = SAMPLES
        .iter()
        .find(|(s, _)| *s == from)
        .ok_or_else(|| format!("no sample {from}: `cooee agent add --list` lists them"))?;
    if !is_id(id) {
        return Err(format!(
            "{id:?} isn't an id: lowercase letters, digits and -"
        ));
    }
    let dir = config.join("agents").join(id);
    if dir.exists() {
        return Err(format!("agent {id} is taken: pick another with --id <id>"));
    }
    for (name, text) in *files {
        cooee_core::write_private(&dir.join(name), text.as_bytes())
            .map_err(|e| format!("{}: {e}", dir.join(name).display()))?;
    }
    cooee_core::say!("added agent {id} from sample {from}: {}", dir.display());
    Ok(())
}
