use cooee_core::Config;

use crate::cli::{Failed, Flag, Invocation};

pub(super) const USAGE: &str = "mcp ls | mcp add <name> (<command> [<arg>...] | --url <url> [--type sse]) [--env K=secret:N|env:N]... [--header K=secret:N|env:N]... | mcp rm|enable|disable <name> | mcp proxy <server> --thread <id> | mcp serve --thread <id>";
pub(super) const FLAGS: &[Flag] = &[];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    match inv.rest().as_slice() {
        [agent @ ("proxy" | "serve"), rest @ ..] => Ok(mcp_agent(agent, rest, inv.usage())?),
        rest => Ok(mcp(&inv.config()?, rest, inv.usage())?),
    }
}

fn mcp_agent(agent: &str, args: &[&str], usage: &str) -> Result<(), String> {
    match (agent, args) {
        ("proxy", [server, "--thread", thread]) => cooee_server::mcp::proxy(server, thread),
        ("serve", ["--thread", thread]) => cooee_server::mcp::serve(thread),
        _ => Err(usage.to_owned()),
    }
}

fn mcp(config: &Config, args: &[&str], usage: &str) -> Result<(), String> {
    use cooee_core::mcp::{Server, save};
    let path = config
        .path
        .as_deref()
        .ok_or("no config file: set HOME or --config")?;
    let mut list = config.mcp.clone();
    let said = match args {
        ["ls"] => {
            for s in &list {
                let on = if s.enabled { "on" } else { "off" };
                println!("[{}] {} · {on} · {}", s.kind, s.name, s.shown());
            }
            return Ok(());
        }
        ["add", name, rest @ ..] => {
            let mut s = Server {
                name: (*name).to_owned(),
                kind: "stdio".to_owned(),
                command: String::new(),
                args: Vec::new(),
                url: String::new(),
                env: std::collections::BTreeMap::new(),
                headers: std::collections::BTreeMap::new(),
                enabled: true,
            };
            let mut words = rest.iter();
            let pair = |v: Option<&&str>| {
                v.and_then(|v| v.split_once('='))
                    .map(|(k, v)| (k.to_owned(), v.to_owned()))
            };
            while let Some(w) = words.next() {
                match *w {
                    "--url" | "--type" if s.command.is_empty() => {
                        let v = words.next().ok_or(usage)?;
                        if *w == "--url" {
                            (*v).clone_into(&mut s.url);
                            if s.kind == "stdio" {
                                "http".clone_into(&mut s.kind);
                            }
                        } else {
                            (*v).clone_into(&mut s.kind);
                        }
                    }
                    "--env" | "--header" => {
                        let (k, v) = pair(words.next()).ok_or(usage)?;
                        let to = if *w == "--env" {
                            &mut s.env
                        } else {
                            &mut s.headers
                        };
                        to.insert(k, v);
                    }
                    _ if s.command.is_empty() && s.url.is_empty() => {
                        (*w).clone_into(&mut s.command);
                    }
                    _ => s.args.push((*w).to_owned()),
                }
            }
            if list.iter().any(|o| o.name == s.name) {
                return Err(format!(
                    "there is an MCP server {name} already: `cooee mcp rm {name}` first"
                ));
            }
            let said = format!("added {name}: [{}] {}", s.kind, s.shown());
            list.push(s);
            said
        }
        ["rm", name] => {
            list.retain(|s| s.name != *name);
            format!("removed {name}")
        }
        [on @ ("enable" | "disable"), name] => {
            let s = list
                .iter_mut()
                .find(|s| s.name == *name)
                .ok_or(format!("no MCP server {name}"))?;
            s.enabled = *on == "enable";
            format!("{name} is {}", if s.enabled { "on" } else { "off" })
        }
        _ => return Err(usage.to_owned()),
    };
    cooee_core::mcp::check(&list)?;
    save(path, &list).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{said}");
    Ok(())
}
