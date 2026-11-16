#!/bin/sh
# Public hygiene: nothing from ci/denylist.txt is in the tree.
set -eu
scripts/hygiene.sh
