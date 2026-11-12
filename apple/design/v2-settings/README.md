# cooee native v2 settings: Sign-ins, Devices, Machines + Terminal, Plugins (#239)

The four avatar-menu rows that #124 left as "v2" (q955), drawn natively. Each copies the content of the working web
screen (q1301). Like `design/v1`, these are real SwiftUI views in `Sources/CooeeUI/` (`SignIns.swift`, `Devices.swift`,
`Machines.swift`, `MachineTerminal.swift`, `PluginsPage.swift`, shared parts in `SettingsParts.swift`). They are fed by
fixtures, with no reducer and no networking yet. Each build issue adds the screen's reducer and client and swaps
`MenuPage`'s "This opens in v2." for its view. The PNGs are stills (`Look.still`), 2×: the phone at iPhone 17
(402 × 874 pt) and the Mac window at 1180 × 780. `CooeeUITests`' `SnapshotTests` now checks `design/v2-settings/`
as well as `design/v1/` (`Frame.folder`). Sign-ins and Devices are the v1 sign-in and pairing designs (`mac-setup-signins`, `pair-*`, `mac-setup-approve`) drawn as settings pages (q1317).

## iPhone 17, dark and light

| PNG | shows | copies | decision |
|---|---|---|---|
| `sign-ins-*` | One group per provider and one row per account (its CLI config folder). Each row is signed in (green), with its usage windows as the provider's own module names them (`session · week · week Opus`, `primary · weekly`, never a fixed 5 h / 7 d) and Sign out, or Sign in…. The default account is marked. One account is mid-sign-in: the CLI's own login runs in a **shed terminal**, its output is shown inline, then the code field with Send / Cancel. Add an account. A provider that isn't installed gets Copy install. | web Settings › providers (#30, #48, #183) | tokens never touch cooee: the login is the CLI's own, in shed; q1377: each provider's own windows, no "Where logins live" line |
| `devices-*` | A pending pairing on top: who and how (`Pixel 9, over your tailnet`), the **check word** `maple`, Decline / Approve · Face ID. Then the paired devices (name, paired, last seen), with **This device** marked: Unpair for this one, Remove for the others. Pair a device (its QR and code, as `mac-setup-qr`) and Passkeys. | web pairing cards and Settings › devices (`pair.ts`, `chat.ts`) | approve shows the same word the new device shows |
| `machines-*` | One card per Mac from shed: online, or offline with its last seen time. Load/cpu, memory, swap and disk free, each with its 24 h sparkline. Alerts in red on the card (`memory critical · disk low`) and on the bad numbers. Tapping a number opens its 24 h chart (studio's memory: `low 38% · high 94%`). Terminals by state. **Terminal ›** on each online machine. | web Machines tab (#32, `cooee-machines/src/html.rs`) | |
| `plugins-*` | Installed plugins: name, version, commit, on/off, state, Not reviewed (not from the index), Update to x.y when one is out (else Update), Logs, Remove. **Enable per persona**: the persona avatars, checked where attached, with their names. Then Add from the index (Add… shows what will run, to confirm) and Add from a git URL…. | web Settings › plugins (#37, #129) | plugins attach to a persona (q954) |
| `terminal-dark` | A machine's terminal (Terminal › on `mini`): its shed terminals as tabs (`coder · cooee`, `chief · work`, `zsh`), New shell, the terminal itself, and a key row (esc, tab, ctrl, arrows, ⌃C) over the keyboard. | shed terminal | q955: Terminal in v2 |
| `you-terminal-dark` | You › **Terminal** (a row under Settings in the avatar menu, `design/v1/avatar-menu-*`): your own shell on the paired Mac, no agent. Closing it leaves it running as a shed session; reopening reattaches with the scrollback above. | shed terminal | q1319, q1320 |

## Styles (q947)

| PNG | shows |
|---|---|
| `sign-ins-terminal-dark.png` | Terminal style: SF Mono, `# claude code` headers, ruled boxes, bracketed actions (`[sign out]`, `[sign in…]`, `[default]`). |
| `machines-scene-light.png` | Scene style: the farm cards with their edge, the same numbers and chart. |

## Mac window

| PNG | shows |
|---|---|
| `mac-sign-ins.png`, `mac-devices.png`, `mac-machines.png`, `mac-plugins.png` | Settings as a sidebar (the avatar menu's seven rows, Terminal included) beside the page. Machines lays its cards out in a grid. |
| `mac-plugins-light.png` | The same, light. |

## Open for the build issues

- Removing a device and removing a plugin confirm first, as the web does. The confirm sheet is native and not drawn.
- Plugins' Logs, Improve and Roll back (web #129) open as sheets. Only Logs is drawn on the row; Improve and Roll back
  sit in the same row on the plugins that have them.
- The Mac app opens Terminal › as a window, not a sheet.
