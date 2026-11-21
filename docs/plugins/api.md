# API

## Settings › Plugins

| Call | Body | Answer |
|---|---|---|
| `GET /plugins` | | `{installed: {name, version, commit, listed, state, enabled, first_party, improving?, previous?, tabs, places, claims, permissions, accepts}[], index: {name, description}[], jobs: {name: text}, issues}` (`claims` (#229): the workflow `source` labels its manifest claims; the native app draws a `GET /threads` row whose `workflow.source` one claims in that plugin's place on a persona it is attached to, and any other workflow row under Home's Others) |
| `GET /plugins/log?name=N` | | the plugin's log, text |
| `POST /plugins/plan` | `{source}` or `{name, at}` | `{commit, text}`: what would run, to confirm |
| `POST /plugins/add`, `/plugins/update` | the plan's body and its `commit` (a confirm holds for that commit only) | a JSON string (`"installing"`, …) |
| `POST /plugins/enable` | `{name, on}` | a JSON string |
| `POST /plugins/improve` | `{name, text}` | `{thread}` |
| `POST /plugins/approve`, `/plugins/rollback`, `/plugins/remove` | `{name}` | a JSON string |

## Plugin backends

`GET` or `POST /p/{name}/api/{path}` reaches the plugin's own backend through cooee's proxy, with its token, never the
cookie. An event stream comes back as SSE messages of `{event, data}`. The `terminal` plugin's calls need the device
unlocked with its passkey: else 403 `locked` (header `Cooee-Locked: 1`).

## Plugin screens (#125)

A plugin whose manifest has `[[places]]` serves its screens as blocks ([Screens](screens.md)). A device
reaches them through cooee, with the plugin's token, never the cookie:

| Call | Body | Answer |
|---|---|---|
| `GET /p/{plugin}/screens/{path}` | | the backend's `GET /screens/{path}`: a screen (JSON), or its stream (SSE, each event `{seq, ops}` as sent, its id kept; `Last-Event-ID` goes along). Every client of one stream shares one backend connection: a later one first gets the kept screen, and a stream that ends after an event is opened again with the last id, its clients still connected |
| `POST /p/{plugin}/action` | `{screen, block, action, values}` | the backend's answer to `POST /action` (`by`: the device), as it sent it; 403 for an action not in its `[permissions] actions` (never sent); 404 for no installed plugin with places; 502 when the backend can't be reached |
