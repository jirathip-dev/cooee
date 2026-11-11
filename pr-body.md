Closes #251

## For humans

The approved design for the new Home screen. Home loses its title bar: two floating buttons sit over one list. Search and your personas scroll away under a glassy top. Terminal gets a shell look, and Scene gets a moving Herd that you can pull down into a full ranch.
It adds pictures and a README only. The app does not change yet: the build issues filed from this design will change it.

## What

- `docs/design/home-v2/`: the HTML mockups (`modern.html`, `terminal.html`, `scene.html` on the live design system), `h2.css`/`h2.js`/`herd.js`, 28 PNGs at 390 (light and dark) and 1440, and a README that maps each PNG to its decision (q1321–q1324, revision q1342/q1349).
- The owner approved it in q1352. q1349 says to keep "HTML now with build issue 1 writing SwiftUI views + snapshots", so this PR has no SwiftUI views and no `apple/design/home-v2/` folder. The frames are in `docs/design/home-v2/`, where design lanes put them.
- origin/staging (9c83e70) is merged in. There are no conflicts.
- Fix round: the red `ui` check at 1af6150 was `web/e2e/chat.spec.mjs:177`, which asserts no browser task runs longer than 50 ms. One task took 196 ms. This PR touches no web code. It is a timing flake, and CI re-runs on the new head.

`src/ net +0 (budget n/a)`: design files only.

## Evidence (head 8f850c5c8a462aaa1f41a1c3c5cc6142d2e44ecb)

- `git diff --name-only origin/staging HEAD | rg -v '^docs/design/home-v2/'`: exit 1. Every changed file is under `docs/design/home-v2/`, so no Swift, Rust or web source changes. The iOS and macOS targets build as staging does, and CI confirms it on this head.
- The PNGs are the design lane's output from `node docs/design/home-v2/shoot.mjs`, which reports 28/28 checks passing in the README. I did not re-run it in this lane because it needs chrome-headless-shell.
