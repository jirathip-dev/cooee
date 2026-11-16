#!/bin/sh
# Licences, advisories and duplicate versions pass cargo deny.
set -eu
cargo deny --locked check
