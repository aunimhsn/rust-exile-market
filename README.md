# rust-project-template

Template for Rust applications (command-line, desktop or web), with the project conventions and the Claude Code setup.

## What it contains

| File | Content |
|---|---|
| `CLAUDE.md` | Rules for Claude Code: communication, naming, language, specifications and roadmap, Git workflow, commit messages, code review, checks, releases. |
| `.claude/agents/code-reviewer.md` | Subagent that reviews a branch before its pull request and writes its notes, in French, to `outputs/`. |
| `specs.md` | Skeleton of the specifications: what the application does. |
| `roadmap.md` | Skeleton of the roadmap: technical choices and steps, one pull request per step. |
| `Cargo.toml` | Package, `unsafe` code forbidden, Clippy pedantic lints, optimized release profile. |
| `rust-toolchain.toml` | Stable toolchain with `rustfmt` and Clippy. |
| `.github/workflows/ci.yml` | Format, lint and test on every pull request, and build with the minimum Rust version (`rust-version`). |
| `.github/workflows/release.yml` | Windows, macOS and Linux archives published as a GitHub release on version tags. |
| `.github/dependabot.yml` | Weekly pull requests to update the crates and the GitHub Actions. |
| `LICENSE-MIT`, `LICENSE-APACHE` | Dual license, the Rust ecosystem default. |

## Start a new project

1. Create the repository from this template, with the **Use this template** button or:

   ```sh
   gh repo create <name> --template aunimhsn/rust-project-template --private --clone
   ```

2. In `Cargo.toml`, update `name`, `description` and `repository` (add `keywords` and `categories` if the crate is published), then run `cargo build` to update `Cargo.lock`.
3. Adapt this README to the project: keep the Requirements, Build, Development, Release and License sections, replace the others.
4. Check the license: keep `MIT OR Apache-2.0`, or change `license` in `Cargo.toml` and the `LICENSE-*` files (they are also copied into the release archives by `.github/workflows/release.yml`).
5. On GitHub, protect `main` (**Settings › Rules › Rulesets**): require a pull request and the `Format, lint and test` and `Minimum Rust version` checks.
6. Set the pull request settings, which are not copied from the template: allow only squash merging, and delete the branches once merged (**Settings › General › Pull Requests**), or:

   ```sh
   gh repo edit --enable-squash-merge --enable-merge-commit=false --enable-rebase-merge=false --delete-branch-on-merge
   ```

7. With Claude Code, write `specs.md`, then split it into steps in `roadmap.md`, and confirm or replace its **Releases** choice.
8. Keep `.github/workflows/release.yml` for a command-line or terminal application; adapt or replace it for a desktop application (installers) or a web application (deployment).

## Requirements

- Rust 1.93 or later (stable toolchain, see `rust-toolchain.toml`)

## Build

```sh
cargo build --release
```

## Development

```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Release

1. Update `version` in `Cargo.toml` (then run `cargo build` to update `Cargo.lock`) and merge it into `main`.
2. Tag the merged commit with the same version and push the tag:

   ```sh
   git switch main && git pull
   git tag v0.1.0
   git push origin v0.1.0
   ```

The [Release workflow](.github/workflows/release.yml) builds the archives for Windows, macOS and Linux and publishes them as a GitHub release. It fails if the tag does not match the version of `Cargo.toml`.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
