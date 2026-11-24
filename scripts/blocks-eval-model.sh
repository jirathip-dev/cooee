#!/bin/sh
# #779: blocks' `blocks eval --model` contract on cooee's model path. Reads `{system, messages}` on stdin, sends the
# system text and the messages as the first message of a new chat with persona $1 and prints the reply's text.
# Usage: scripts/blocks-eval-model.sh <persona> [cooee flags, e.g. --config <file>]
set -eu
persona=${1:?usage: blocks-eval-model.sh <persona> [cooee flags]}
shift
text=$(jq -r '[.system] + [.messages[] | "\(.role):\n\(.text)"] | join("\n\n")')
cooee "$@" send --new "$persona" "$text" --wait --timeout 600
