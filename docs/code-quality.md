# Code quality

How the code of Exile Market is written. `CLAUDE.md` holds the conventions of the project (files, language, commits, checks); this file holds the rules of the code itself.

Read it at the start of each step that changes code; `code-reviewer` reviews against it. A rule is followed, or the pull request says why not. Clippy (pedantic) and `rustfmt` already enforce their own rules: they aren't repeated here.

## Names

The name comes first: a reader who sees only the name should know what the thing is for.

- A name says what a thing is or does, in the words of the domain (the game, the trade site, poe2scout), not how it is computed: `affix_limits`, not `counts`; `copied_lines`, not `list`.
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
- At most four parameters. Two of the same type side by side (`bool, bool`, `usize, usize`) become a struct or an enum.
- No boolean parameter that switches what a function does: two functions, or an enum named by its cases.
- The decision apart from the wiring: the rule in a pure function, without egui, threads or I/O, and tested; the code that calls it stays thin.

## Types

- Invalid states can't be written: an enum, not two fields always set together or always opposite, not flags that cross.
- `match` on the enums of the project are exhaustive: no `_ =>` that would swallow a new variant. Adding a variant must point to every place to update.
- A newtype when two values of one type can be mixed up (the index of a line and the index of a candidate; pixels and points), or when a value carries an invariant checked once, where it is built.
- `Option` and `Result`, never a sentinel (`-1`, `""`, `NaN`).
- The smallest visibility: private, then `pub(super)`, then `pub(crate)`. A `pub` field only on a plain data type.

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

- Each item has a doc comment that says what it is for, in one sentence, before how.
- A comment in a body says why, never what the code already says.
- A comment that no longer matches the code is a bug, fixed in the same change.
- The code says the rule of today; the history of its decisions, and their dates, live in `docs/`.

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
- Each rule is tested on data written by hand. The tests on the real data of the game are kept for what a patch of the game changes.
- A test fails for one reason: one behavior per test.

## Refactoring

- No change of behavior: the tests, the counts of the draw (`RECORDED`) and the generated game data stay the same. A test whose assertion changes is named in the pull request, with the reason.
- Before moving code, check it is tested; a path without a test gets one first.
- The moves first, in commits of their own: `git diff --color-moved=dimmed-zebra` shows them unchanged, but for paths, `use` lines and visibility. Then one commit per improvement.
- Each small refactoring seen on the way is done in the same pull request. One that would change the behavior goes to `backlog.md`.
