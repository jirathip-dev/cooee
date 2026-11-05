# Topcoat chat spike (#10)

A throwaway: cooee's chat screen in [Topcoat](https://github.com/tokio-rs/topcoat) `=0.11.0`, measured against the
criteria in `docs/decisions/0001-ui-framework.md`. Its own Cargo workspace, outside `crates/`. Not production code.

- `/`: the thread list. `/live`: a thread fed by the fake shed (`src/shed.rs`): 2,000 numbered tokens at 50 tokens/s,
  two tool calls changing status, an approval row, the final reply. `/long`: 1,000 messages.
- `theme/`: herdsman's farm theme and scheme, copied unchanged (`theme/SOURCE`, `theme/SHA256SUMS`).

## Run

```sh
./run.sh                                            # http://127.0.0.1:3000
PORT=8390 HOST=$(tailscale ip -4) ./run.sh          # on the tailnet: http://<that ip>:8390 from the iPhone
TOKENS=2000 RATE=50 ./run.sh                        # the fake shed's run (the defaults); ↻ in the thread restarts it
```

`run.sh` installs the Topcoat CLI into `.tools/` (not on the host), bundles the browser runtime, runs the release build.

## Measure

With the server running (Node 22+, Chrome for Testing from Playwright's cache or `CHROME=<binary>`):

```sh
HEADFUL=1 node measure/measure.mjs http://127.0.0.1:3000 /tmp/shots   # JSON on stdout, 390 px screenshots in /tmp/shots
```

`HEADFUL=1` opens a real Chrome window: headless Chrome reports no frame presentation, so its latency numbers are
meaningless. `measure/results-run*.json` are the runs the ADR quotes.
