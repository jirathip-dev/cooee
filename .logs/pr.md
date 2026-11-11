Closes #252

## For humans
The iPhone and Mac app's Look sheet now has a Theme list: the theme packs installed on your paired server (farm, kitchen, garden, and any you add). Picking one recolours Home, chats and Terminal at once, in light and dark, and Scene draws that pack's scene and figures. Farm stays the default, and a pack you remove on the server drops out of the list.

## What changed
- Server: `GET /api/v1/looks` answers the installed themes and schemes as JSON (the same data the web pages embed).
- Native: Settings keeps a `theme` (default farm; old saved settings read as farm). The menu loads the looks when Home starts and each time the menu opens. Root builds the palette from the picked pack's scheme tokens for the current mode, else farm. A pack other than farm draws Scene's strip and the sprites from its `scene.svg` / `sprites.svg` on the server (a WebKit view, no new dependency); farm keeps the app's own coat sprites.
- e2e: `serve.sh` installs the three first-party packs; `LiveUITests` picks each one in Modern and Scene and keeps screenshots.

## Evidence (head: pending CI)
