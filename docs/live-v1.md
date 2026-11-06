# v1 live: the plain `cooee` instance on the Mini

The owner runs the install commands (they change the host); the conductor confirms with `cooee doctor` and records
each checklist result in the v1-live PR. `cooee-drover` (the herdsman plugins, jirathip-dev/cooee-drover) is set up
later the same way with its own port and `[[plugins]]` entries.

## Install

1. `cooee install --instance cooee --port 4747` writes the plist and `~/.config/cooee/cooee/config.toml` (`port`, and
   `allowed_hosts` with the tailnet name when Tailscale is up), and prints the commands below.
2. Run the printed lines yourself:
   - shed's install line, if it printed one (shed isn't running);
   - `launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/dev.cooee.cooee.plist`;
   - `tailscale serve --bg --https=4747 http://127.0.0.1:4747`.
3. `cooee doctor` (default instance): every line `ok`. Its last line names the tailnet URL,
   `https://<mini>.<tailnet>.ts.net:4747/`. The log is `~/.local/share/cooee/cooee/cooee.log`.

## Phone checklist (iPhone)

| # | Check | Result |
|---|---|---|
| 1 | Open the tailnet URL in Safari; Share → Add to Home Screen; open it from the Home Screen. | |
| 2 | Pair: `cooee pair` on the Mini, scan the QR code or enter the code; the thread list shows. | |
| 3 | Start a thread with Claude Code (through shed) and get a streamed reply. | |
| 4 | See a tool row, and approve one tool. | |
| 5 | Turn on notifications, lock the phone, get a push when the agent needs you, tap it into the thread. | |
| 6 | Do a provider login from the phone (M3). | |
| 7 | Switch Style and Sprites in Appearance. | |
| 8 | Type Thai in the composer and send it. | |
