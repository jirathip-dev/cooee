# cooee app icons: 6 sets of 4, the icon follows the look, a picker override (#225)

There are 24 iOS and macOS app icons. They are flat vector, in the look of Corral's horses (its herd-art Treatment A):
a head-and-shoulders portrait facing left, flat fills with soft shading (black or white at low opacity), and one
solid dark background per set. The icon **follows the look** (Style, and for Scene the theme), and Look → App icon
lets you **pick any of the 24 instead**. Owner decisions: q1281–q1289.

The pages are built on the native v1 look (`apple/design/v1/`). `ai.css` copies `Theme.swift`'s `Palette.farmDark`
and `.farmLight` token for token. Its groups, rows, uppercase headers and checkmarks follow `Sheets.swift`'s `Group2`
and `SettingRow` and `Menu.swift`'s `pick` row: 48 px rows, a 16 px group radius, 14 px row padding, and the
checkmark in `link`. The only new pieces are the icon squircle, the picker's four-tile grid and the two wallpapers.

| file | what |
|---|---|
| `icons/` | The 24 icons. `farm-*.png` are Corral's four Treatment-A horses at 1024 × 1024 opaque RGB. Palomino and Grey are byte-identical to Corral's shipped masters. Bay and Black are re-rendered on dark grounds (q1289). The other 20 are `<set>-<name>.svg` at 1024 × 1024. |
| `farm.py` | Renders the four `farm-*.png` with **Corral's own generator**: `uv run --no-project --with pillow python docs/design/app-icons/farm.py <corral checkout at 060220c3>`. It uses `gen_icons.py` from Corral's #462 design gate and the canonical `horsesvg.py`, with the same crop, pose and supersampling, and changes only the `BG` of each coat. It fails if Palomino or Grey drift from Corral's shipped bytes, so the pipeline is proven identical. |
| `art.mjs` | Draws the 20 SVGs: `node docs/design/app-icons/art.mjs`. |
| `icons.html`, `picker.html`, `ai.js`, `ai.css` | The two pages. `?f=<frame>&mode=dark\|light` shows one frame. |
| `shoot.mjs` | `node docs/design/app-icons/shoot.mjs <chrome-headless-shell> [regexp]` regenerates every PNG, each at full height (390 wide at 2×, 1440 at 1×). It checks each shot for sideways scroll, anything past the right edge, and text cut without an ellipsis. **All 9 pass.** |

## The 24 icons

The first icon of each set is the one the look shows when you follow it.

| set | background | 1 (followed) | 2 | 3 | 4 |
|---|---|---|---|---|---|
| **Modern**: cooee's kookaburra | navy `#13263d` | `modern-laughing`: **Laughing**, beak open, eyes shut, laugh lines | `modern-perched`: **Perched** on a gum branch | `modern-glance`: **Side glance**, a wide eye looking back, brow up | `modern-wings`: **Wings out**, blue wing patches |
| **Terminal**: prompt glyphs in a window | `#0d1117` (github-dark's `--bg`) | `terminal-prompt`: **Prompt** `>_` in green | `terminal-dollar`: **Dollar** `$` in cyan | `terminal-chevron`: **Chevron** `❯` in purple | `terminal-lambda`: **Lambda** `λ` in blue |
| **Farm**: Corral's horses | night pasture `#1f2a38` (Bay, Palomino), deep pine `#22302a` (Black, Grey): Corral's two dark grounds | `farm-bay`: **Bay** | `farm-palomino`: **Palomino** | `farm-black`: **Black** | `farm-grey`: **Grey** |
| **Garden**: the shop's plants | leaf `#1c3324` | `garden-monstera`: **Monstera**, the garden theme's own icon leaf | `garden-sunflower`: **Sunflower** | `garden-cactus`: **Cactus** in flower | `garden-tulips`: **Tulips** |
| **Kitchen**: the line's cooks | espresso `#2a1f1a` | `kitchen-head-chef`: **Head chef**, toque and moustache, the pack's blue neckerchief | `kitchen-pastry`: **Pastry chef**, pink cap and whisk | `kitchen-line-cook`: **Line cook**, red bandana and beard | `kitchen-sous`: **Sous chef**, paper cap, glasses and ladle |
| **Bush**: drawn now, reused by #22 (q1283) | red earth `#3b2318` | `bush-roo`: **Roo** | `bush-wombat`: **Wombat** | `bush-koala`: **Koala** on a gum branch | `bush-emu`: **Emu** |

## The PNGs and the decision each shows

### `icons.html`: the sheet

| PNG | shows | decision |
|---|---|---|
| `icons-1440.png` | **Follow the look** on the left. A fresh install shows Laughing, Style Modern shows Laughing, Terminal shows Prompt, and Scene shows its theme's first (farm Bay, garden Monstera, kitchen Head chef, bush Roo once #22 ships). Next to it are all 24 at home-screen size (60 pt) on a **dark** and a **light** wallpaper. | q1281 (follow), q1288 (Laughing on a fresh install). Each set's dark background keeps the icons readable on both wallpapers. |
| `icons-390.png` | The same 24 on both wallpapers, at phone width. | They read at 60 pt: the silhouettes hold without the detail. |

### `picker.html`: Look → App icon

| PNG | shows | decision |
|---|---|---|
| `look-390-dark.png`, `-light.png` | **Look** (the native app's Appearance, reached from the avatar menu's Settings) keeps Scheme and Style and gains an **App icon** group. Its one row shows the current icon in place of `SettingRow`'s tinted symbol, with the value `Follows the look` and a ›. | The picker sits under Look, beside the Style it follows, rather than on a new Appearance screen. |
| `picker-390-dark.png`, `-light.png` | **App icon**, while following. The top row is **Follow the look** ✓, with the icon it shows now (`Now: Laughing · Modern`). No paragraph under it (q1289). Then each set is a group of **four tiles** (a 64 pt icon and its name), the same grid as Corral's `AppIconPicker`. The icon the look shows now has a **dashed ring**, which means "following, not picked". | Following is one row at the top, not a 25th tile. The dashed ring shows what you get without picking anything. |
| `picked-390-dark.png`, `-light.png` | **Koala picked.** The Koala tile has a **solid ring in `link`, a ✓ badge and its name in `link`**. Follow the look loses its ✓ and reads `Off · would show Laughing · Modern`. There is no footnote. | A picked icon stays until you choose Follow the look again (scope 3). |
| `picker-1440.png` | The three frames side by side: Look, following (dark), and Koala picked (light). | |

**Set order in the picker:** Modern, Terminal, Farm, Garden, Kitchen, Bush. That is the order the look resolves
them: the two Styles first, then Scene's themes. Modern leads because it is the fresh-install icon.

## For the impl lane (not drawn)

- **Asset names (proposed).** The primary `AppIcon` is Modern Laughing, so a fresh install and a `nil`
  `alternateIconName` both show it. The 23 alternates are `<Set><Name>` (`ModernPerched`, `TerminalPrompt`,
  `FarmBay`, …, `BushEmu`). As in Corral (#464), the picker's previews are regular imagesets (`<Name>Preview`),
  because iOS 18 and later does not serve appiconset renditions to `UIImage`.
- **SVG to PNG.** iOS needs 1024 × 1024 opaque PNGs. Render each SVG at 1024 with no alpha. The Farm PNGs are
  already in that form.
- **Errors** reuse Corral's copy: "Couldn't change the Home Screen icon. Tap Koala to try again."
- **iOS confirms every switch** with its own alert ("You have changed the icon for cooee"). When the icon follows the
  look, a Style change triggers that alert. That is expected, and it is the system's alert, not ours.

## Open questions for the owner

1. **The native app has no Theme setting yet.** Look has Scheme and Style only, and Scene draws the farm. Until a
   Theme exists, following the look in Scene always gives Bay. Garden, Kitchen and Bush can only be picked by hand.
   Should #225 add a Theme picker to Look, or should following wait for the issue that brings themes to native?
2. **Black on a dark ground is low contrast** (q1289). Its coat `#3b3b44` against deep pine `#22302a` measures 1.24:1, and against any dark ground it stays under 1.7:1. Deep pine reads best of the candidates (night pasture, `#121a26`, `#0f141c`), because the green separates from the cool grey coat by hue, and the pink muzzle and ear carry the silhouette at 60 pt. If that is still too dim, a tweak to Black's coat or mane is the alternative, which would no longer be Corral's art as-is.
3. **macOS.** Does the Mac app get the picker too? A running app can change its Dock icon, but its Finder icon stays
   the primary. The recommendation: the Mac shows the same art and follows the look in the Dock, with no picker.
