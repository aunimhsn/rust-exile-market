# Code quality

How the code of Exile Market is written. `CLAUDE.md` holds the conventions of the project (files, language, commits, checks); this file holds the rules of the code itself.

Read it at the start of each step that changes code; `code-reviewer` reviews against it. A rule is followed, or the pull request says why not. Clippy (pedantic) and `rustfmt` already enforce their own rules: they aren't repeated here.

## Names

The name comes first: a reader who sees only the name should know what the thing is for.

- A name says what a thing is or does, in the words of the domain (the game, the trade site, poe2scout), not how it is computed: `affix_limits`, not `counts`; `copied_lines`, not `list`.
- A module or a type is named by what it gives, not by its place next to another one: no module `other_…`, `misc`, `utils` nor `common` (the module that tells the lines in doubt is `doubts`, not `other_readings`). `other` stays the word for the second of a pair (`other_fit`, `other_room`).
- Whole words. No abbreviation but those of the domain and of Rust: `id`, `dps`, `ui`, `ctx`, `i18n`. A single letter only for an index or a closure parameter of a line or two.
- One notion, one word, across the whole code: a tier is always a `tier`. Two notions never share a word: when one word means two things (`currency` names four), one of them is renamed.
- Types are nouns (`ModifierTiers`). A function that computes a value is named by what it returns (`trade_tiers`, `possible`); a function that acts, by a verb (`save`, `send_copy`).
- A predicate reads as a question: `is_`, `has_`, `can_`, `may_`, or a verb in the third person (`rolls_on`, `holds`). A boolean field or variable too: `is_fractured`, not `flag`.
- Conversions follow the Rust API guidelines: `as_` borrows for free, `to_` costs, `into_` consumes, `from` builds. Constructors: `new`, `with_…`, `of(…)`.
- The path of a module is part of the name: `price::estimate`, not `price::price_estimate`.
- A number with a unit says it: a `Duration`, or a suffix (`_ms`, `_pixels`, `_points`).
- A test is named by the behavior it pins, as a sentence: `a_sanctified_item_gets_no_tier_from_its_values`.
- A name the code has outgrown is renamed in the change that outgrew it.

## Functions

- One function does one thing, at one level of detail, and reads from top to bottom. Past about 50 lines or three levels of nesting, look for the part that deserves a name.
- The case that stops returns early (`let … else`, `?`), so the main path isn't nested.
- At most four parameters, `self` included.
- Two parameters of the same type side by side become a struct or an enum when a call can swap them unseen: literals (`f(true, false)`, `f(3, 7)`), or values without a name of their own at the call. Two values named at the call (`english`, `french`) may stay.
- No boolean parameter that switches what a function does: two functions, or an enum named by its cases.
- The decision apart from the wiring: the rule in a pure function, without egui, threads or I/O, and tested; the code that calls it stays thin.

## Types

- Invalid states can't be written: an enum, not two fields always set together or always opposite, not flags that cross.
- `match` on the enums of the project are exhaustive: no `_ =>` that would swallow a new variant. Adding a variant must point to every place to update.
- A newtype when two values of one type can be mixed up (the index of a line and the index of a candidate; pixels and points), or when a value carries an invariant checked once, where it is built.
- `Option` and `Result`, never a sentinel (`-1`, `""`, `NaN`).
- The smallest visibility: private, then `pub(super)`, then `pub`. In a binary crate, as here, `pub` reaches the crate only, as `pub(crate)` would: one spelling, `pub`. A `pub` field only on a plain data type.

## Ownership and borrowing

- Borrow by default: `&str`, `&[T]` and `&T` parameters, `impl Iterator` returns. Own (`String`, `Vec`) only what outlives its source or crosses a thread.
- A `clone()` is a choice: to cross a thread, to keep a value past its source, or a cheap copy (`Copy`, `Arc`). Never to quiet the borrow checker: then the structure is wrong, and a struct split in two lets its fields be borrowed apart.
- `Cow` when a value is mostly borrowed and sometimes built.
- Data between threads: owned by one thread, sent through a channel. `Arc<Mutex<…>>` only where a channel can't do, with the reason in a comment.

## Modules

- One file, one responsibility, named by it. A file that holds two subjects that change for different reasons is split: the data kept by hand apart from the code that reads it, the rules apart from the drawing.
- Past about 800 lines of code (tests aside), a file is looked at for its parts. The length is a sign; the responsibilities decide.
- The tests of a module past a few hundred lines go to its `tests.rs` (`CLAUDE.md`).
- The dependencies go one way: the domain (`item`, `game_data`, the prices, `trade`) knows neither egui nor `ui` nor `app`; the interface reads the domain.
- Reviewing one rule takes reading its file and its tests, not a whole area.

## Rules of the domain

- One rule in one place: a rule of the game or of the trade site (a threshold, a bound, a list of words) is written once and called by every use. Each copy is a place where the application can drift.
- The words of the game, in each language, live in `src/item/labels.rs`; the texts of the interface in `locales/`, never in the code.
- A number of the rules gets a named constant, with a comment that says where it comes from (a measure, the game, the mock-up).

## Errors

- A `Result` carries its context: what was being done, on what.
- No `unwrap` outside the tests (Clippy `unwrap_used`); an `expect` only for an invariant, which its message states.
- An error is caught where its information still is (the status and the message of a response), not flattened into one variant.
- The world is expected to be wrong at times (the copied text, the game, the network): it is handled gently. A broken invariant is a bug: it shows.

## Comments

- Each module, type, function and constant has a doc comment that says what it is for, in one sentence, before how; but `main`, the tests (their name is their sentence) and the methods of a trait `impl`, which their module and their trait document. A field or a variant has one when its name and its type don't say it all.
- A comment in a body says why, never what the code already says.
- A comment that no longer matches the code is a bug, fixed in the same change.
- The code says the rule of today. The source of a fact stays next to it (what was measured, where and when), since it tells when to check again; the story of the decisions (what was tried, who chose, what changed) lives in `docs/`.

## Patterns

- Simple first (KISS): a function before a trait, an enum before a trait object, a struct literal before a builder.
- A pattern only once its value is diagnosed, and that value written in the pull request:
  - a trait at a seam a test must replace (the network, X11, the clock);
  - an enum for a closed set of cases;
  - a newtype for an invariant;
  - a builder for a value of many optional parts (the data of the tests);
  - a typestate where an order matters and a mistake would be costly.
- Never a layer that adds without giving: no trait with one implementation and no test double, no generic used with one type.
- SOLID, read for Rust: one responsibility per module and per type; a new case added by a new variant or a new `impl`, the exhaustive `match` naming every place to update; small traits; a trait at a seam, not everywhere.

## Tests

- A test reads as an example of its rule: its name says the behavior, its body the case, built by a small helper rather than fifty lines of set-up.
- Each rule is tested on data written by hand, in the `tests.rs` of its file. The tests on the real data of the game are kept for what a patch of the game changes, in a `game_tests.rs` next to it: the ones to look at, and to record again, when the game data are generated again, while the `tests.rs` must still pass as they are. What the tests of several files share (the builders of their data, the readers of a result) lives in the `fixtures.rs` of their module.
- An assertion compares the whole value (a list, a section, a struct); a part (`contains`, `starts_with`) only when the rest doesn't matter. An expected exception is a named list, compared whole.
- Several cases of one behavior may share a test; two behaviors are two tests.
- Mutations are no part of the usual flow, which the code review and the CI guard. A series is run only on the owner's go, asked beforehand with what it would bring on this code, how many mutations and about how long: it takes time and slows the step. A mutation is one small wrong change of the code, the nearest wrong neighbour of a rule (the best tier for the tier found, a `>=` for a `==`), which a test of the file must catch. One that survives is a question, not yet a missing test: a case to write, or code that does nothing; it is handed to the code review without the author's opinion. A series runs in a worktree apart, never in the project folder, so that no mutation can be left in the code, and each run of the tests is bounded in time and in memory, since a mutation can make a loop endless.

## Refactoring

- No change of behavior, but the small fixes of the last rule, each in its commit: the tests, the counts of the draw (`RECORDED`) and the generated game data stay the same, checked after each commit, not only at the end, so that a difference names its commit. Where a commit changes the generator (`tools/game-data`), run it after that commit at the commits the `NOTICE.md` of `main` names (without arguments, it reads `master`, which moves), and compare its files and what it prints, byte for byte. A tool that reads the trade site (`tools/exchange-icons`) isn't run for a refactoring: the next rule applies to it. A test whose assertion changes is named in the pull request, with the reason.
- Before moving code, check it is tested; a path without a test gets one first, or, where only the network or the game reaches it, it is changed only by steps the compiler checks, and the pull request says so.
- The moves first, in commits of their own: `git diff --color-moved=dimmed-zebra --color-moved-ws=allow-indentation-change` shows them unchanged, but for paths, `use` lines, visibility and the line breaks `rustfmt` puts again, the same tokens. Then one commit per improvement.
- Each small refactoring seen on the way is done in the same pull request. A small fix of the behavior seen on the way, with its test, goes in a commit of its own, named in the pull request as its change of behavior and checked by its review; one that needs a decision or a mock-up goes to `backlog.md`.
