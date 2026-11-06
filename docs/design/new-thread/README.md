# New thread is a composer; providers is a flat list; the chat opens on send (#48)

Three static pages on the app's own look. **Revised for the #48 amendments:** no dev text, the agent chip carries
profiles (#40), and from 1024 px a sidebar that's always there (§ 7). The pages load `web/src/app.css`, `web/src/look.js`, `web/themes/` and
`web/schemes/` as they are today; nothing is copied. `new.css` holds this design's additions (tokens only, no raw
colour). `mock.js` builds `LOOK` the way the server does, and draws the one state look.js doesn't have yet (loading).
`side.html` is the sidebar, one copy that `mock.js` puts in each page's `<aside class="side">`.

| page | screens | shots |
|---|---|---|
| `composer.html` | 1 the new-thread composer · 2 the agent sheet · 3 the folder sheet · 4 another folder: the chip on its last-used profile | `composer-390-r2`, `composer-1440-r2` |
| `chat.html` | 4 the chat right after send, connecting · 5 connecting failed, with retry | `chat-390-r2`, `chat-1440-r2` |
| `providers.html` | 6 Settings › providers | `providers-390-r2`, `providers-1440-r2` |

The main shots are farm, Modern, dark. Each page is also shot at 390 in `-kitchen-light`, `-garden-dark` and
`-terminal-light` (the terminal theme in the Terminal style), and the chat once more at 1440 in `-terminal-light`
for the Terminal sidebar: 16 PNGs in all. Every name ends in the round (`-r2`, `REV` in `shoot.mjs`), so a revised
shot never shows up as an older, cached copy. The 390 shots are DPR 2 and the 1440
shots DPR 1, all under reduced motion. Each screen is one phone screen (844 px) or one desktop screen (900 px), stacked
on the page under a caption.

**View or regenerate** (from the repo root, no deps):
- `node docs/design/new-thread/shoot.mjs --serve` prints the URL. Pages take `?theme=farm|kitchen|garden|terminal`,
  `&mode=system|light|dark`, `&style=modern|terminal`, `&sprites=on|ascii|off` and `&s=<screen>` (one screen alone).
- `node docs/design/new-thread/shoot.mjs <chrome-headless-shell> [name regexp]` shoots the 16 PNGs and checks every
  screen against the size scale below: no sideways scroll; every chip, button and one-line input is `--ctl` tall, has
  the `--r-ctl` radius and `--fs` text (inputs 16 px); every sheet and Settings row is `--row`; cut text is
  ellipsised; at 390 the composer fits one screen; from 1024 px the sidebar is shown (320 px, its rows `--ctl`) and
  the header's `[+]` isn't, below it the reverse; and no dev text (`…-acp`, `System default`) is anywhere on the page.
  **All 16 pass.** The impl's screenshot test can reuse `audit()`.

**No dev text.** Nothing on screen names an agent id (`claude-acp`, `codex-acp`) or says `System default`. Chips,
sheets, the sidebar and the thread bar show the provider's name (`claude`, `codex`, `gemini`) and the account, or a
profile's label. **An account** reads as its email when its CLI reports one, else its cooee name (`lab`). The
system-default login has no name: with no email reported it reads `main login` (open question 8). The captions over
each frame are the mockup's, not the UI.

**Data:** illustrative. The accounts, folders, the message and the error text are made up.

## 1. New thread = composer

- `[+]` (from 1024 px the sidebar's `[+ new thread]`) opens **`/new`**, not the `#new` card on the list. The view has the header, a thread bar `‹ new thread`, an
  empty space, and the composer. The composer is the chip row `[claude · jk@work.com ▾] [cooee ▾]`, then the box
  `message claude` and `[send]`. Nothing else: no agent radios, no account selects, no folder list, no sign-in link.
- **Phone:** the composer sits at the bottom, where the chat's composer will be, so send changes nothing under your
  thumb. It fits one screen with the keyboard up or down. **From 1024 px:** the sidebar is on the left and the
  composer is a 720 px card in the upper third of the column beside it.
- **The agent chip** (the profile chip of #40) reads `<provider> · <account>` for Plain (`claude · jk@work.com`), or a
  named profile's label (`drover conductor`, screen 4). Until #40 lands there are no named profiles, and it is
  always Plain's `<provider> · <account>`. **The folder chip** reads the folder's own name (`cooee`); its full path is in the chip's title and
  the sheet. Two full paths don't fit one row at 390 in the Terminal style.
- A chip whose sheet is open gets a 2 px `--l` ring (`aria-expanded`).
- The box's placeholder names the provider (`message claude`), as the chat's does, for a profile too.

## 2. The agent sheet and the folder sheet

Both are the app's sheet: from the bottom on a phone (as tall as the list, at most 80 %, over the scrim), and from
1024 px a 420 px panel under its chip with no scrim. The rows are `.sr` rows, 44 px. The current pick has the `✓`.

- **Agent:** first **profiles** (#40), each its label with, on the right, its provider and, for a plugin's profile,
  the plugin's name (`claude · drover`). The section is left out when there are none. Then Plain's accounts: one
  section per installed provider (`claude`, `gemini`), each with its accounts. A tap on a profile or a signed-in
  account sets the chip and closes the sheet. A signed-out account is muted, reads `signed out`, and has `[sign in]`
  on its row. The sign-in runs in today's terminal panel, which opens under that row, and once it ends, the account
  is picked. Providers that aren't installed aren't listed. The foot links to Settings › providers
  (`add an account or a provider in settings ›`).
- **Folder:** the search box and `[+ new folder]` (in the folder picked) on one row, then **recent** (the folders
  that have threads, newest first) and **all folders** (the roots and one level in; search finds two levels, as
  today). Each recent folder shows its last-used pick on the right (`claude · jk@work.com`, or `drover conductor`), so
  you can see what the agent chip will switch to.

## 3. Default = last used, per folder

- The agent chip starts at the pick of the **newest thread in the chosen folder**: a profile, or Plain's provider and
  account. Picking another folder moves the chip to that folder's last-used pick (screen 4: `~/Projects/herdsman`
  last used `drover conductor`). A folder with no threads takes the newest thread's pick overall. With no threads at
  all, it takes Plain's first signed-in account (in Settings order).
- This needs no new setting or storage: the thread store already keeps each thread's agent, account and folder, so
  the server works out the defaults and the composer page gets them as data (folder → profile, or
  `provider · account`); #40 adds the thread's profile to what the store keeps. The same
  pick shows on every device.
- If the last-used account is now signed out or gone (or the profile removed with its plugin), the chip takes the
  next rule's pick instead. It never starts on a signed-out account.
- `default for new threads` and `[make default]` leave Settings, along with `Account.default` as a pick. The system-default
  login is still listed as an account, by its email (see "No dev text").

## 4. The chat opens on send

- `[send]` on `/new` opens `/t/<id>` at once. The page is drawn from what the device already has: the title (the
  message's first line until the automatic title arrives), `claude · jk@work.com · ~/Projects/cooee` (a named
  profile: `drover conductor · ~/Projects/herdsman`), and your message as the first item. From 1024 px the thread is
  on top of its folder in the sidebar at once, its dot amber while it connects and red if it fails.
- Where the reply will come there is the **connecting line**, in the usual state format:
  `[sprite] saddling up · connecting claude · jk@work.com` in farm, `firing up` in kitchen, `watering` in garden. The
  terminal theme says plain `↻ connecting claude · jk@work.com`. The sprite is the theme's 132 × 100 loading figure
  drawn at line height (32 × 24). With sprites off it is the dot, and in ascii `o.o`. The word is amber like
  working, and the rest is muted. The thread bar's state reads the same word. The agent's first item replaces the
  line.
- **A failed connect** replaces the line with a card that has a `--r` border (a `--r` bar in Terminal):
  `[fail sprite] refused · couldn't connect claude · jk@work.com`, the reason on its own line (`claude stopped:
  Invalid API key`: the agent's own message, with any agent id in it put as the provider's name and no exit code),
  and `[retry]`. Your
  message stays where it was and is not marked failed. `[retry]` connects again and sends the same message. The
  composer stays usable.
- For impl, `look.js` gains a `loading` state for `[data-st]`, drawn the way `mock.js`'s `fillLoading` draws it.
  Today look.js would draw the 132 × 100 figure in a 24 × 24 box.
- **Timing:** today `POST /threads` runs `hub.create` (login home, agent spawn, ACP session, first prompt) before it
  answers, and only then does the page navigate and load `/t/<id>`. The impl measures each step (send → POST answered
  → page loaded → first item drawn) before changing anything and puts the numbers in the PR. The intended shape: the
  server stores the thread and the message, answers with the id, and connects in the background. The connecting and
  failed states reach the page over the thread's event stream.

## 5. Settings › providers = flat list

- Settings' `providers ▸` row opens **providers** in the Settings sheet (`‹ providers ×`), in place of the
  `#signins` card on the list page.
- **One row per account:** the provider tag, the account (its email, else its name), `✓ signed in` with
  `[sign out]`, or `signed out` with `[sign in]`. **Each installed provider ends with** `[provider] [+ account]`. **A
  provider that isn't installed is one row:** `[codex] not installed [copy install]`. The button copies
  `npm install -g @openai/codex` and reads `copied` for a moment.
- Order: installed providers in the app's order (claude, codex, gemini), each with its accounts (the system-default
  login first), then the ones that aren't installed.
- The provider tag is a quiet pill (fg at 8 %) in Modern and plain `--p` text in Terminal. It has no brackets there,
  because Terminal's brackets mean a button.
- The sign-in's terminal panel opens under the row, as today. The per-account line of the last sign-in and usage goes
  away, because the flat row has no room for it (see the open questions).

## 6. The size scale

One scale for the new-thread view, both sheets and Settings › providers. The chat's composer takes it too, so nothing
jumps on send:

| token | what | Modern | Terminal |
|---|---|---|---|
| `--ctl` | every chip, button, one-line input, and the box's first line | 36 px | 28 px |
| `--row` | every list row in a sheet or in Settings | 44 px | 44 px |
| `--r-ctl` | the radius of chips, buttons and inputs (the box keeps it as it grows) | 18 px | 6 px |
| `--fs` / `--fs-meta` | chip, button and row text / the text beside or under it | 15 / 13 px | 13 / 11 px |

Inputs are 16 px in both styles, because iOS zooms into anything smaller. It is the scale's one exception. Primary
keys (`send`, `sign in`, `retry`) are filled in `--l`. Secondary keys (`sign out`, `+ account`, `copy install`,
`+ new folder`) are `--card` with an `--edge` ring. In Terminal both are bracket keys.

## 7. Desktop: the sidebar (1024 px and wider)

Herdsman's [chat] tab was the model (`assets/serve.css` #386, `docs/design/conductor-chat/chat-1440.png`): the thread
list in a column on the left, the thread beside it with a rule between, the thread's prose capped at 100ch, `‹`
gone. cooee follows it, with these changes for the amendment: one `[+ new thread]` on top in place of herdsman's
`[+ thread]` per repo, one-line rows, and folders that fold.

- **The header** stays across the top (brand, `↻`, `◐`, `⚙`). Its `[+]` goes: `[+ new thread]` is it.
- **The sidebar** is 320 px, always there, scrolling on its own, with a right `--edge` rule. Herdsman's column is 400 px
  because its rows have a second line. On top is **`[+ new thread]`**, `--ctl` tall, a secondary key that has the
  2 px `--l` ring while `/new` is open (Terminal: `[+ new thread]` filled).
- **Then a folder per project**, the folders ordered by their newest thread: `▾ ~/Projects/cooee 3`, the full path
  ellipsised (Terminal: in `--link`), and its thread count. A tap folds it (`▸`), and the fold is kept on the device.
  Under it, newest first, one row per thread: **the state dot** (amber working or connecting, red needs you or
  failed, muted done or idle), **the title** (bold while unread, muted once read, as on the list), and **the age**.
  The open thread's row has the fg-at-8 % fill (Terminal: plus a 3 px `--l` bar). Every row and folder line is
  `--ctl` tall.
- **Beside it:** `/new` (the composer card in the upper third), a chat (the thread bar, items and composer span the
  column, their content 100ch wide in its middle), or `/`, which from 1024 px is the composer too, because the sidebar
  is the list (the Settings › providers shot shows it behind the sheet).
- **Below 1024 px** nothing changes: no sidebar, the list at `/`, `‹` back.
- The agent and folder sheets and the composer card switch at 1024 px too, so the desktop form comes in as one. The
  app's own Settings and look sheets stay at their 1000 px switch.

## Open questions for the owner

1. **`/new` as its own view**, not a card above the list. It lets the composer sit where the chat's will, and send
   then changes nothing under your thumb. **Recommend: as drawn.** With no threads at all, `/` could open `/new`
   with the theme's empty figure in its empty space.
2. **The composer in the upper third at 1440**, not at the bottom. **Recommend: as drawn**: the bottom of a 900 px
   window is far from `[+ new thread]` at the top of the sidebar.
3. **The folder chip shows the name only** (`cooee`). Two folders with the same name in different roots look the same
   on the chip, but the sheet tells them apart. **Recommend: as drawn.**
4. **Account label = email, else name.** The account's cooee name (`work`, `lab`) shows only when the CLI reports no
   email. **Recommend: as drawn.** The alternative is `name · email`, which doesn't fit a 390 chip.
5. **The last sign-in and usage line** (`last sign-in 2d ago · …`) leaves the provider rows. Keep it as a second
   13 px line under each account row (rows become 56 px), or drop it? **Recommend: drop it** for one flat 44 px list.
6. **Last-used comes from the thread store** (server, every device). The alternative is per device in localStorage.
   **Recommend: the thread store.**
7. **The foot link in the agent sheet** (`add an account or a provider in settings ›`) is not in the spec.
   **Recommend: keep it.** Otherwise the sheet has no way to add an account.
8. **A system-default login with no email** has no name to show. **Recommend: `main login`**, which is plain words
   and not a setting. The alternative is the provider's name alone (`claude`), which reads oddly next to the tag in
   Settings › providers.
9. **`/` at 1024 px and wider** opens the composer beside the sidebar, not a second copy of the list. **Recommend:
   as drawn.** The alternative is the last open thread.
10. **The sheet title `agent`** covers both profiles and Plain's accounts. **Recommend: as drawn.** `profile` would
    read oddly until #40 lands, when the sheet holds accounts only.
