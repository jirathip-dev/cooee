# Every CI gate, locally, the same as CI and `drover turn done` (gatekeeper; a repo gate is one file in .gatekeeper/gates/).
check:
    ./.gatekeeper/run check

# Rewrites every generated file (#356): the crate indexes and Cargo.lock. A conflict in one: take either side, run it.
gen:
    scripts/gen.sh

# The blocking Swift gates on apple/ (#147); --nightly adds Features coverage, unused imports and Periphery (#487).
apple-check *flags:
    scripts/apple-check.sh {{flags}}
