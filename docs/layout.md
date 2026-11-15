# Layout (#300)

## API

The ADR: [architecture/layout-blocks](architecture/layout-blocks/adr.md.

The owner's `layout.json` beside `config.toml`, read every second. The server serves only a layout that passes the
app's rules (`cooee layout check <file>` runs the same ones): with no file, or no good one yet, the app's bundled
default (`apple/Sources/CooeeUI/Default/layout.json`). A bad write changes nothing served.

| Call | Body | Answer |
|---|---|---|
| `GET /layout` | | `{etag, layout}` and an `ETag` header; 304 with no body when `If-None-Match` names it |
| `GET /layout/events` | | SSE: `changed {etag}` after a good write; `broken {line, col, why}` after a bad one, and at once to a new subscriber while the file is broken |
| `POST /layout/reset` | | writes the default to `layout.json`, the old file kept as `layout.json.bak`; answers as `GET /layout`; 409 with no config folder |
