## Summary

Decision: cooee gets a one-page site, written by blocks' own static site renderer: a hero line beside a live chat, six one-line features each drawn as a real cooee screen, a themes showcase, install for the app and the server, and a footer. A switcher at the top restyles the whole page live in any style (Modern, Terminal) or pack (farm, kitchen, garden, bush), in any of swag's 14 colour schemes; a pack also puts its scene behind the hero.
Screens: the one page, at 1440 and 390, in three looks: Modern (the default), the farm pack and Terminal in Nord.
Open questions: (1) blocks sections carry no HTML `id`, so the nav's in-page links (#features, #install) need a small blocks change, filed against blocks by impl, or the nav keeps only GitHub and docs; (2) the TestFlight link needs the public beta URL.

# cooee site (#826)

The page is not hand-written HTML. `gen/` is a design-only Rust program that builds one blocks `Screen` and hands it to
`blocks::site::page` (the same library function blocks' own site uses), pinned to the blocks and swag commits cooee's
`Cargo.toml` pins. `page/` is its output, as it would be deployed.

## Images

| Image | Shows |
|---|---|
| `site-modern-1440.png`, `site-modern-390.png` | The default look (Modern, cooee's plain pair). Sections in order: top bar with the look switcher, hero (the line, Install and GitHub, a live chat beside it at 1440, under it at 390), Features (six, one line each, each with its screen: 3 across at 1440, 1 at 390), Themes (a chats screen in each look), Install (the app card and the server card), footer. |
| `site-farm-1440.png`, `site-farm-390.png` | The farm pack picked: every token on the page changes to the pack's scheme, the scene style's labels apply, and the pack's scene art fills the hero backdrop. |
| `site-terminal-nord-1440.png`, `site-terminal-nord-390.png` | The Terminal style in Nord: the scheme row under the switcher picks any of swag's 14 schemes for the chosen style or pack, so every combination (6 × 15) is one tap away. |

## Decisions, by section

1. **Hero:** blocks `hero` (title `cooee`, one line, Install and GitHub) next to the chat screen as `screens/chat.rs`
   builds it: a `message` from You, Planner's reply with its `tool` part and prose, then the `composer`.
2. **Features**, one line each, each with the screen the product draws:
   - Chat and personas: the sessions screen (`screens/sessions.rs`), a section per persona.
   - Chief routing: a Chief chat whose `route` tool hands off, with the link row chat.rs writes for it.
   - Plugins like drover: drover's queue as cooee shows the plugin's screen.
   - Machines: Settings › Machines (`settings.rs` `machine_rows`), online state and tags.
   - Modern terminal: the terminal plugin's screen. It is no blocks screen (shed's cell runs, drawn by the plugin's
     `tab.js`), so the site carries it as a blocks `island` named `cooee-terminal` whose props are a cell-run fixture
     and whose fallback is the text; cooee already uses `island`. Impl draws it with the plugin's own row code.
   - iPhone and Mac apps: Settings › Devices (`device_rows`), the phone marked "this device".
3. **Themes showcase:** one small chats screen per style and pack, each scoped to its own scheme pair, packs on their
   scene. The switcher (Look row: Modern, Terminal, Farm, Kitchen, Garden, Bush; Colours row: "its own" plus every
   swag scheme) uses blocks.js's own look switching: each choice is a named look (`farm`, `terminal~nord`) in the
   page's `data-looks`, worn by a `look:` button. No custom switcher code.
4. **Install:** The app card: TestFlight, and the Mac `.dmg` shown as "with the next release" until #825 ships.
   The server card: the one-line install and `cooee setup`, exactly as the README has them today (impl checks them
   against `docs/install.md` and `scripts/install.sh` once #825 lands), and a link to `docs/install.md`.
5. **Footer:** GitHub and Docs.

**Only blocks kinds cooee's screens already use** on the page itself, plus `hero` and `nav`, which are the static
site's page kinds (blocks' site uses them the same way). **CSS:** besides `blocks.css`, one `site.css` the renderer
takes, as blocks' own site has: the grid (hero two-up, features three-up at 1440), the scene backdrops per pack,
and the scoped scheme tokens per showcase tile, generated from swag's data. No colour is hand-picked except the
terminal's background, which is the plugin's own.

**Fixtures:** made-up personas (Planner, Cook, Coder, Chief), chats, machines ("This Mac", "Linux box") and repo
issues. No host names, home paths, private names or real data.

## For impl

- The screens in `gen/src/screens.rs` are hand-shaped to match each builder's current output. Impl replaces each
  with a call to the builder on a fixture hub (the AC's test: a site screen must come from a builder), which needs
  cooee-server to expose its screen builders to the site build. The layout, copy and switcher stay as drawn.
- Blocks' check needs block ids that are unique across the page, but every chat builder names its log `log` and its
  composer `say`. Two chat screens on one page clash, so the site build must give each embedded screen its own id
  prefix. The design does that by hand for the Chief chat.
- A composer is sticky in blocks.css and covers the last message inside a framed screen; `site.css` makes it static.
- Every look (6 × 15) is inlined in the page's `data-looks`, so `index.html` is about 290 KB. Impl may trim it to
  the looks the switcher offers.
- In the Terminal style the buttons read `[ [ Modern ] ]`: blocks.css's default brackets plus the terminal style's
  own. That comes from blocks and swag, as the app would draw it; the design leaves it.
- The looks come from `swag::schemes()`, `swag::packs()` and cooee's `packs/scene` and `packs/terminal`
  `style.json` (the keys blocks' `Style` doesn't know are dropped, as cooee's own look code does).

## Rebuild

```sh
cd docs/design/site/gen
CARGO_TARGET_DIR=$DROVER_SCRATCH_TARGET/target cargo run --offline -- ../page <blocks checkout>/web/dist
cd .. && node shoot.mjs <chromium binary>
```
