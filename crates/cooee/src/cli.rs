use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use cooee_core::Config;

pub(crate) struct Command {
    /// The first word: the file's name.
    pub name: &'static str,
    /// Its part of the usage line.
    pub usage: &'static str,
    pub flags: &'static [Flag],
    pub run: fn(&Invocation) -> Result<(), Failed>,
}

pub(crate) enum Flag {
    Switch(&'static str),
    Value(&'static str),
}

pub(crate) enum Failed {
    Usage,
    Because(String),
    Exit(ExitCode),
}

impl From<String> for Failed {
    fn from(why: String) -> Self {
        Self::Because(why)
    }
}

pub(crate) struct Invocation {
    instance: String,
    path: Option<PathBuf>,
    explicit: bool,
    usage: String,
    words: Vec<String>,
    switches: Vec<&'static str>,
    values: Vec<(&'static str, String)>,
}

fn flag(args: &mut Vec<String>, name: &str) -> Result<Option<String>, ()> {
    let Some(i) = args.iter().position(|a| a == name) else {
        return Ok(None);
    };
    if i + 1 >= args.len() {
        return Err(());
    }
    let value = args.remove(i + 1);
    args.remove(i);
    Ok(Some(value))
}

impl Invocation {
    /// Takes every switch, then `--instance`, `--config` and every value flag out of `args`; `Failed::Usage` when a
    /// value is missing.
    pub fn parse<'a>(
        mut args: Vec<String>,
        flags: impl Iterator<Item = &'a Flag>,
        usage: String,
    ) -> Result<Self, Failed> {
        let (mut on, mut valued) = (Vec::new(), vec!["--instance", "--config"]);
        for f in flags {
            let (Flag::Switch(name) | Flag::Value(name)) = f;
            let to = if matches!(f, Flag::Switch(_)) {
                &mut on
            } else {
                &mut valued
            };
            if !to.contains(name) {
                to.push(*name);
            }
        }
        let mut switches = Vec::new();
        for name in on {
            if let Some(i) = args.iter().position(|a| a == name) {
                args.remove(i);
                switches.push(name);
            }
        }
        let mut found = Vec::new();
        for name in valued {
            if let Some(v) = flag(&mut args, name).map_err(|()| Failed::Usage)? {
                found.push((name, v));
            }
        }
        let mut inv = Self {
            instance: String::new(),
            path: None,
            explicit: false,
            usage,
            words: args,
            switches,
            values: found,
        };
        inv.instance = inv.value("--instance").unwrap_or("cooee").to_owned();
        inv.explicit = inv.value("--config").is_some();
        inv.path = inv
            .value("--config")
            .map(PathBuf::from)
            .or_else(|| Config::default_path(&inv.instance));
        Ok(inv)
    }

    /// Every word left, the command's first.
    pub fn words(&self) -> Vec<&str> {
        self.words.iter().map(String::as_str).collect()
    }

    /// What is left after the command's word.
    pub fn rest(&self) -> Vec<&str> {
        self.words.iter().skip(1).map(String::as_str).collect()
    }

    pub fn on(&self, switch: &str) -> bool {
        self.switches.contains(&switch)
    }

    pub fn value(&self, flag: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(f, _)| *f == flag)
            .map(|(_, v)| v.as_str())
    }

    pub fn instance(&self) -> &str {
        &self.instance
    }

    /// The config file: `--config`, else the instance's default path.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Whether `--config` named the file.
    pub fn explicit(&self) -> bool {
        self.explicit
    }

    /// The whole usage line.
    pub fn usage(&self) -> &str {
        &self.usage
    }

    /// The instance's config, loaded once (the defaults when there is no file).
    pub fn config(&self) -> Result<Config, Failed> {
        Ok(load(self.path(), &self.instance)?)
    }
}

pub(crate) fn load(path: Option<&Path>, instance: &str) -> Result<Config, String> {
    let config = path.map_or_else(|| Ok(Config::default()), Config::load);
    let config = config.map_err(|e| e.to_string())?;
    Ok(Config {
        instance: instance.to_owned(),
        ..config
    })
}

pub(crate) fn confirm(text: &str, yes: bool) -> Result<(), String> {
    println!("{text}");
    if yes {
        return Ok(());
    }
    print!("go ahead? [y/N] ");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let mut answer = String::new();
    let _ = std::io::stdin().read_line(&mut answer);
    match answer.trim() {
        "y" | "yes" => Ok(()),
        _ => Err("nothing changed".to_owned()),
    }
}
