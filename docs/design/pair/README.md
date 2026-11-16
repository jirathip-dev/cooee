# cooee pairing, redone: design (#47)

Two static pages in the shared design system (#7: `../look/`, Modern by default, every theme, scheme, style and
sprites switch). `pair.css` holds this design's additions: tokens only, plus one raw colour, the QR tile (black on
white in every scheme, so any camera reads it). The pages import `../look/look.js`, which now resolves its themes,
schemes and sprites next to itself (a three-line change; the look's own shots are unchanged).

| page | shots | what it shows |
|---|---|---|
| `pair.html` | `pair-{390,1440}-{dark,light}` | an unpaired device: the welcome, the iOS hint, three steps, the code boxes, the other ways; then its 9 states |
| `approve.html` | `approve-{390,1440}-{dark,light}` | a paired device: the approve card on the list; then the push, approved, declined, the QR sheet, `cooee pair`, settings › devices with passkeys, no passkey here, approved elsewhere, expired |
| both | `pair-terminal-390-light` (garden), `approve-terminal-390-dark` (kitchen), `pair-themeterminal-390-dark` | the Terminal style, and the terminal theme (no sprites: its icon is the figure) |

Each page is the screen first, then a "design sheet" below a rule: one card per state, three across from 1000 px.
Query parameters are the look's: `?theme=…&mode=…&style=…&sprites=…`. Serve the repo root (the pages use
`../look/`, which reads the vendored `packs/themes/`): `python3 -m http.server`, then open `/docs/design/pair/pair.html`.

**Regenerate:** `node docs/design/pair/shoot.mjs <chrome-headless-shell> [name regexp]` (repo root) shoots the 11
PNGs (390 at DPR 2, 1440 at DPR 1, full height, reduced motion) and runs the look's gate-2 audit on each. One
addition to the audit: the code boxes, a shown code and the check word are `--fs-code` (32 px), herdsman's own
pairing-code size. **All 11 pass.** `qr.svg` is a real QR (the `qrcode` crate cooee already uses) of
`https://mac.tailnet.example/pair?code=730518`; the terminal QR is the crate's `Dense1x2` text, as
`cooee pair` prints today.

**Data:** illustrative. Device names, the tailnet name, codes and the check word are made up.

## An unpaired device (`pair.html`)

**The shell only (q528).** The header is `cooee` and the connection dot. No `[+]`, `[◐]`, `[⚙]`, no thread data, no
service worker (as today). The theme and look still apply (the device's own pick, else the server's default theme).

1. **The welcome:** the theme's 132 × 100 `empty` figure (sprites off or the terminal theme: the theme's icon), `pair
   this device`, and one line: `cooee runs on your Mac. Pair this device once and your threads show up here.`
2. **The iOS hint**, above the steps, only in Safari on iPhone and iPad (`navigator.standalone === false` on iOS),
   never in the installed app and never on a desktop: the share icon, `Add cooee to your Home Screen first.`, then
   `Tap [share] Share, then Add to Home Screen, and pair in the app it adds. Safari and the Home Screen app keep
   separate sign-ins.` It sits first because pairing in Safari and then installing leaves the app unpaired.
3. **Three numbered steps** (the owner's q537):
   1. `Open cooee on your Mac or a paired phone`: `On the Mac run cooee pair. On a phone: settings › devices › pair a device.`
   2. `Scan its QR code with the Camera`: `cooee opens here with the code filled in and pairs at once.`
   3. `No camera? Approve instead`: `Tap ask to pair below, then approve on your paired phone with Face ID.`
4. **The code card:** `or type the code`, six boxes (48 × 60, 32 px digits, a gap after the third as codes are
   printed `482 913`), `6 digits · good for 10 minutes · pairs on the sixth`, and `[pair]`, disabled until six digits
   are in. A digit moves to the next box; ⌫ in an empty box goes back; a paste (or iOS's one-time-code autofill on
   the first box) fills all six; the sixth digit sends it, so `[pair]` is for keyboards and screen readers. At
   1440 the card is the right column.
5. **The other ways**, as two settings rows in the card: `no camera nearby? [ask to pair]` and
   `paired here before? [sign in with passkey]`. The passkey row shows only once the server has a passkey.

**Its states** (one card each below the screen):

| state | what it shows |
|---|---|
| typing | three digits in, the fourth box focused (an `--l` ring) |
| from a QR | `/pair?code=482913` opened by the Camera: all six in, green, `pairing…`, nothing to tap; then the list |
| paired · the passkey offer | `✓ paired · iPhone 16`, `Save a passkey? Face ID then signs this phone back in if Safari forgets it, and approves new devices.` `[save a passkey] [not now]`, `later: settings › devices › passkeys` |
| waiting for approval | the theme's `loading` figure, `waiting for approval`, the check word at 32 px (`maple otter`), `Your paired devices got a request. Approve it where it shows maple otter.`, `expires in 4:52 [cancel]`. It pairs by itself when approved |
| wrong code | the boxes in `--r`, `That code isn't right. Check the six digits on your Mac or phone and type them again.` `4 tries left this minute`. The boxes clear and the first is focused |
| expired or used code | the boxes muted, `That code has run out. A code works once, for 10 minutes. Show a new one: cooee pair, or pair a device on your phone.` `[type a new code]` |
| approval declined | `iPhone 16 said no.` `Your request to pair (maple otter) was declined. If that was a mistake, ask again.` `[ask again] [use a code]` |
| offline | `Can't reach your Mac.` `Turn on Tailscale on this device, check that the Mac is awake, then try again.` `[try again] tried 0:08 ago` |
| too many tries | the boxes disabled, `Too many tries. Five wrong codes a minute is the limit. The boxes open again in 0:42.` (5 per minute per IP) |

A passkey sign-in that finds no passkey for this cooee says, in the same red card: `No passkey for this cooee on this
device. Pair with a code instead.`

## A paired device (`approve.html`)

- **The approve card**, at the top of the thread list on every paired device while a request is open:
  `Pair a new device?` in `--a`, the age, `iPad · Safari, over your tailnet` (the label from its user agent),
  `check word maple otter`, `[approve with Face ID] [decline]`. Approve runs a WebAuthn assertion with user
  verification against this device's passkey; the server pairs the new device only on a valid one. The label
  follows the platform: `approve with Touch ID` on a Mac, `approve with passkey` elsewhere.
- **The push**, to every paired device whatever its push toggles: title `Pair a new device?`, body
  `iPad · Safari · check word maple otter`. A tap opens cooee at the card.
- **Approved:** `✓ iPad paired`, `It's in settings › devices. The other devices' cards cleared.` `[rename it] [done]`.
  **Approved elsewhere:** `✓ iPad paired on iPhone 16`, and the card clears by itself.
- **Declined:** `Declined · iPad`, `It won't be paired, and it sees "iPhone 16 said no".` `[undo]` for 10 s.
- **The request ran out** (5 minutes, nobody answered): `iPad's request expired`, `It can ask again.`
- **No passkey on this device:** the same card, with `Approving needs a passkey. Save one on this device, or approve
  on iPhone 16.` and `[save a passkey] [decline]`.
- **settings › devices › pair a device:** the QR (216 px tile), `Scan it with the new device's Camera, or type the
  code there.`, the code at 32 px (`730 518`), `one use · expires in 9:41 · <tailnet name>`, `[done] [new code]`. The
  QR encodes `https://<tailnet name>/pair?code=<code>`. A new code drops the old one.
- **`cooee pair` on the Mac:** a line of what to do, the QR, `code 482 913 (one use, 10 minutes)`, `link …`, then it
  waits and prints `✓ paired: iPad · Safari` when the code is used (Ctrl-C leaves the code open until it expires).
- **settings › devices:** the devices as #7 drew them, then **passkeys**: one row per passkey, the device that made
  it, `made 6 Nov · used today`, `[remove]`.

## Open questions for the owner

1. **Step 3 reads "No camera? Approve instead".** The spec has the QR pair at once (scope 2, AC "a QR code →
   prefilled and paired"), so approval is the other path, not a step after scanning. The alternative: every pairing,
   QR or code, waits for a Face ID approval on a paired device (safer if a QR is photographed; one more tap).
   **Recommend: as drawn.**
2. **Showing a QR on a paired phone: Face ID first?** As drawn, `pair a device` shows the QR at once (the phone is
   unlocked and paired). With a passkey, it could ask Face ID first. **Recommend: no, as drawn.**
3. **Who is offered a passkey?** The spec says the first paired device. Approving needs one, so a device without one
   can only decline. **Recommend: offer it after every pairing** (the same card); the first device is the one drawn.
4. **The check word:** two words from a fixed list (`maple otter`), not digits, so it can't be mistaken for a code.
   **OK?**
5. **A request lasts 5 minutes** (codes last 10). **OK?**
6. **The terminal QR in a dark terminal** shows light modules on dark (as today's `cooee pair`). The iPhone Camera
   usually reads it; impl checks with a real phone and, if not, prints it inverted.
7. **iOS 26 Safari** keeps Share in the `⋯` menu in its compact tab bar. The hint says `Tap [share] Share`; add
   `(in the ⋯ menu)` on iOS 26? **Recommend: yes, by user agent.**
8. **Declined has `[undo]` for 10 s.** New; it saves a mis-tap. **OK?**
