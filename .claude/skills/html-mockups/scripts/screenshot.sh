#!/bin/sh
# Headless screenshot of a mock-up page, to check a change before asking the owner.
#
#   screenshot.sh <url> <output.png> [width] [height | full] [selector]
#
# The page must be served (python3 -m http.server 8765 --bind 127.0.0.1 at the repository
# root). Uses Playwright and its own Chromium when installed (see screenshot.cjs): closer to
# the Chromium of VS Code where the owner looks at the mock-up, it reports the errors of the
# browser console, `full` captures the whole page, and a selector captures one window only
# and lists its texts cut or wrapped. Otherwise uses Firefox, installed normally or as a
# Flatpak; a throwaway profile avoids disturbing a running Firefox; its fonts differ, so the
# screenshot shows the direction and the owner tunes the pixels by eye.
set -eu

url=$1
output=$2
width=${3:-1400}
height=${4:-2000}
selector=${5:-}

# A relative output path would be resolved in another folder by Firefox.
case $output in
  /*) ;;
  *) output="$PWD/$output" ;;
esac

playwright_home=${PLAYWRIGHT_HOME:-$HOME/.local/share/mockup-tools}
if command -v node >/dev/null 2>&1 && [ -d "$playwright_home/node_modules/playwright" ]; then
  exec node "$(dirname "$0")/screenshot.cjs" "$url" "$output" "$width" "$height" "$selector"
fi

if [ -n "$selector" ]; then
  echo "One window needs Playwright (see screenshot.cjs); Firefox takes the whole window." >&2
fi
if [ "$height" = full ]; then
  echo "A full page needs Playwright (see screenshot.cjs); Firefox takes the window only." >&2
  height=2000
fi

if command -v firefox >/dev/null 2>&1; then
  firefox_command="firefox"
  profile=$(mktemp -d)
  shot=$output
elif command -v flatpak >/dev/null 2>&1 && flatpak info org.mozilla.firefox >/dev/null 2>&1; then
  firefox_command="flatpak run org.mozilla.firefox"
  # A Flatpak can only write inside its own data folder: shoot there, then move the image.
  folder="$HOME/.var/app/org.mozilla.firefox/cache/mockup-screenshots"
  profile="$folder/profile"
  shot="$folder/screenshot.png"
  mkdir -p "$profile"
else
  echo "Neither Playwright nor Firefox found: install Playwright (see screenshot.cjs)." >&2
  exit 1
fi

# shellcheck disable=SC2086 # the command may be two words
$firefox_command --headless --no-remote --profile "$profile" \
  --window-size "$width,$height" --screenshot "$shot" "$url"
if [ "$shot" != "$output" ]; then
  mv "$shot" "$output"
fi
echo "$output"
