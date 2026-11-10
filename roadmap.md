# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Reference

Read only by the steps that need them:

- [Technical choices](docs/technical-choices.md): the libraries, the services and the main designs, with their reasons.
- [Releases](docs/releases.md): how to publish a version, and what its archives hold.
- [Images](docs/images.md): the game images the application may show, and the rules of Grinding Gear Games.
- [poe2scout usage](docs/poe2scout-usage.md): the limits of the price database and the rules the application follows.
- [Trade site usage](docs/trade-site-usage.md): the risk accepted by the owner, the limits and the guardrails.
- [Game data](docs/game-data.md): the stat ids and the modifier tiers, their sources and how they fit together.
- [Backlog](backlog.md): what is noted and not being done; read when the next work is chosen (steps 19 and 21).
- [Lessons](docs/lessons.md): what working on the project taught, kept for the retrospective (step 22).
- [Roadmap archive](docs/roadmap-archive.md): the detail of the steps done.

## Path of Exile 2 1.0

Path of Exile 2 leaves early access with its version 1.0 on 2026-12-11. The modifiers, their tiers and ranges, the trade site data and maybe the format of the copied text can change. After this release:

- copy real items again (English and French) and rerun the parser tests;
- check the stat ids, the pseudo stats and the filters of the trade site, and the categories of poe2scout;
- see whether a macOS client comes with 1.0 (none on 2026-11-01: `"mac": false` on Steam for the app 2694490, while Path of Exile 1 has one; the players of a Mac run the Windows client through CrossOver or Whisky). If one does, the price-check hotkey on macOS (left out of step 14a2 by the owner on 2026-11-01): `global-hotkey` (Carbon) and the copy sent with CGEvent through `objc2-core-graphics`, already a dependency of `arboard`, whose functions are safe (`CGEvent::new_keyboard_event`, `set_flags`, `post`, Ctrl read with `CGEventSource::flags_state`); the copy of the game is Ctrl+C there too (`Ctrl + C` on every system in Exiled Exchange 2, which presses Ctrl again on a Mac); the Accessibility permission, needed to send keys, asked at the first Ctrl+D only (`CGPreflightPostEventAccess`, `CGRequestPostEventAccess`);
- check the names of the properties of a waystone (`waystone_properties` in `src/item/labels.rs`; French `Efficacité des Monstres` from Exiled Exchange 2, not seen in a copy yet, but written so by the French trade site on 2026-10-31), the same names among the `properties` of `/fetch` on both trade sites (read for the tooltip of an offer, 13k5c2) and their ids among the `map_filters` of the trade site (`/data/filters`; `map_rare_monsters` is "Monster Rarity" and `map_magic_monsters` "Monster Effectiveness", checked by two searches on 2026-10-31): an unknown name is logged (`ParseWarning::UnknownWaystoneProperty`);
- check the properties of a gem in `/fetch` (`Level` type 5, the quality type 6, `gemSockets`) and the types of its `requirements` (62 to 65; the strength, type 63, not seen yet), read for the tooltip of an offer (13k5d2);
- check the tiers and ranges given by the advanced copy, and generate the game data again (stat ids and modifier tables); then decide on the `divisor` of a line without a number, which no table holds (parser review of 17e3, recommendation 14);
- check the bases of the modifiers of the essences and of the alloys (`ESSENCES` and `ALLOYS` in `tools/game-data/src/modifiers.rs`) against Craft of Exile, poe2db and the trade site, and add the new ones;
- compare the crafts of the game (the crafted modifiers that aren't an alloy's, e.g. the `Motivating` of the owner's sceptre) with the lines of the alloys: crafts have no table, so a line marked crafted from the in-game trade that only an alloy holds is named "Alloy" (`Table::may_hold` in `src/game_data/modifiers.rs`; no case known on 2026-10-31, second code review of 13k3d2); if a craft shares a line and a range with an alloy on a base, name no currency for that line.

## Steps

- [x] **0. Mock-ups** (#4)
- [x] **1. Application window** (#5)
- [x] **2. Clipboard watching** (#7)
- [x] **3. Item parser** (#9)
- [x] **4. Interface language** (#10)
- [x] **5. Settings** (#11)
- [x] **6. Title bar** (#12, #13, #14)
- [x] **7. Price data** (#15, #16)
- [x] **8. French item names** (#17)
- [x] **9. Price display** (#18, #19)
- [x] **10. Item stats and filters** (#20, #23, #25, #26)
- [x] **11. Trade site client** (#28)
- [x] **12. Offers display** (#29, #30)
- [x] **13. Estimated prices** (fifty-four pull requests, #31 to #148)
- [x] **14. Overlay POC** (fifteen pull requests, #150 to #165)
- [x] **15. Update notice** (#166)
- [x] **16. Bug report** (#167, #168, #169)
- [ ] **17. First release**: version 0.1.0, to a small audience about a month before Path of Exile 2 1.0 (2026-12-11). In six sub-steps (owner's choices on 2026-11-05: three pull requests, then the site added after 17a, and the audit before the tag):
  - [x] **17a, the executable** (#170)
  - [x] **17a2, the new icon** (#179)
  - [x] **17b, the hosting of the site** (outside this repository)
  - [ ] **17c, the content of the page**: the page that presents the tool (what it does, screenshots, the download, the display mode the game needs, the disclaimer of Grinding Gear Games, never naming another application), in its own repository `aunimhsn/exile-market.sylent.tools` and its own Claude Code session (`../exile-market.sylent.tools`, GitHub Pages): one page, English at the root and French under `/fr/`, the anonymous counter and the email list of the launch plan later, the screenshots of the application last. Checked here once the page is online. The download page then replaces the page of sylent.art of the launch plan, and the README, the release notes and maybe the link "sylent.art" of the status bar of the settings (`AUTHOR_SITE` in `src/main.rs`; mock-up first if it changes) point to it.
  - [x] **17d, the process, the license and the README** (#180, #181)
  - [ ] **17e, the audit of the whole repository**: the first review of the whole repository at once, before the first executable reaches players (every review so far read one branch). In three sub-steps (owner's choices on 2026-11-09):
    - [x] **17e1, the agents and the rule** (#182)
    - [x] **17e2, the security audit** (#183)
    - [ ] **17e3, the quality review**: `code-reviewer` run area by area on 2026-11-10, in five passes (the parser and the game data; the interface; the network; the desktop; the rest), its notes in French in `outputs/2026-11-10-<area>-area-review.md` (`parser`, `interface`, `network`, `desktop`, `core`): 3 high, 23 medium and 26 low recommendations, no blocking bug. Every recommendation is applied (owner's choice on 2026-11-10), but the grouping of the state of `app.rs`, left for after the release ([backlog.md](backlog.md)). In four sub-steps, each with its `code-reviewer` pass on the branch (a second one when the guardrails of the trade site or the matching of the game data change); no second review of the five areas after them:
      - [x] **17e3a, the behavior fixes** (#186)
      - [ ] **17e3b, the tests**, in two pull requests (2026-11-10), the game data apart, whose matching is critical and reviewed twice:
        - [x] **17e3b1, the tests of the game data** (#187)
        - [ ] **17e3b2, the other tests**: the network: the real limit headers read by a pure function, no cookie sent back, the `User-Agent`, the pace, a seam under `Poe2Scout` for the order, the paging, the stop at the first failure, the leagues read whole and the failure after answers; the tests that depend on real time. The settings: a real unknown key, the file of the v0.1.0 as a fixture, a failed write. The interface: a smoke test that draws the item view with every test item, in both languages. The desktop: the predicate of the pointer window of Wine. `clippy::unwrap_used` for the workspace. The assumptions checked on KWin only, one line each in the guides of step 17f.
      - [ ] **17e3c, the two warnings**: mock-up first (two or three options each), then `specs.md` and the code: the price of an exchange item traded too few times (355 of the 638 exchange items of the owner's cache on 2026-11-09 have fewer than 10 trades; the warning exists for a unique only), and a search the trade site refused or answered in a way the application can't read, told apart from "can't be reached". With them, since a visible text goes through the mock-up first, two details of the French wording (interface review, recommendation 7): "Recherchez" next to "Chercher" in five messages, and "recherche dans" of `offers-limit` where the two other messages say "prochaine recherche dans".
      - [ ] **17e3d, the splits**: moves only, no change of behavior, after the two warnings so that the code moved is final; one pull request per area if they are many, decided at its start. `ui/item_stats.rs` (the model, the properties, the modifiers, the rolls, the view), `game_data/modifiers.rs` (tables, copied, lookup, trade, splits, rewrite), `tools/game-data/src/modifiers.rs` (its tests, the currencies, the lines), `hotkey.rs` (the keys), `hotkey/x11.rs` (the active window, the copy), `overlay/activation.rs` (the X screen), the pure helpers of `app.rs`, the tests past a few hundred lines to their `tests.rs`; `src/price/` for `prices`, `pricing` and `estimate`, `offers.rs` and `currencies.rs` renamed; the item names thread out of `trade/mod.rs`; one guarded transport for the refusal of the trade site; one X11 session; the words of a Time-Lost jewel into `item/labels.rs`; the repetitions (`widgets`, the counted rolls, `OffersState::Items`, the per-language values), the `messages!` macro of `i18n.rs`, the macOS scaffolding, `Written` in `tier_rolls.rs`, the leftovers (`RollBar`, `quiet_tier`, `List.exchange`), and the one-line notes "outside the area" of the five reviews.
  - [ ] **17f, the checks and the tag**: every guide of `docs/tests/windows/` run at once by the Windows tester, not at hand before (owner, 2026-11-03; if they aren't available in time, a short smoke test on Windows, and the release notes say Windows is less tested); the window over the game tried on GNOME (Mutter), its keyboard above all: the application gives the keyboard back to the game only while the desktop tells the game as its active window, or none (KWin keeps the game; Mutter unchecked, second code review of 14b2); the region selection of a screenshot tool (KDE Spectacle) opens under the window over the game, an override-redirect window above everything (found by the owner on 2026-11-01; the full-screen capture takes it): check what Exiled Exchange 2 does, and whether the window should give way (hidden while a screenshot tool runs, or nothing to do); the fixes found; the commitments to third parties written down (`docs/poe2scout-usage.md`: tell the poe2scout maintainers on their Discord once the releases reach 100 downloads); the release notes (the display mode the game needs, as on the download page of sylent.art next to the link to the release), then the tag `v0.1.0`. Before the tag, the builds of the Release workflow on macOS checked green: they aren't required on the pull requests, whose lint for macOS runs from Linux.
- [ ] **18. Donation method**: after the first release, brainstorm with the owner the ways the players could support the project (GitHub Sponsors, Ko-fi, Patreon, Liberapay, Open Collective…), compare them (fees, payouts to the owner, one-off or monthly gifts, what the rules of Grinding Gear Games allow a third-party tool: never a feature in exchange for money), and choose one; then where it shows: the `README.md`, the GitHub sponsor button (`.github/FUNDING.yml`), and maybe a discreet link in the application (mock-up first). On the download page, a banner is designed and waits for this choice (`backlog.md` of `exile-market.sylent.tools`): its link says « Support it on Ko-fi », to change if another service is chosen.
- [ ] **19. More settings**: after the first release, the settings the players may want: the entries of [backlog.md](backlog.md) (Features, Settings), chosen with the owner from what the players ask.
- [ ] **20. Automatic updates**: after the first release, if the application finds its players: the new version downloaded in the background and installed at the next launch, like Discord, with a signature checked before installing anything; Velopack (a real installer for Windows and macOS) or the `self_update` crate (the executable replaced), and Flathub for Linux, whose system updates it; to decide with the installers of the first release.
- [ ] **21. Polish**: the last touches, picked with the owner from [backlog.md](backlog.md): its sections Polish, Edge cases and Waiting on others, and the features of step 21.
- [ ] **22. Retrospective**: right after the polish, the lessons of [docs/lessons.md](docs/lessons.md) sorted one by one, with the owner: into a project template of a Rust desktop application (to create, with the technologies chosen here: egui and eframe, X11 on Linux, Fluent, the Excalidraw wireframes and the HTML mock-ups, the CI and the releases, the agents, the skills, the settings that hide the unused skills and connectors), into the `CLAUDE.md`, the skills or the agents of this project, or dropped; the rule that writes the lessons goes into the template too. Only a command-line template and a static website template exist today.
- [ ] **23. Price fallback**: a plan B, after the polish, for when poe2scout can't be reached: the price of a currency or a unique from the trade site (its currency exchange, or the median of its cheapest offers), on a click only, with a search block of its own in the price block; the cached prices stay shown with their age until then. Within the [Trade site usage](docs/trade-site-usage.md) guardrails: never when an item is copied, never in the background, never a local copy of poe2scout (poe2scout reads the currency exchange API of Grinding Gear Games and the trade site from one server; every player doing it would be far more requests). Mock-up first.
- [ ] **24. Responsive layout and price history**: at the very end, after the price fallback (owner's choice on 2026-10-31): wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
