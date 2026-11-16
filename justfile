# Every CI gate, locally (ci/gates/, one file each); `just check fmt clippy` runs only those.
check *gates:
    ci/gates.sh {{gates}}

# Rewrites every generated file (#356): the crate indexes and Cargo.lock. A conflict in one: take either side, run it.
gen:
    scripts/gen.sh

# The Swift quality gate on apple/ (#147): format, anti-slop, SwiftLint, build, tests, coverage, Periphery.
apple-check:
    scripts/apple-check.sh
