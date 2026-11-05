# Every CI gate, locally (the ui gate needs `npm ci` in web/e2e once).
check: fmt clippy test doc deny cap deps hygiene ui

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

deps:
    scripts/dep-direction.sh

hygiene:
    scripts/hygiene.sh

# Playwright at 390 and 1440 px with axe; screenshots in ui-390/ and ui-1440/.
ui:
    cd web/e2e && npx playwright test
