#!/bin/sh
# Fails when a target of the release workflow builds egui-winit without its `clipboard` feature:
# the copies and the pastes in the fields of egui would then stay inside the application, without
# any error. The code compiles and the tests pass all the same: without this check, only pasting
# from or into another application shows it.
#
#   system-clipboard.sh
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
  tree=$(cargo tree --locked --target "$target" --edges no-dev,features --invert egui-winit)
  if printf '%s\n' "$tree" | grep -q 'egui-winit feature "clipboard"'; then
    echo "$target: writes to the clipboard of the system"
  else
    echo "$target: egui-winit is built without its clipboard feature (see eframe in Cargo.toml)"
    status=1
  fi
done
exit "$status"
