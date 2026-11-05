#!/bin/sh
# No setup-specific names or secrets (ci/denylist.txt) in any tracked file.
set -eu
cd "$(dirname "$0")/.."
pat=$(grep -v -e '^#' -e '^$' ci/denylist.txt | paste -sd '|' -)
if git ls-files -z ':!ci/denylist.txt' | xargs -0 grep -nIE "$pat"; then
  echo "setup-specific names or secrets found (ci/denylist.txt); use placeholders"
  exit 1
fi
echo "no setup-specific names or secrets"
