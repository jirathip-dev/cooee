# Every CI gate, locally (ci/gates/, one file each); `just check fmt clippy` runs only those.
check *gates:
    ci/gates.sh {{gates}}

# Rewrites every generated file (#356): the crate indexes and Cargo.lock. A conflict in one: take either side, run it.
gen:
    scripts/gen.sh

# The blocking Swift gates on apple/ (#147, #487): format, anti-slop, SwiftLint, build, tests, Core and API coverage.
apple-check:
    scripts/apple-check.sh

# Every Swift gate, as the nightly runs them: apple-check plus Features coverage, unused imports and Periphery.
apple-check-nightly:
    scripts/apple-check.sh --nightly
