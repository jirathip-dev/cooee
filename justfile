# Every CI gate, locally (the ui gate needs `npm ci` in web/e2e once).
check: fmt clippy test doc deny cap deps hygiene neutral lessons themes ts ui

fmt:
    cargo fmt --all --check
    topcoat fmt --check $(git ls-files 'crates/*.rs' 'web/*.rs')

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

# Core names no provider (#200): only crates/cooee-core/src/providers/ and tests may.
neutral:
    scripts/neutral-core.sh
    ci/neutral-core-test.sh

# The lesson checks' own tests (#323); the checks themselves run on a PR in .github/workflows/lessons.yml.
lessons:
    ci/pr-body-test.sh

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

# The Swift quality gate on apple/ (#147): format, anti-slop, SwiftLint, build, tests, coverage, Periphery.
apple-check:
    scripts/apple-check.sh
