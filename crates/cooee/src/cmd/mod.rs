mod accounts;
mod config;
mod devices;
mod doctor;
mod install;
mod layout;
mod machine;
mod mcp;
mod pair;
mod perf;
mod plugin;
mod push;
mod reports;
mod secrets;
mod serve;
mod setup;
mod theme;
mod update;

use crate::cli::Command;

pub(crate) const ALL: &[Command] = &[
    Command {
        name: "accounts",
        usage: accounts::USAGE,
        flags: accounts::FLAGS,
        run: accounts::run,
    },
    Command {
        name: "config",
        usage: config::USAGE,
        flags: config::FLAGS,
        run: config::run,
    },
    Command {
        name: "devices",
        usage: devices::USAGE,
        flags: devices::FLAGS,
        run: devices::run,
    },
    Command {
        name: "doctor",
        usage: doctor::USAGE,
        flags: doctor::FLAGS,
        run: doctor::run,
    },
    Command {
        name: "install",
        usage: install::USAGE,
        flags: install::FLAGS,
        run: install::run,
    },
    Command {
        name: "layout",
        usage: layout::USAGE,
        flags: layout::FLAGS,
        run: layout::run,
    },
    Command {
        name: "machine",
        usage: machine::USAGE,
        flags: machine::FLAGS,
        run: machine::run,
    },
    Command {
        name: "mcp",
        usage: mcp::USAGE,
        flags: mcp::FLAGS,
        run: mcp::run,
    },
    Command {
        name: "pair",
        usage: pair::USAGE,
        flags: pair::FLAGS,
        run: pair::run,
    },
    Command {
        name: "perf",
        usage: perf::USAGE,
        flags: perf::FLAGS,
        run: perf::run,
    },
    Command {
        name: "plugin",
        usage: plugin::USAGE,
        flags: plugin::FLAGS,
        run: plugin::run,
    },
    Command {
        name: "push",
        usage: push::USAGE,
        flags: push::FLAGS,
        run: push::run,
    },
    Command {
        name: "reports",
        usage: reports::USAGE,
        flags: reports::FLAGS,
        run: reports::run,
    },
    Command {
        name: "secrets",
        usage: secrets::USAGE,
        flags: secrets::FLAGS,
        run: secrets::run,
    },
    Command {
        name: "serve",
        usage: serve::USAGE,
        flags: serve::FLAGS,
        run: serve::run,
    },
    Command {
        name: "setup",
        usage: setup::USAGE,
        flags: setup::FLAGS,
        run: setup::run,
    },
    Command {
        name: "theme",
        usage: theme::USAGE,
        flags: theme::FLAGS,
        run: theme::run,
    },
    Command {
        name: "update",
        usage: update::USAGE,
        flags: update::FLAGS,
        run: update::run,
    },
];
