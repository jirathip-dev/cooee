## Summary

Decision: Settings gets a **Connectors** page (what's connected, then the reviewed catalogue, then "Add by URL…"); an entry's page says what leaves the Mac before you tap **Connect**; sign-in is the service's own page in the system sheet, then one tap attaches it to a persona; a persona's sheet lists its connectors, each with its account, and **Attach a connector** picks a signed-in account in one tap.
Screens: the Settings row, the catalogue, an entry, the sign-in sheet (plus "connected, attach it" and the pasted-token case), a persona's connectors, and the Mac window. Every frame in Modern, Terminal and Scene, each dark and light (48 images).
Open questions: (1) Should signing in end on the "attach it to a persona" prompt (drawn), or go back to the entry? (2) The Settings row shows "1 needs sign-in" in amber when an account needs it: OK to surface it there?

## Built (#524)

The scratch package below is gone: its views are the app's own now (`CooeeUI/Connector*.swift`,
`PersonaConnectors.swift`), and `apple/Tests/CooeeUITests/Frames/Connectors.swift` draws every frame from them
(`CooeeUI/Fixtures/ConnectorFrames*.swift` holds the parts that stay drawn: the system sign-in sheet, the open attach
menu, the Mac window's sidebar) and compares it with its PNG here. `COOEE_RECORD=1` rewrites a PNG.

## How these were first made

A scratch package that depends on `apple/` by path and draws with CooeeUI's own `SettingsPage`, `Group2`, `Line`,
`SettingRow`, `ActionPill`, `StateWord`, `RoundButton`, `FigureView`, `PhoneChrome` and `MacChrome`. Only the new pieces
are here, under `Sources/Connectors/`, fed by fixtures (`Model.swift`). There is no API client and no app logic, and
no app target links it. `swift test` in this folder rewrites every PNG: iPhone 17 at 402 × 874 pt and the Mac window
at 1440 × 860 pt, both at 2×. The names are `<frame>-<style>-<mode>.png`, with style `modern`, `terminal` or `scene` and mode `dark` or `light`. Every name, host and tool list is an example
([callers](../../../docs/architecture/connectors/callers.md)).

## Revision 2

The revise came with no note. This round draws what the issue asks for and the first round left out: every frame in all three styles, dark and light. Before, Terminal was drawn only dark, Scene only light, and the signed-in and token frames only in some styles. The layout and the decisions are unchanged.

## Frames and the ADR decisions they show

| PNG | shows | ADR |
|---|---|---|
| `settings-row-*` | The avatar menu gains **Connectors** under Plugins. Its value is the count, or in amber `1 needs sign-in` when an account needs it, so a refused refresh is visible without opening the page. | 1 (Settings › mcp servers becomes Settings › Connectors); 6 (*needs sign-in*) |
| `catalogue-*` | Settings › Connectors. **Connected**: one row per server in the registry, whatever its source. A catalogue entry shows `✓ reviewed · run by Linear`. A typed URL shows amber `⚠ not reviewed · mcp.acme.dev`. A command shows `a command on this Mac · added on the Mac`. Under each comes one line per account (`main · signed in · Chief, Researcher`). An account that needs sign-in gets an amber **Sign in**. **Add from the catalogue**: label, host (or `built in · read only`), the reviewed `leaves` sentence, and the tool counts (`5 run · 4 ask first`). Last comes **Add by URL…**, with its warning. | 1, 2, 3, 12 |
| `entry-*` | Notion before connecting. The header reads `reviewed · run by Notion` and `pinned to mcp.notion.com · checked 20 Nov 2026`. One **Connect** button sits under it, with the line "Opens Notion's own sign-in page. Your Mac keeps the key; no persona sees it." **What leaves this Mac** has three lines: the entry's sentence with its host, that signing in tells the service this Mac's tailnet name, and that what it returns is read by the persona's model. **Tools** lists read (run without asking), write (ask you first, in the chat) and never (not offered), each with its names. | 2, 8, 11 |
| `signin-*` | **Connect** opens the system's web sign-in sheet on the service's own page. The page is system and service UI, so it looks the same in every cooee style; only the page behind it changes. Behind the sheet, the account reads `main · signing in…`. The sheet closes itself when the Mac's callback redirects to `cooee://connector`. | 5 |
| `signed-in-*` | After the sheet closes: `Notion is connected as main`, then **Attach it to a persona** with the persona avatars, and **Not now**. A tap on an avatar is the third tap (entry, Connect, persona). The entry now shows **Accounts**: `main · signed in`, **Sign out**, and **Add an account…**. | 5, 9; callers 1 "three taps" |
| `token-*` | The token case: GitHub's account needs sign-in (`the token was revoked`), and **Sign in again** opens cooee's own sheet. It has the entry's help text, a masked field with **Paste**, the line "Goes straight to your Mac's keychain. No file, chat or log keeps it.", and **Save**. | 6, 7 |
| `persona-*` | Coder's edit sheet, with a **Connectors** group after Instructions. **Linear** has its account chip (`work ⇅`, the picker for this persona's one account), its counts and **Detach**. **GitHub** (`main`) reads `needs sign-in · its tools are off until then`, with **Sign in**; the pick stays. **Attach a connector** is drawn open. It lists every signed-in account as `Linear · main`, `Linear · work` (attached, checked) and `acme · main` (amber `not reviewed`), so one tap picks both the connector and the account. **From the catalogue…** goes to the catalogue. All of it is the persona's normal save (✓). | 9, 3, 6 |
| `mac-*` | The Mac window has the Settings sidebar, with **Connectors** selected and an amber dot. The list column is the phone's catalogue page, and the selected row (Linear) opens its entry beside it. The entry shows two accounts (`main` used by Chief and Researcher, `work` by Coder), each with Sign out, then **Add an account…**, what leaves, the tools and **Remove Linear** ("Personas keep their picks, which read 'not connected'"). | 1, 9, 10, 11; `DELETE /connectors/{name}` |

## Decisions I made (open to revision)

- **The account is a chip on the pick, not a separate row.** It is one account per connector per persona (decision
  9), so the chip is the picker. Attach lists signed-in accounts directly, so a connector with no signed-in account
  is reached through **From the catalogue…**, which signs in first.
- **A pick whose account needs sign-in stays attached** and says its tools are off. Its button is **Sign in**, not
  Detach. This matches the statechart: signing out leaves the picks.
- **Catalogue rows open the entry; they have no Add button.** Nothing is saved until **Connect**, because the entry
  page is where the owner reads what leaves the Mac. Behind the button, `POST /connectors` and the sign-in start
  happen in one tap.
- **Tool names are shown in full** on the entry, grouped as run / ask first / never. A tool missing from every list
  asks (decision 8). The entry page doesn't repeat that rule; the Add by URL line says it.
- **Remove** sits on the entry, last and in red, with its consequence on the line below.

## Not drawn

- The typed URL's plan card (`POST /connectors/plan`, the exact URL, the sign-in server's host, Face ID). It reuses
  the plugin proposal's card.
- The write tool's card in the chat (slice C, already built), and "Mark as read" with Face ID on a typed server's
  tools.
- The owner's own sign-in client field (`Client::Owner`, slice F's Google entries). It is the token sheet with an id
  and a secret.
