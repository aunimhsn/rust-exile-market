#!/bin/sh
# Runs Clippy for Windows from Linux, like the CI on Windows (.github/workflows/ci.yml), so that
# the code written for Windows alone is checked before a push. Nothing is linked: the C code of
# the dependencies (ring) goes to a stand-in compiler that writes empty files, enough for a
# check, in a target directory of its own so that they never reach a real build.
set -eu
# From the root of the project, wherever the script is run from.
cd "$(dirname "$0")/.."

# The standard library of the target, for the version of Rust of rust-toolchain.toml: added
# again after each new version, at once when it is there.
rustup target add x86_64-pc-windows-msvc

target=x86_64-pc-windows-msvc
# At a fixed path: a path that changes would make Cargo check ring and what uses it at every run.
stub=$(pwd)/target/clippy-windows/stand-in
mkdir -p "$stub"

# Writes the empty object (-Fo) or archive (-out:) the build script asks for.
cat > "$stub/cl" <<'EOF'
#!/bin/sh
for argument in "$@"; do
    case "$argument" in
        -Fo* | /Fo*) : > "${argument#?Fo}" ;;
        -out:* | /out:* | /OUT:*) : > "${argument#*:}" ;;
    esac
done
EOF
chmod +x "$stub/cl"
cp "$stub/cl" "$stub/lib"

CC_x86_64_pc_windows_msvc="$stub/cl" \
    AR_x86_64_pc_windows_msvc="$stub/lib" \
    CARGO_TARGET_DIR=target/clippy-windows \
    cargo clippy --locked --workspace --all-targets --all-features --target "$target" -- -D warnings
