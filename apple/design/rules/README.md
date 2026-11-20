# Approval rules in Settings and a provider's gaps in the agent picker (#536, provider contract slice 5)

## Summary

Decision: a persona's sheet gets a **Rules** row that opens its rules, listed as the server's plain sentences, strictest first, each with where it came from. A rule the agent never asks for is tagged "Codex never asks for this", on the phone and the Mac. **Add a rule** opens a sheet with four choices: the answer, the kind, the pattern and the folder. It shows no preview. Swiping a rule left (on the Mac, hovering it) and confirming removes it. The agent picker lists each agent's gaps in amber. An agent with no gaps gets no line.
Screens: the rules list, adding a rule, removing one, the agent picker with a gap, and the Mac window. Each is drawn in Modern, Terminal and Scene, dark and light (30 images).
Open questions: none

## Revised (q2716)

Dropped: the "strictest wins" line under the list, the "A gap is…" line under the picker, the sub-folder hint in the add sheet, Claude Code's green "Asks before everything" line, and the Mac's amber "3 of these never come up" header. There is no dry-run preview in the add sheet. The phone frames now show Coder (on Codex) instead of Chief, so the "Codex never asks for this" tag is on the phone list as well as the Mac.

## How it was made

This is a scratch package, made the same way as `composer-v2` and `connectors`. It depends on `apple/` by path and draws with
CooeeUI's own `Group2`, `SettingRow`, `StateWord`, `ActionPill`, `CardButton`, `RoundButton`, `Glyph`, `Dim`,
`Backdrop`, `PhoneChrome` and `MacChrome`, plus `ToolKind`'s symbols and words from the tool rows. Only the new pieces are
here, under `Sources/RulesDesign/`, fed by fixtures (`Model.swift`). There is no API client, no app logic and no app target.
Running `swift test` in this folder rewrites every PNG: iPhone 17 at 402 × 874 pt and the Mac window at 1440 × 860 pt, both at
2×. Terminal and Scene are read from `packs/<pack>/style.json`. The names are `<frame>-<style>-<mode>.png`.

## Frames and the ADR decisions they show (`docs/architecture/provider-contract/`)

| PNG | shows | ADR |
|---|---|---|
| `list-*` | Coder › **Rules**. One group, **Coder's rules**. Each row is the rule's `words` exactly as the server sends them ("Allow running cargo test … in ~/work"). Under each is where it came from: `from a card · 18 Nov` (Always allow), `added here · 20 Nov` (Settings) or `starter rule`. The mark is the kind's tool-row symbol, coloured by effect (green allow, amber ask, red deny); Terminal draws `[allow]`, `[ask]` or `[deny]` instead. Rows are sorted strictest first, with no line explaining it. Rows whose kind is in the agents `AsksNot` gaps add `· Codex never asks for this` in amber. Last comes **Anything else: Ask you on a card**, the persona's `policy`, read only. The two starter rules are listed and removable like any other. | callers §5 `GET /personas/<id>/rules` and its `words`; shapes `Rule.by` card / settings / starter; `Effect` deny > ask > allow; adr open question 2 (starter rules); boundaries: the app never builds a rule's sentence |
| `add-*` | **Add a rule** opens **New rule** over the list. **Answer** has Allow, Ask first and Deny. **When it wants to** has Read, Edit, Run, Search, Fetch, Tool and Other, the tool rows' words and symbols. The pattern field's label and hint follow the kind; Run shows "Its first words; end with * for anything after them". **In** offers the persona's folder (the default) or **Any folder**. **Add** posts the rule. A 400's message ("what is wrong") shows in red under the field, and the sheet stays open. | callers §5 `POST /personas/<id>/rules {effect, kind, pattern, folder?}`; shapes' matching table (kind → pattern); `folder: None` = any folder |
| `remove-*` | Swipe a rule left to show **Remove**. A confirmation shows the rule's sentence and the line "The next request it matches asks you on a card again." **Remove rule** sends the DELETE. The confirmation is there because removing a deny rule loosens what the agent may do. | callers §5 `DELETE …/rules/<id>` → "the next matching request is a card again" (AC 2) |
| `picker-*` | Coder's sheet. The **Agent** group shows `Codex · gpt-5.5` with an amber `4 gaps`, and a **Rules** row under it. Tapping the agent opens **Coder's agent**. Each agent row has its name, its models and then its gaps, one amber `⚠` line each (Gap `words()` from the server). An agent with no gaps gets no line. Picking an agent with gaps is allowed. There is no note under the list. Model and Effort follow, as today. | shapes `Gap { PersonalFiles, StillOn, AsksNot }`, "`words()` is the line the agent picker … show[s]"; callers (Codex's gaps: PersonalFiles, AsksNot Read / Search / Run); capabilities "the agent picker says what it doesn't do" |
| `mac-*` | The Mac window: Coder's sheet on the left with **Rules** selected, and Coder's rules beside it. No header under the title. As on the phone, rows whose kind is in the agent's `AsksNot` gaps add `· Codex never asks for this`. The hovered row shows **Remove** (Mac has no swipe), which opens the same confirmation. | adr decision 5 (Codex never asks inside its sandbox, so rules on those do nothing there); the same list, add and remove |

## Decisions I made (open to revision)

- **Rules sit under the persona, not as a Settings sidebar page.** Rules are per persona (`<config>/agents/<id>/rules.toml`), and the persona's sheet is where its settings live, as with its connectors (#524).
- **One list, strictest first,** with no separate groups for deny, ask and allow. The sentence already starts with its verb, and the colour marks the effect.
- **No preview in the add sheet** (owner, q2716). After **Add**, the new rule appears in the list with the server's sentence.
- **The `policy` row is read only.** The issue doesn't ask to change it.
- **The gap lines are the server's `words()`.** The app only places them. The "never asks for this" tag on a rule (phone and Mac) matches the rule's kind against the agent's `AsksNot` gaps, so the API needs to send gaps as `{kind, words}`, not only as sentences.

## Not drawn

- The three-button card itself (slice 4) and the tool row's "ruled by" line.
- A rule save that fails. The add sheet stays open with the server's message, as described above.
