# First-party plugin: terminal

## How it works

`crates/cooee-terminal`, built into cooee: a real shell on the Mac from the phone, for the owner-only commands a chat
hands over (setup, keychain, `launchctl` …), instead of an SSH app. `cooee setup` adds its entry to each instance on
the instance's port + 1000 with a fresh token:

```toml
[[plugins]]
name = "terminal"
label = "Terminal"
url = "http://127.0.0.1:5747"
module = "/tab.js"
token = "a-third-long-random-secret"
```

- **Shells:** [+ shell] starts `$SHELL -l` in `~`, as the owner, in a shed terminal labelled `cooee.owner=1`, so it
  outlives the page and cooee. The list has the owner's shells, then the agents' terminals (every other shed
  terminal but sign-ins), which can be watched but never typed into (403). [close] ends a shell's whole process tree.
- **View:** shed keeps the screen (`screen.watch`); the backend streams its updates (rows of coloured runs) as
  `screen` events, and the tab draws them as a monospace grid in the theme's colours. Pinch zooms the font, and the
  shell is resized to fit; [earlier lines] shows the scrollback. No terminal emulator runs on the phone.
- **Input:** the keyboard's text goes in as text (Thai too), in order; the key bar at the tab's foot, on the keyboard
  while it is open: Esc, Ctrl (sticky for the next letter), Tab, ↑ ↓ ← →, ^C, paste and ↑ history (the lines typed
  here, to type again).
- **Gate:** the device's passkey (Face ID) opens the tab, registered on first use; 10 minutes without a touch or key
  lock it (blurred, no input) until Face ID again, and the shells keep running ([plugins.md](plugins.md)).
- **Insert text:** `POST /insert` `{"text"}` (through the tab's proxy) types into the shell used last, else the
  newest, else a new one, and never presses Enter: trailing line ends go, a `\` continuation joins its lines and any
  other line end becomes `; `. It answers `{"terminal"}`. A chat's code block has **[▸ terminal]** beside [copy] when this plugin is on
  (#29): the owner's tap opens the Terminal tab, which asks for Face ID first when locked, then inserts the block's
  text and opens that shell (the tab's `#name`).

## API

Its other calls: `GET /list`, `POST /shell`, `GET /watch?t=<name>` (the stream), `POST /input` `{terminal, text}`,
`/keys` `{terminal, keys}` (shed's key names), `/resize` `{terminal, cols, rows}`, `/close` `{terminal}`,
`/scrollback` `{terminal}` and `/seen` (use, which keeps the unlock alive).
