#!/bin/sh
# The lesson checks' own tests (#323); the checks themselves run on a PR in .github/workflows/lessons.yml.
set -eu
ci/pr-body-test.sh
