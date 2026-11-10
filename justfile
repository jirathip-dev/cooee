# Every CI gate, locally (the ui gate needs `npm ci` in web/e2e once).
check: fmt clippy test doc deny cap deps hygiene themes ts ui

# Installs the pinned topcoat into .tools/ when it is not on PATH; `scripts/fmt.sh fix` rewrites.
fmt:
    scripts/fmt.sh

clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings
    # At most 10 allows in the workspace, each with a reason (clippy checks the reason).
    test "$(grep -rE 'allow\(' crates web/src | wc -l)" -le 10

test:
    cargo test --workspace --locked

doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

deny:
    cargo deny --locked check

cap:
    scripts/line-cap.sh
    ci/line-cap-test.sh

deps:
    scripts/dep-direction.sh

hygiene:
    scripts/hygiene.sh

# The theme gate (docs/themes.md) on the first-party packs and the starter.
themes:
    cargo run --quiet --locked -p cooee -- theme check packs/themes/* packs/starter

# web/ts: tsc strict and Biome (blocking), then tsc-rs as a trial (needs `npm ci` in web once).
ts:
    cd web && npx tsc -p ts && npx tsc -p ts/tsconfig.sw.json && npx biome check ts
    -cd web && npx tsc-rs -p ts && npx tsc-rs -p ts/tsconfig.sw.json

# Playwright at 390 and 1440 px with axe; screenshots in ui-390/ and ui-1440/.
ui:
    cd web/e2e && npx playwright test
