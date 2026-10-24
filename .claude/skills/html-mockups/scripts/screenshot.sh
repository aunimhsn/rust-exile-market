#!/bin/sh
# Headless screenshot of a mock-up page, to check a change before asking the owner.
#
#   screenshot.sh <url> <output.png> [width] [height]
#
# The page must be served (python3 -m http.server 8765 --bind 127.0.0.1 at the repository
# root). Uses Firefox, installed normally or as a Flatpak; a throwaway profile avoids
# disturbing a running Firefox. The display scale differs from the owner's screen: the
# screenshot shows the direction, the owner tunes the pixels by eye.
set -eu

url=$1
output=$2
width=${3:-1400}
height=${4:-2000}

# A relative output path would be resolved in another folder by Firefox.
case $output in
  /*) ;;
  *) output="$PWD/$output" ;;
esac

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
  echo "Firefox not found: install it, or take the screenshot by hand." >&2
  exit 1
fi

# shellcheck disable=SC2086 # the command may be two words
$firefox_command --headless --no-remote --profile "$profile" \
  --window-size "$width,$height" --screenshot "$shot" "$url"
if [ "$shot" != "$output" ]; then
  mv "$shot" "$output"
fi
echo "$output"
