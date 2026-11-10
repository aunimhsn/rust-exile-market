#!/bin/sh
# Fails when a target of the release workflow builds ureq with its `cookies` feature: the HTTP
# client would then keep the `POESESSID` session cookie the trade site sets in each answer, and
# send it back with the next request, which docs/trade-site-usage.md forbids. The features of a
# crate are those every crate of the build asks for: a dependency can turn this one on without a
# line changed in this repository. A test checks it too (src/http.rs), on the system of the
# tests only.
#
#   no-session-cookie.sh
set -eu

targets=$(sed -n 's/^ *\(- \)\{0,1\}target: //p' .github/workflows/release.yml)
if [ -z "$targets" ]; then
  echo "No target found in the release workflow"
  exit 1
fi

status=0
for target in $targets; do
  # In a variable first, so that a failure of cargo stops the script with its own message.
  # Without the dev-dependencies, which the release build doesn't have.
  tree=$(cargo tree --locked --target "$target" --edges no-dev,features --invert ureq)
  if ! printf '%s\n' "$tree" | grep -q 'ureq feature "rustls"'; then
    echo "$target: ureq isn't found with its features, the check can't tell"
    status=1
  elif printf '%s\n' "$tree" | grep -q 'ureq feature "cookies"'; then
    echo "$target: ureq is built with its cookies feature (see docs/trade-site-usage.md)"
    status=1
  else
    echo "$target: keeps no cookie"
  fi
done
exit "$status"
