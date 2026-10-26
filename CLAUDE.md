# CLAUDE.md

## Communication

- Always communicate with the user in French through the Claude Code harness (chat responses, explanations, questions).

## Naming conventions

- Follow Rust conventions: Rust source files and module directories use snake_case (e.g. `src/file_parser.rs`, `src/user_settings/`).
- All other files and directories use kebab-case (e.g. `docs/getting-started.md`, `scripts/`).
- Standard names keep their conventional form (e.g. `Cargo.toml`, `README.md`, `CLAUDE.md`).
- File and directory names must be in English.

## Language

- Everything in the project (code, identifiers, comments, documentation, commit messages) must be written in English.
- Exception: the translations of the user interface, if the project has any. Only the translated strings and their comments are in the other language; the code stays in English.
- If the user interface is translated, its texts are never hard-coded: each message goes into the translation file of every language (English is the reference), read with the library chosen in `docs/technical-choices.md`.
- Exception: the code review notes in `outputs/` are written in French.

## Specifications and roadmap

- Before writing code, describe the expected behavior of the application in `specs.md`, then split the work into steps in `roadmap.md`, and write the technical choices and their reasons in `docs/technical-choices.md`.
- At the start of each session, read `roadmap.md` to know the progress; when asked to continue, resume at the first unchecked step.
- Each step is delivered in its own pull request, which checks the step's box in `roadmap.md`.
- Keep `roadmap.md` consistent with `specs.md` when the specifications change.
- Keep `roadmap.md` short, since it is read at every session: the steps to come in detail, one line per step done with its pull requests. When the pull request of a step (or of a sub-step) is merged, move the detail of the step to `docs/roadmap-archive.md`. Reference notes (technical choices, rules of the external services) live in `docs/`, linked from `roadmap.md`, and are read only by the steps that need them.
- Read `specs.md` by section: list its headings first (`grep -n '^#' specs.md`), then read only the sections the step needs.
- When the pull request of a step (or of a sub-step, such as 10b) is merged, tell the owner that the step is done and remind them to close the session and start a new one for the next step: a fresh session costs far fewer tokens, and `roadmap.md` and the memory hold what is needed to resume.

## Interface design

- Draw the interface before coding it: first an editable grayscale wireframe in Excalidraw (`docs/wireframes/`, `excalidraw-wireframes` skill), which the owner edits too; then, once the layout is agreed, the interactive HTML mock-up (`docs/mockups/`, `html-mockups` skill).
- For a desktop application, decide in the first wireframes whether the window keeps the title bar of the system or draws its own, and draw its buttons from the start.
- The VS Code Excalidraw extension doesn't reload a file changed on disk: before rewriting a wireframe the owner has open, ask them to save it and close its tab, and commit their version first.
- A mock-up is split into small files (the shared styles and scripts, one style sheet and one script per block, one script per section), so that a change reads only what it needs.
- Once approved, the mock-up (its page and its folder) is the single source of truth of the interface. A design change goes into the mock-up first, then into `specs.md` and into the code when that part is already coded. The wireframe only fixed the first layout and isn't kept up to date.
- Check a visual change with a few screenshots, of the window concerned only, and delete them once looked at: they are for this check, not for the owner, who looks at the mock-up and the application themselves.

## Connectors

- The claude.ai connectors (Claude Docs and the others) are turned off for this project in `.claude/settings.json` (`disableClaudeAiConnectors`): their instructions and tools would be sent with every message. Remove the setting only if the project needs one of them.
- The skills a project doesn't use (documents, slides, spreadsheets, browsers, charts, Claude API…) are hidden from the model in the same file (`skillOverrides`, `user-invocable-only`): their descriptions would be sent with every message. The owner can still run them with `/<name>`; to give one back to the model, remove its line or use `/skills`.

## Working folders

Two folders are the exchange area between the owner and Claude Code. They are ignored by Git, except their `.gitkeep`, and hold only temporary files.

- `inputs/`: what the owner gives for context: screenshots, examples, references, copied texts. Read them when the owner points to them; never commit, move or delete them unless asked.
- `outputs/`: what Claude Code produces for the owner: reports, code review notes, previews, screenshots. Name them `<YYYY-MM-DD>-<subject>.<ext>` when they are meant to be kept for a while.
- Anything meant to last goes elsewhere and is committed: documentation in `docs/`, images used by the application in `assets/`, test data next to the tests.

## Git workflow

- Never commit or push directly to `main`.
- For each task, create a dedicated branch from `main`, commit and push only on that branch.
- Every change must go through a pull request targeting `main`.
- Name branches `<type>/<short-description>` in kebab-case, using the commit types below (e.g. `feat/dark-mode`, `fix/settings-save`).
- Never switch branches in the main project directory while the owner may be testing a branch there (for example with `cargo run`): it would change the code they run. Work on any other branch in a separate Git worktree outside the project (`git worktree add <directory> -b <branch> origin/main`), and remove it with `git worktree remove <directory>` once the work is pushed.
- When the owner says they aren't running or testing anything, work on the task branch in the main project directory instead, so that they can follow the files changing in their editor; switch it back to `main` once the pull request is merged.

## Commit messages

Follow [Conventional Commits 1.0.0](https://www.conventionalcommits.org/en/v1.0.0/). Pull request titles use the same format.

```
<type>(<optional scope>): <description>

<optional body: what and why>

<optional footer>
```

| Type | Use for |
|---|---|
| `feat` | A new feature for the user. |
| `fix` | A bug fix. |
| `docs` | Documentation only (`README.md`, `specs.md`, `roadmap.md`, `CLAUDE.md`, doc comments). |
| `style` | Formatting with no code change. |
| `refactor` | A code change that neither fixes a bug nor adds a feature. |
| `perf` | A performance improvement. |
| `test` | Adding or fixing tests only. |
| `build` | Build system or dependencies (`Cargo.toml`, `Cargo.lock`). |
| `ci` | CI configuration (`.github/`). |
| `chore` | Other maintenance (tooling, `.gitignore`, Claude Code agents). |
| `revert` | Reverting a previous commit. |

- Scope: optional, the module or area concerned (e.g. `feat(settings): ...`, `fix(parser): ...`).
- Description: imperative mood, lowercase, no final period, 72 characters max (e.g. `feat(settings): save the settings atomically`).
- Breaking change: add `!` after the type or scope (e.g. `feat(settings)!: ...`) and explain it in a `BREAKING CHANGE:` footer.

## Code review

- Before opening any pull request that changes code or its build (`src/`, tests, `Cargo.toml`, `build.rs`, `rust-toolchain.toml`, `.github/workflows/`), run the `code-reviewer` subagent (`.claude/agents/code-reviewer.md`) on the branch.
- Run it once per pull request, on the whole branch, to save tokens; a step too large for one review is split into sub-steps in `roadmap.md`.
- It writes its recommendations, in French, to `outputs/<YYYY-MM-DD>-<branch>-code-review.md`. These notes are ignored by Git.
- Summarize the recommendations to the user and apply only the ones they approve, then open the pull request.

## Development

- Before committing, run and make sure all of these pass:
  - `cargo fmt --all --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-features`
- The same checks run in CI (`.github/workflows/ci.yml`) on every pull request.
- `unsafe` code is forbidden and Clippy pedantic lints are enabled (see `[lints]` in `Cargo.toml`).
- `rust-version` in `Cargo.toml` is the minimum Rust version: CI also builds with it. Raise it when a dependency or a language feature needs a newer Rust.

## Releases

- Pushing a version tag (e.g. `v0.1.0`) that matches the version of `Cargo.toml` publishes a GitHub release with the executables for Windows, macOS and Linux (`.github/workflows/release.yml`).
- This workflow suits a command-line or terminal application. For a desktop application (installers) or a web application (deployment), adapt or replace it, and record the choice in `roadmap.md`.
