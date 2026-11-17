# Styles as packs (#488)

## Summary

Decision: A style becomes one small data file in a pack: a closed list of choices (type, corners, surfaces, marks,
figures, label decoration, the scene strip). The app keeps one renderer and one built-in style, Modern, which is
that same file, bundled. Terminal and Scene become two first-party packs. A pack ships no code and can't change text.
Slices: the file, its check and the two packs on the server; the app's figures and scene strip read it; every other
per-style branch reads it; packs replace the built-in list and their frames leave the app's gate; fewer pack frames.
Open questions: four, each with a default: eight Terminal wordings, the Terminal keyboard, upgrades, the packs' repo.

## Files

| File | What it holds |
|---|---|
| [`adr.md`](adr.md) | The question, what the code says, the options, the decision, what tokens can't express, the gate, the trust rule, the snapshot rule, the consequences, the #300 amendment, the open questions, the hot files |
| [`callers.md`](callers.md) | How a view, the root, the pick, the tests and the CLI read after the change, written first |
| [`shapes.md`](shapes.md) | `style.json` for Modern, Terminal and Scene, the gate's rules, the Swift and Rust types, the wire |
| [`boundaries.md`](boundaries.md) | Which module may call which, and the statecharts of a pack on the server and the style on a device |
| [`migration.md`](migration.md) | The five impl issues in order, with what each adds, deletes and proves, and the line estimates |

## At a glance

```text
 packs/terminal/style.json ─┐
 packs/scene/style.json ────┤ gate (cooee theme check, every load)
 <config>/themes/*/style.json ┘        │
                                       ▼
                         GET /api/v1/looks {styles, broken}
                                       │
   CooeeUI/Default/style.json          ▼
   (Modern, bundled) ───────► Style ──► Look ──► Look+Style.swift ──► every view
                               ▲                  label · radius · pill · surface · chip · Mark · FigureView · Stage
              Settings keeps the last good one
```

A style is 12 keys. Every value is a name from a closed list, a number from 0 to 1, a colour, or at most four
punctuation characters. The app's tests draw each design frame once, in Modern; a pack's frames are a closed set
beside them.
