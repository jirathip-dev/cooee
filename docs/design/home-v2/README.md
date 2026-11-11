# cooee native Home v2: no title bar, floating buttons, search and personas that scroll away (#251)

Home loses the v1 `cooee` bar (`apple/design/v1/home-*`). In every Style, two buttons float over one scroll. The
avatar (opens the You sheet) is top-left and + (new chat) is top-right. The scroll holds the search field, then one
row of all your personas (pinned first), then #229's content: your personas' chats, then Others. Scrolled, the search
and the row are gone and the chats run under the top. There is no `cooee`, `~/cooee` or `Chats` heading in any Style.
Owner decisions: q1321–q1324 and the notes in conductor thread t70.

There are three pages, one per Style, built on the live design system as #229's `home-workflow` was. They link
`web/src/app.css`, `web/schemes/farm*.json` (the same palette as `apple/Sources/CooeeUI/Theme.swift`), #116's
`ns.css` and #125's `sd.css`, plus the farm pack's `theme.json` (its words). The Scene horses are the app's own
sprites (`apple/Sources/CooeeUI/Sprites/horse-<coat>-<pose>.png`). `h2.css` holds this design's additions. `herd.js`
is herdsman's Herd ranch (`assets/herd.js` / `herd.css`, #37/#71), copied as q1323 asks. The only things changed in
the copy are the gate in the fence, the trail, and where each horse stands.

`node docs/design/home-v2/shoot.mjs <chrome-headless-shell>`, run from the repo root, regenerates every PNG under
reduced motion. It checks each shot for sideways scroll, anything past the right edge, and cut text without an
ellipsis. **All 22 pass.** Open a page without a query to see its gallery. `?f=<frame>` shows one frame, and
`?mode=dark|light` sets the mode. Serve the repo root over http. Phone shots are 390 × 844 at 2×. Mac shots are the
window at 1440 × 900 at 1×. **Data:** #124's fixtures, with Writer and Designer added so the row overflows.

## The PNGs and the decision each shows

### Modern (`modern.html`)

| PNG | shows | decision |
|---|---|---|
| `top-390-light.png`, `top-390-dark.png` | At the top. The glass avatar `G` and the glass `+` float over the list, with no title between them. Search, then the personas row: Chief, Coder and Researcher are pinned, then a hairline, then the rest (Account, Conductor, …, which run off the edge and scroll sideways). A ring shows the state (cyan working, red waiting), and an unread count shows where there is one. Then the chats, as #229. | q1321, q1322; owner: no title bar |
| `scrolled-390-light.png`, `scrolled-390-dark.png` | Scrolled. The search and the row are gone, and Coder's row runs under a **glassy top** (blurred, fading out at its lower edge, as iOS 26's scroll edge effect). The two buttons stay. | owner: "the top chats show through a glassy, transparent top" |
| `empty-390-light.png`, `empty-390-dark.png` | No personas. Search, then Others. There is no row and no gap (as #249). | #251 item 5 |
| `top-1440.png` (light) | Mac window. Home is the 390 pt sidebar: traffic lights, then the avatar and + floating, search, the row, and the chats (Coder open). | #251: Mac window |
| `scrolled-1440.png` (dark) | Mac, the sidebar scrolled under its glassy top. | |

### Terminal (`terminal.html`)

| PNG | shows | decision |
|---|---|---|
| `term-top-390-dark.png`, `term-top-390-light.png` | `[g]` and `[+]` in place of the buttons. Search is a prompt line, `$ grep ▌`, with a blinking block cursor. The row is each persona's **ASCII figure** in a ruled box (`web/src/look.rs`'s two frames, `~o=`/`=o~` and so on, swapping on the state's beat), with its name, its farm word (`at the gate` in red) and a square unread count. The chats take the same figures. At the bottom is a **tmux-style status line**: `mini │ load 0.42 calm │ 2 working │ 1 waiting │ 09:41`, with waiting in red. | q1324; owner: `[g]` / `[+]`, no `~/cooee` |
| `term-scrolled-390-dark.png`, `term-scrolled-390-light.png` | Scrolled. The top is the background itself with a dashed rule under `[g]` and `[+]`: no glass and no blur. The status line stays. | #251 item 2, Terminal |
| `term-empty-390-dark.png` | No personas: the prompt, then Others. | #251 item 5 |
| `term-1440.png` (dark) | Mac window in Terminal. The status line runs under the sidebar. | |

### Scene (`scene.html`)

| PNG | shows | decision |
|---|---|---|
| `scene-top-390-light.png`, `scene-top-390-dark.png` | Day, calm. In Scene the personas row **is the Herd strip**, and its sky runs up under the status bar. The buttons are **wooden signs** (nailed planks), and the search field sits on the sky. Each persona acts out its state: Coder and Conductor plough **the trail**, Researcher waits **at the gate** (in the fence, right), Chief stands **in the barn door**, and Account, Writer and Designer **graze** on the hill. The chat rows use the same horses. Dark mode changes only the list; the sky follows the clock, not the mode. | q1323 movement by state; owner: Scene buttons in the theme's look |
| `scene-busy-390-light.png` | **Busy** (the paired Mac's load ≥ half its limit): overcast, more and faster clouds, and a haze. | q1323 load weather |
| `scene-held-390-light.png` | **Held** (the Mac refuses new work): storm sky and rain. | q1323 load weather |
| `scene-night-390-dark.png` | **Night** (19:00–07:00, as herdsman's `auto` light): herdsman's night palette, stars, the moon, the lit barn window and dimmed coats. | q1323 day/night by the clock |
| `scene-scrolled-390-light.png` | Scrolled. The top is a band of the same sky with a **fence rail** along its lower edge, and the signs stay on it. It follows the weather and night, as the strip does. | #251 item 2, Scene |
| `scene-motion-390-light.png` | The motion by state, one strip each, with ghosts where the loop goes. On the trail the horse walks and turns at each end (with a stride bob). At the gate it paws. In the stable it stays still. Grazing, it dips to the grass. Clouds drift with the load, and rain falls when held. | #251 item 3: a frame per state |
| `scene-1440.png` (light) | Mac window in Scene. The strip fills the sidebar's top, under the traffic lights. | |

## Behaviour

- **One scroll.** The search field and the personas row are the top of the list, not a header. Scrolled, they are
  gone. **Pull down** at the top shows search with the keyboard up, as Telegram does. #232's pull to refresh moves to
  pulling past a further threshold, with its spinner under the search field, so the two never fight. A tap on the
  status bar scrolls back to the top and brings both back.
- **The top once scrolled:** glass in Modern (`.glassEffect`, iOS 26's scroll edge), the background and a dashed rule
  in Terminal, and the sky and a fence rail in Scene. The two buttons never move.
- **Pinned first:** pinned personas lead the row in their pin order, then a hairline, then the rest by last activity.
  Pinning stays where it is (the persona sheet). The row is hidden when you have no personas.
- **Motion** builds on #246's sprite frames. Trail walkers move along the strip, a waiting horse paws, and grazers
  dip. Weather comes from the paired Mac's load: **calm** under half its limit, **busy** from there, and **held** when
  it refuses new work. These are herdsman's thresholds. Day and night follow the phone's clock. Terminal's figures
  swap frames, working fastest, and the prompt's cursor blinks. **Reduce Motion stops all of it.** Every shot here is
  the Reduce Motion still: horses on their marks, frame 1 of each figure, and the cursor shown.

## What is reused and what is new

- **Reused:** the farm schemes, `.av`, `.bd`, `.pr` rows, #229's row content and Others, the farm pack's words,
  the app's horse sprites, `look.rs`'s two-frame figures, and herdsman's Herd ranch, palette, clouds, night sky and
  storm.
- **New** (`h2.css`): `.h2b` floating buttons (glass, `[g]`/`[+]`, wooden sign), `.h2t` the scrolled top for each
  Style, `.h2q` search, `.h2p` the personas row with its pinned separator and state rings, `.tmp`/`.tmc` prompt and
  cursor, `.tmf` animated figure boxes, `.tms` the status line, `.hd2`/`.hh2` the Herd strip and its horses.
- **Goes:** v1's `TopBar` title (`cooee` / `~/cooee`), the `# pinned` / `Chats` headings, and the pinned-only row.

## Suggested build issues, once this is approved

1. **Home v2 shell (all Styles):** the floating avatar and + over one `ScrollView`, search and the personas row
   inside it (pinned first, hidden with none), the scrolled top for each Style, and pull-down search next to #232's
   refresh. Snapshot every frame here.
2. **Terminal Home:** the prompt-line search with a blinking cursor, the animated figures, and the status line. This
   needs the paired Mac's load and the working and waiting counts from the server (likely one more field on `/api/v1`).
3. **Scene Herd strip:** port herdsman's ranch to SwiftUI `Canvas` (one `TimelineView` clock). Horses take their
   places by state on #246's frames, with load weather and day/night. Reduce Motion gives the still.

## Open questions for the owner

1. **Where the design lives.** The issue asks for SwiftUI views and snapshots in `apple/design/home-v2/` "as #124".
   This lane's role allows mockups only, with no product code, so this design is HTML on the shared design system in
   `docs/design/home-v2/`, as #229 and #232 were. **Recommend:** build issue 1 writes the SwiftUI views and their
   snapshots, and `apple/design/home-v2/` is created then. The alternative is a design revision that adds SwiftUI
   views now.
2. **Pull to refresh vs. pull-down search (#232).** **Recommend:** pulling down reveals search first, and pulling
   further refreshes. The alternative is to drop pull to refresh on Home, since the list is live.
3. **Pinned separator.** **Recommend:** the hairline after the pinned personas, as drawn. The alternative is no mark,
   so order alone shows the pins.
4. **The status line's machine.** With several Macs paired, **recommend** the one this phone is connected to, which
   is the same Mac that sets Scene's weather. The alternative is the busiest one.
5. **Scene at 390 with many personas.** The strip fits about 8 horses. **Recommend:** past that, the grazers go
   smaller and further back, and the strip never scrolls sideways. The alternative is a sideways-scrolling herd, which
   would fight the list's vertical scroll.
