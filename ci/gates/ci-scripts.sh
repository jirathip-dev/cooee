#!/bin/sh
# The CI's own scripts (#329): which legs a run needs, the staging reuse, the nightly's issue.
set -eu
ci/ci-scripts-test.sh
ci/test-gate-test.sh
