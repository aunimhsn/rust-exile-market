# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Reference

Read only by the steps that need them:

- [Technical choices](docs/technical-choices.md): the libraries, the services and the main designs, with their reasons.
- [Images](docs/images.md): the game images the application may show, and the rules of Grinding Gear Games.
- [poe2scout usage](docs/poe2scout-usage.md): the limits of the price database and the rules the application follows.
- [Trade site usage](docs/trade-site-usage.md): the risk accepted by the owner, the limits and the guardrails.
- [Game data](docs/game-data.md): the stat ids and the modifier tiers, their sources and how they fit together.
- [Roadmap archive](docs/roadmap-archive.md): the detail of the steps done.

## Path of Exile 2 1.0

Path of Exile 2 leaves early access with its version 1.0 on 2026-12-11. The modifiers, their tiers and ranges, the trade site data and maybe the format of the copied text can change. After this release:

- copy real items again (English and French) and rerun the parser tests;
- check the stat ids, the pseudo stats and the filters of the trade site, and the categories of poe2scout;
- check the tiers and ranges given by the advanced copy, and generate the game data again (stat ids and modifier tables).

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
- [ ] **13. Estimated prices**, in eleven pull requests, from the [game data](docs/game-data.md):
  - [x] **13a, the game data** (#31)
  - [x] **13b, the stat ids** (#32)
  - [x] **13c, the better side of a line** (#34)
  - [x] **13d, the trade site templates** (#36)
  - [x] **13e1, the range field** (#39), as in the mock-up (decided on 2026-10-27): the minimum field becomes a range field "min – max" (a filter has a minimum and a maximum, kept within the range of the stat; a minimum above the maximum brings the other half to it), three characters a side, four on the row of a stat whose range reaches them, centered, no placeholder, "min – max" over the column; a line better lower (read from its words, as in the offer tooltip) gets its maximum instead of its minimum; the tooltip of the field ("Rolls 40 to 60", "Lower is better"); the bar drawn by a style value (`Position` now, the zone searched for step 18).
  - [ ] **13e2, the uniques searched with their filters**: the checked filters and their minimums and maximums in the search and in the link of Check on trade (until then, a unique is searched by its name, base and corrupted state); each value of a stat linked to its roll (its sign, a `reduced` line, an inverted stat: the minimum and the maximum swap), an option stat searched by its option (EE2 stores some ids with `|option`); a line of several numbers searched at their average; a local and a global stat in a `count` group of minimum 1; a line without trade id shown with the circle struck through and its tooltip, not a filter; the minimum and the maximum read for the search with a minimum never above the maximum, even when the player didn't leave the half (the range field brings them together only then).
  - [ ] **13f, the filters of the rare and magic items and jewels**: every modifier at 90 % of the copied value (since 10b), a line better lower at 110 % as its maximum (since 13e1), the base type checked, the item level unchecked (the **Item** group); for a weapon, the **Weapon** group: the physical DPS checked, the elemental DPS and the damage and attack speed modifiers unchecked; for a jewel, all its modifiers.
  - [ ] **13g, the pseudo totals**: the **Pseudo** group (total elemental resistance, maximum life, attributes…), its totals added up from the modifiers, checked, and the modifiers they add up unchecked; a table of rules with its tests.
  - [ ] **13h, the normal items**: the base type, the sockets and the item level checked, the quality unchecked; how to search an exceptional base exactly (its name or its number of sockets), checked on the trade site.
  - [ ] **13i, the estimated price**: the text before the search, then **Estimated price**: the median of the 5 cheapest similar offers, suspicious offers excluded, "18 similar", "Median of the 5 cheapest similar offers.", the uncertainty warning under 5; how an offer is suspicious when the reference price is the estimate itself (to decide with the owner).
  - [ ] **13j, the modifier tables**: the modifiers and bases of RePoE in the generator; the roll bar of the rare and magic modifiers over every tier of the base, the range field kept within the range of every tier and its tooltip with the rolls of every tier ("Rolls 10 (T11) to 174 (T1)"), the item class as a type (the category of each base type from the items of Exiled Exchange 2, with its English name), their tier and range read from the tables when the item was copied without the advanced copy (to decide).
- [ ] **14. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
- [ ] **15. Update notice**: at launch, ask the GitHub releases for the latest version, and when it is newer than the running one, a bubble "Version 0.2.0 available" with a link to its download page, closed for good per version (like the bubble of a new league); mock-up first. Needed before the first release: a player kept on an old version broken by a change of the game (Path of Exile 2 1.0, a new copied text format) or of the trade site wouldn't know it.
- [ ] **16. First release**: executable name (without the `rust-` prefix) and application id from one constant, application icon from the emblem, a readable message when the window can't open (no OpenGL, no X11 / XWayland), README usage (with where the log is), Wayland help, clipboard help and services called, version 0.1.0.
- [ ] **17. Overlay POC**: after the first release, a proof of concept of the price shown over the game, the strongest point of Exiled Exchange 2, to compete with it there too.
- [ ] **18. More settings**: after the first release, the settings the players may want, gathered while building the application; first, turning the pseudo stats on or off among the filters of the rare items (their default to decide then); the roll bar of the stats drawn as the zone searched, for the players coming from Exiled Exchange 2 (the zone between the minimum and the maximum colored, a tick at the roll, the lowest value on the left), instead of the position of the roll, the default (mock-up first: this variant was drawn and compared on 2026-10-27, in step 13e).
- [ ] **19. Automatic updates**: after the first release, if the application finds its players: the new version downloaded in the background and installed at the next launch, like Discord, with a signature checked before installing anything; Velopack (a real installer for Windows and macOS) or the `self_update` crate (the executable replaced), and Flathub for Linux, whose system updates it; to decide with the installers of the first release.
- [ ] **20. Polish**: the last touches, at the very end: a tooltip on the DPS of a weapon telling it counts its quality, modifiers and runes ("With its quality, modifiers and runes"), so that it isn't taken for the DPS of the base; the tests of the large modules moved to a file of their own, as for the parser (`src/item/parser/tests.rs`), starting with `src/roll_difference.rs` (some 800 lines, most of them tests), so that reading the code costs fewer tokens.
- [ ] **21. Price fallback**: a plan B, after everything else, for when poe2scout can't be reached: the price of a currency or a unique from the trade site (its currency exchange, or the median of its cheapest offers), on a click only, with a search block of its own in the price block; the cached prices stay shown with their age until then. Within the [Trade site usage](docs/trade-site-usage.md) guardrails: never when an item is copied, never in the background, never a local copy of poe2scout (poe2scout reads the currency exchange API of Grinding Gear Games and the trade site from one server; every player doing it would be far more requests). Mock-up first.
