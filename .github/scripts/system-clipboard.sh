#!/bin/sh
# Fails when a target of the release workflow builds egui-winit without its `clipboard` feature:
# `Context::copy_text` (Copy whisper, Copy the log) would then keep the text inside the
# application while the pill still reads "Copied". The code compiles and the tests pass all the
# same: without this check, only pasting into another application shows it.
#
#   system-clipboard.sh
set -eu

targets=$(sed -n 's/^ *- target: //p' .github/workflows/release.yml)
if [ -z "$targets" ]; then
  echo "No target found in the release workflow"
  exit 1
fi

status=0
for target in $targets; do
  # In a variable first, so that a failure of cargo stops the script with its own message.
  tree=$(cargo tree --locked --target "$target" --edges features --invert egui-winit)
  if printf '%s\n' "$tree" | grep -q 'egui-winit feature "clipboard"'; then
    echo "$target: writes to the clipboard of the system"
  else
    echo "$target: egui-winit is built without its clipboard feature (see eframe in Cargo.toml)"
    status=1
  fi
done
exit "$status"
