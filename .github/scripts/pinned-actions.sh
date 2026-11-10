#!/bin/sh
# Fails when a workflow uses an action by a tag or a branch, which its owner can move to
# another code: each action is pinned to a commit, its version in a comment, and Dependabot
# updates both.
set -eu
# From the root of the project, wherever the script is run from.
cd "$(dirname "$0")/../.."

if grep -nE '^[[:space:]]*(-[[:space:]]+)?uses:' .github/workflows/*.yml \
    | grep -vE 'uses: [^@[:space:]]+@[0-9a-f]{40}( # v[0-9][^[:space:]]*)?$'; then
  echo "Pin these actions to a commit: uses: owner/action@<40 hexadecimal digits> # v1.2.3"
  exit 1
fi
echo "Every action is pinned to a commit"
