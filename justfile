# Every CI gate, locally.
check: fmt clippy test doc deny cap deps hygiene neutral lessons themes ci-scripts

fmt:
    cargo fmt --all --check
    topcoat fmt --check $(git ls-files 'crates/*.rs')

clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings
    # At most 10 allows in the workspace, each with a reason (clippy checks the reason).
    test "$(grep -rE 'allow\(' crates | wc -l)" -le 10

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

# The CI's own scripts (#329): which legs a run needs, the staging reuse, the nightly's issue.
ci-scripts:
    ci/ci-scripts-test.sh

# Rewrites every generated file (#356): the crate indexes and Cargo.lock. A conflict in one: take either side, run it.
gen:
    scripts/gen.sh

# The Swift quality gate on apple/ (#147): format, anti-slop, SwiftLint, build, tests, coverage, Periphery.
apple-check:
    scripts/apple-check.sh
