#!/bin/sh
# Runs Clippy for macOS from Linux, for the CI (.github/workflows/ci.yml) and before a push: a
# machine with macOS may not be free for a quarter of an hour, and a check needs none. Nothing is
# linked: the C code of the dependencies (ring) goes to a stand-in compiler that writes empty
# files, enough for a check, in a target directory of its own so that they never reach a real
# build. The release workflow still builds the real executables on macOS.
#
# Needs the standard library of the target once: rustup target add aarch64-apple-darwin
set -eu
# From the root of the project, wherever the script is run from.
cd "$(dirname "$0")/.."

target=aarch64-apple-darwin
stub=$(mktemp -d)
trap 'rm -rf "$stub"' EXIT

# Writes the empty object the build script asks for (-o <file>).
cat > "$stub/cc" <<'EOF'
#!/bin/sh
previous=
for argument in "$@"; do
    if [ "$previous" = -o ]; then
        : > "$argument"
    fi
    previous=$argument
done
EOF
# Writes the empty archive the build script asks for (ar <flags> <archive> <objects>).
cat > "$stub/ar" <<'EOF'
#!/bin/sh
: > "$2"
EOF
chmod +x "$stub/cc" "$stub/ar"

CC_aarch64_apple_darwin="$stub/cc" \
    AR_aarch64_apple_darwin="$stub/ar" \
    CARGO_TARGET_DIR=target/clippy-macos \
    cargo clippy --locked --workspace --all-targets --all-features --target "$target" -- -D warnings
