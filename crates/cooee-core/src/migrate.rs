//! The move to one folder per agent (#359, q1617), the only module that knows the old paths: `<config>/personas/`,
//! `<config>/profiles/`, `<data>/personas/` and `<data>/routines.json`. [`plan`] only reads; [`Plan::apply`] stages
//! every agent under `agents.migrating/`, commits by renaming it `agents/`, then renames each old path
//! `<name>.bak-<date>`. Nothing is deleted or edited, and `agents/` existing is the only "done" marker.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{
    agents::{self, Agent, Role, Style},
    memory::{self, Note},
    profiles::Profile,
    routines::{self, Routine},
};

/// Where an install is in the move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Nothing old and no `agents/`: nothing to do (the first save makes `agents/`).
    Fresh,
    /// `agents/` exists and no old path does.
    Done,
    /// `agents/` exists and an old path still has its old name: reported, never touched.
    LeftOver,
    /// The old layout: to move.
    Old,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Write {
        to: PathBuf,
        from: Vec<PathBuf>,
        bytes: Vec<u8>,
        note: String,
    },
    Copy {
        to: PathBuf,
        from: PathBuf,
    },
    Warn(String),
    Backup {
        from: PathBuf,
        to: PathBuf,
    },
}

/// The move, as data: `--dry-run` prints its lines, a run logs the same.
#[derive(Debug)]
pub struct Plan {
    state: State,
    steps: Vec<Step>,
    config: PathBuf,
    data: PathBuf,
    agents: usize,
    routines: usize,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Persona {
    name: String,
    role: Option<Role>,
    style: Option<Style>,
    fleets: Vec<String>,
    avatar: Option<String>,
    character: String,
    instructions: String,
    folder: String,
    agent: String,
    model: String,
    effort: String,
    machine: String,
    plugins: Vec<String>,
    can_message: Vec<String>,
    pinned: bool,
    profile: Option<String>,
}

#[derive(Deserialize)]
struct OldRoutines {
    routines: Vec<Routine>,
}

fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if p.is_dir() {
            out.extend(files_under(&p));
        } else {
            out.push(p);
        }
    }
    out.sort();
    out
}

fn tomls(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    out.retain(|p| p.is_file() && p.extension().is_some_and(|e| e == "toml"));
    out.sort();
    out
}

fn stem(p: &Path) -> &str {
    p.file_stem().and_then(|s| s.to_str()).unwrap_or("")
}

/// The move of the install with config folder `config` and data folder `data`, its backups dated `today`. Reads
/// only.
#[must_use]
pub fn plan(config: &Path, data: &Path, today: &str) -> Plan {
    let mut old = Vec::new();
    for p in [
        config.join("personas"),
        config.join("profiles"),
        data.join("personas"),
        data.join("routines.json"),
    ] {
        if p.exists() && !old.contains(&p) {
            old.push(p);
        }
    }
    let mut plan = Plan {
        state: State::Fresh,
        steps: Vec::new(),
        config: config.to_owned(),
        data: data.to_owned(),
        agents: 0,
        routines: 0,
    };
    if config.join("agents").exists() {
        plan.state = if old.is_empty() {
            State::Done
        } else {
            State::LeftOver
        };
        for o in &old {
            let o = plan.show(o);
            plan.warn(format!(
                "left over {o}: agents/ exists, so it was not migrated; move agents/ away and run `cooee config migrate`"
            ));
        }
        return plan;
    }
    if old.is_empty() {
        return plan;
    }
    plan.state = State::Old;
    plan.agents();
    plan.routines();
    for from in old {
        let name = from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut to = from.with_file_name(format!("{name}.bak-{today}"));
        for n in 2.. {
            if !to.exists() {
                break;
            }
            to = from.with_file_name(format!("{name}.bak-{today}-{n}"));
        }
        plan.steps.push(Step::Backup { from, to });
    }
    plan
}

impl Plan {
    /// Where the install is.
    #[must_use]
    pub fn state(&self) -> State {
        self.state
    }

    fn show(&self, p: &Path) -> String {
        match (p.strip_prefix(&self.config), p.strip_prefix(&self.data)) {
            (Ok(rel), _) => rel.display().to_string(),
            (_, Ok(rel)) => format!("<data>/{}", rel.display()),
            _ => p.display().to_string(),
        }
    }

    fn warn(&mut self, why: String) {
        self.steps.push(Step::Warn(why));
    }

    fn write(&mut self, to: PathBuf, from: &[PathBuf], bytes: Vec<u8>, note: String) {
        let from = from.to_vec();
        self.steps.push(Step::Write {
            to,
            from,
            bytes,
            note,
        });
    }

    fn profiles(&mut self) -> Vec<(PathBuf, Agent)> {
        let dir = self.config.join("profiles");
        let mut out: Vec<(PathBuf, Agent)> = Vec::new();
        for file in tomls(&dir) {
            let shown = self.show(&file);
            let read = fs::read_to_string(&file).map_err(|e| e.kind().to_string());
            let parsed = read.and_then(|t| {
                toml::from_str::<Profile>(&t).map_err(|_| "it doesn't parse".to_owned())
            });
            let p = match parsed.and_then(|p| p.check().map(|()| p)) {
                Ok(p) if p.hidden => {
                    self.warn(format!(
                        "{shown}: hidden, so not migrated; it stays in the backup"
                    ));
                    continue;
                }
                Ok(p) if out.iter().any(|(_, a)| a.id == p.name) => {
                    self.warn(format!(
                        "{shown}: a second profile {}; not migrated",
                        p.name
                    ));
                    continue;
                }
                Ok(p) => p.rooted(&dir),
                Err(e) => {
                    self.warn(format!(
                        "{shown}: not migrated ({e}); it stays in the backup"
                    ));
                    continue;
                }
            };
            let text = p
                .instructions
                .as_deref()
                .map(fs::read_to_string)
                .transpose();
            let text = text.unwrap_or_else(|_| {
                self.warn(format!("{shown}: its instructions file can't be read"));
                None
            });
            let id = p.name.clone();
            out.push((
                file,
                Agent {
                    id,
                    ..p.into_agent(text.unwrap_or_default())
                },
            ));
        }
        out
    }

    fn agents(&mut self) {
        let profiles = self.profiles();
        let lent = agents::all(None, &self.data, |_| None);
        let mut made: Vec<(Agent, Vec<PathBuf>)> = Vec::new();
        for file in tomls(&self.config.join("personas")) {
            let (id, shown) = (stem(&file).to_owned(), self.show(&file));
            let read = fs::read_to_string(&file).map_err(|e| e.kind().to_string());
            let parsed = read.and_then(|t| {
                toml::from_str::<Persona>(&t).map_err(|_| "it doesn't parse".to_owned())
            });
            let p = match parsed {
                Ok(p) if agents::is_id(&id) => p,
                Ok(_) => {
                    self.warn(format!("{shown}: its name isn't an id; not migrated"));
                    continue;
                }
                Err(e) => {
                    self.warn(format!(
                        "{shown}: not migrated ({e}); its notes and skills move to agents/{id}/"
                    ));
                    continue;
                }
            };
            let runs = p.profile.as_deref().map(|name| {
                let owner = profiles.iter().find(|(_, a)| a.id == name).cloned();
                let theirs = lent
                    .iter()
                    .find(|a| a.id == name)
                    .map(|a| (PathBuf::new(), a.clone()));
                (name.to_owned(), owner.or(theirs))
            });
            let role = p.role.unwrap_or(if id == "conductor" {
                Role::Orchestrator
            } else {
                Role::Plain
            });
            let mut a = Agent {
                id: id.clone(),
                name: p.name,
                role,
                style: agents::style(role, p.style),
                fleets: p.fleets,
                avatar: p.avatar,
                character: p.character,
                instructions: p.instructions,
                folder: p.folder,
                agent: p.agent,
                model: p.model,
                effort: p.effort,
                machine: p.machine,
                plugins: p.plugins,
                can_message: p.can_message,
                pinned: p.pinned,
                ..Agent::default()
            };
            let mut from = vec![file];
            match runs {
                Some((_, Some((pf, f)))) => {
                    self.merge(&mut a, f, &shown);
                    from.extend((!pf.as_os_str().is_empty()).then_some(pf));
                }
                Some((name, None)) => {
                    self.warn(format!(
                        "{shown}: runs as profile {name}, which is hidden or gone; it starts Plain"
                    ));
                }
                None => {}
            }
            made.push((a, from));
        }
        for (file, f) in profiles {
            if made.iter().all(|(a, _)| a.id != f.id) {
                let role = if f.id == "conductor" {
                    Role::Orchestrator
                } else {
                    Role::Plain
                };
                let style = agents::style(role, None);
                made.push((Agent { role, style, ..f }, vec![file]));
            }
        }
        let mut ids: Vec<String> = made.iter().map(|(a, _)| a.id.clone()).collect();
        for (a, from) in made {
            self.folder(a, &from);
        }
        self.agents = ids.len();
        for d in fs::read_dir(self.data.join("personas"))
            .into_iter()
            .flatten()
            .flatten()
        {
            let id = d.file_name().to_string_lossy().into_owned();
            if d.path().is_dir() && agents::is_id(&id) && !ids.contains(&id) {
                self.kept(&id);
                ids.push(id);
            }
        }
    }

    fn merge(&mut self, a: &mut Agent, f: Agent, shown: &str) {
        let id = a.id.clone();
        if f.name != a.name {
            self.warn(format!(
                "agents/{id}: `name` differs in {shown} and its profile; the persona's is kept"
            ));
        }
        if f.agent != a.agent {
            self.warn(format!(
                "agents/{id}: `agent` differs in {shown} and its profile; the profile's is kept"
            ));
        }
        if !f.instructions.is_empty() {
            self.warn(format!(
                "agents/{id}: its profile's instructions file was never read for it; not migrated"
            ));
        }
        if f.lent.is_some() {
            self.warn(format!(
                "agents/{id}: runs as a plugin's profile; its launch keys are copied"
            ));
        }
        a.agent = f.agent;
        if !f.folder.is_empty() {
            a.folder = f.folder;
        }
        a.avatar = a.avatar.take().or(f.avatar);
        (a.account, a.settings, a.env, a.mcp, a.policy) =
            (f.account, f.settings, f.env, f.mcp, f.policy);
    }

    fn folder(&mut self, mut a: Agent, from: &[PathBuf]) {
        let dir = self.config.join("agents").join(&a.id);
        let profiles = self.config.join("profiles");
        if let Some(s) = a.settings.clone()
            && let Ok(rel) = s.strip_prefix(&profiles)
            && s.exists()
        {
            let files = if s.is_dir() {
                files_under(&s)
            } else {
                vec![s.clone()]
            };
            for from in files {
                let to = dir.join(from.strip_prefix(&profiles).unwrap_or(&from));
                if fs::read(&from).is_ok_and(|b| {
                    String::from_utf8_lossy(&b).contains(&*profiles.to_string_lossy())
                }) {
                    let shown = self.show(&to);
                    self.warn(format!(
                        "{shown}: names a path under profiles/, which is renamed as its backup"
                    ));
                }
                self.steps.push(Step::Copy { to, from });
            }
            a.settings = Some(rel.to_owned());
        }
        for (name, bytes) in agents::files(&a) {
            self.write(dir.join(name), from, bytes, String::new());
        }
        self.kept(&a.id);
    }

    fn kept(&mut self, id: &str) {
        let (old, dir) = (
            self.data.join("personas").join(id),
            self.config.join("agents").join(id),
        );
        let jsonl = old.join("memory.jsonl");
        if let Ok(text) = fs::read_to_string(&jsonl) {
            let notes: Vec<Note> = text
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            let (notes, md) = memory::render(notes);
            let note = format!(" ({} notes)", notes.len());
            self.write(dir.join("memory.md"), &[jsonl], md.into_bytes(), note);
        }
        let skills = old.join("skills");
        for from in files_under(&skills) {
            let to = dir
                .join("skills")
                .join(from.strip_prefix(&skills).unwrap_or(&from));
            self.steps.push(Step::Copy { to, from });
        }
    }

    fn routines(&mut self) {
        let file = self.data.join("routines.json");
        let Ok(bytes) = fs::read(&file) else { return };
        let Ok(old) = serde_json::from_slice::<OldRoutines>(&bytes) else {
            let shown = self.show(&file);
            return self.warn(format!(
                "{shown}: it doesn't parse; its routines stay in the backup"
            ));
        };
        let (toml, runs) = routines::files(&old.routines);
        self.routines = old.routines.len();
        let note = format!(" ({} routines)", self.routines);
        let from = [file];
        self.write(
            self.config.join("routines.toml"),
            &from,
            toml.into_bytes(),
            note,
        );
        self.write(
            self.data.join("routine-runs.json"),
            &from,
            runs,
            String::new(),
        );
    }

    fn line(&self, s: &Step) -> String {
        match s {
            Step::Write { to, from, note, .. } => {
                let from: Vec<String> = from.iter().map(|f| self.show(f)).collect();
                format!(
                    "write  {:<36} from {}{note}",
                    self.show(to),
                    from.join(", ")
                )
            }
            Step::Copy { to, from } => {
                format!("copy   {:<36} from {}", self.show(to), self.show(from))
            }
            Step::Warn(why) => format!("warn   {why}"),
            Step::Backup { from, to } => format!("backup {} -> {}", self.show(from), self.show(to)),
        }
    }

    fn summary(&self) -> Option<String> {
        let n = (self.agents, self.routines);
        (self.state == State::Old)
            .then(|| format!("{} agents, {} routines; nothing is deleted", n.0, n.1))
    }

    /// What it does, one line per step: names files and keys, never a value.
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        let mut out: Vec<String> = self.steps.iter().map(|s| self.line(s)).collect();
        out.extend(self.summary());
        out
    }

    /// Does it, logging each line as it goes: stages the agents, commits them by one rename, then the backups.
    ///
    /// # Errors
    /// A file that can't be written or renamed; before the commit nothing is left but the old layout.
    pub fn apply(self, mut log: impl FnMut(&str)) -> io::Result<()> {
        if self.state != State::Old {
            self.lines().iter().for_each(|l| log(l));
            return Ok(());
        }
        let (agents, staged) = (
            self.config.join("agents"),
            self.config.join("agents.migrating"),
        );
        match fs::remove_dir_all(&staged) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        let at = |to: &Path| {
            to.strip_prefix(&agents)
                .map_or_else(|_| to.to_owned(), |rel| staged.join(rel))
        };
        let mut staging = || -> io::Result<()> {
            fs::create_dir_all(&staged)?;
            for s in &self.steps {
                match s {
                    Step::Write { to, bytes, .. } => crate::write_private(&at(to), bytes)?,
                    Step::Copy { to, from } => {
                        fs::create_dir_all(at(to).parent().unwrap_or(&staged))?;
                        fs::copy(from, at(to))?;
                    }
                    Step::Warn(_) | Step::Backup { .. } => continue,
                }
                log(&self.line(s));
            }
            Ok(())
        };
        if let Err(e) = staging() {
            let _ = fs::remove_dir_all(&staged);
            return Err(e);
        }
        fs::rename(&staged, &agents)?;
        for s in &self.steps {
            match s {
                Step::Backup { from, to } => fs::rename(from, to)?,
                Step::Warn(_) => {}
                _ => continue,
            }
            log(&self.line(s));
        }
        self.summary().iter().for_each(|l| log(l));
        Ok(())
    }
}
