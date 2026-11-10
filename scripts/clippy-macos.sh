#!/bin/sh
# Runs Clippy for macOS from Linux, for the CI (.github/workflows/ci.yml) and before a push: a
# machine with macOS may not be free for a quarter of an hour, and a check needs none. Nothing is
# linked, so the C code of the dependencies (ring) needs no compiler: `true` stands in for it, in
# a target directory of its own. The release workflow still builds the real executables on macOS.
#
set -eu
# From the root of the project, wherever the script is run from.
cd "$(dirname "$0")/.."

# The standard library of the target, for the version of Rust of rust-toolchain.toml: added
# again after each new version, at once when it is there.
rustup target add aarch64-apple-darwin

CC_aarch64_apple_darwin=true \
    AR_aarch64_apple_darwin=true \
    CARGO_TARGET_DIR=target/clippy-macos \
    cargo clippy --locked --workspace --all-targets --all-features --target aarch64-apple-darwin -- -D warnings
