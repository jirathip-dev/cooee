# Every CI gate, locally (ci/gates/, one file each); `just check fmt clippy` runs only those.
check *gates:
    ci/gates.sh {{gates}}

# Rewrites every generated file (#356): the crate indexes and Cargo.lock. A conflict in one: take either side, run it.
gen:
    scripts/gen.sh

# The blocking Swift gates on apple/ (#147); --nightly adds Features coverage, unused imports and Periphery (#487).
apple-check *flags:
    scripts/apple-check.sh {{flags}}
