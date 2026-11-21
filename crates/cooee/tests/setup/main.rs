//! `cooee setup` with fake `shed`, `launchctl`, `systemctl`, `tailscale` and `curl` on `PATH` (each logs its arguments to
//! `$HOME/calls.log` and keeps its state in `$HOME`) and a fake shed server for doctor's hello: a fresh machine runs
//! every step once, in order; a second run skips them all; `--dry-run` writes nothing; a doctor failure stops with its
//! fix; an existing config keeps its keys; a shed below the minimum protocol stops setup before it changes anything.
//! Also `scripts/install.sh`, and `cooee update` (#332, #584) against fake GitHub releases (`curl`), with the fake
//! `launchctl kickstart` (`systemctl restart` on Linux) serving the installed binary that `curl` asks on the port.

#[path = "../support/mod.rs"]
mod support;

mod install;
mod setup;
mod shed;
mod steps;
mod themes;
mod update;
