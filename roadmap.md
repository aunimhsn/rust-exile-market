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
- [ ] **13. Estimated prices**, in fifteen pull requests, from the [game data](docs/game-data.md):
  - [x] **13a, the game data** (#31)
  - [x] **13b, the stat ids** (#32)
  - [x] **13c, the better side of a line** (#34)
  - [x] **13d, the trade site templates** (#36)
  - [x] **13e1, the range field** (#39)
  - [x] **13e2, the uniques searched with their filters** (#42)
  - [x] **13f, the filters of the rare and magic items and jewels** (#44)
  - [x] **13g, the pseudo totals** (#47)
  - [x] **13h, the normal items, the sockets, the quality and the required level** (#49)
  - [x] **13i, the estimated price** (#51)
  - [ ] **13j, the modifier tables**, in four pull requests:
    - [ ] **13j1, the tables**: the modifiers and bases of RePoE in the generator (`modifiers.ndjson`, the tags of each base in `bases-<language>.ndjson`), and every tier of a prefix or suffix on the base of its item, read by the application (`game_data::modifier_tiers`); nothing visible yet. Checked against the tiers of the advanced copy and of the offers of the trade site.
    - [ ] **13j2, the roll bar and the range field over every tier**: mock-up first (the bar of the rare and magic modifiers); the bar over every tier of the base, the range field kept within the range of every tier, and its tooltip with the rolls of every tier ("Rolls 10 (T11) to 174 (T1)").
    - [ ] **13j3, the item class as a type**: the category of each base type (generated in 13h, with its English name), searched in place of the base type when the player unchecks it (a bow among all the bows; until then, any base); its id checked on the trade site.
    - [ ] **13j4, the tier without the advanced copy**: to decide first, whether the tier and its range are read from the tables when the item was copied with Ctrl+C (tiers that overlap, the prefix and the suffix of rarity of a ring, the lines of a hybrid copied as modifiers of their own); dropped if not.
  - [ ] **13k, the modifiers of the endgame mechanics**: every edge case of a modifier tied to a mechanic, studied first in Exiled Exchange 2, RePoE and the trade site, with copied items asked to the owner (English and French, advanced copy, with a screenshot) at the start of the step:
    - the **desecrated** modifiers: revealed, searched by their explicit id when they have one, like Exiled Exchange 2 (to check on the trade site), or by their desecrated id; unrevealed (`Desecrated Prefix`), a filter of the number of unrevealed modifiers (`misc_filters.veiled`, `pseudo.pseudo_number_of_unrevealed_mods`) in place of their line; those of the lords of the Abyss (Amanamu, Kurgal, Ulaman), outside the modifier tables;
    - the **fractured** modifiers, searched today by their fractured id; the **sanctified** items;
    - the modifiers of the **Breach** (`breach_desecration` among the tags of RePoE) and its rings, and those of the other endgame mechanics found on the way;
    - the **Time-Lost jewels**: their modifier tables, left out of the generator in 13j1 (159 tables), since the game writes their lines within `Small Passive Skills in Radius also grant …` or `Notable Passive Skills in Radius also grant …` while RePoE gives the bare line; a Time-Lost jewel copied by the owner tells which;
    - their style in the stats (a tag, a color): mock-up first.
- [ ] **14. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
- [ ] **15. Update notice**: at launch, ask the GitHub releases for the latest version, and when it is newer than the running one, a bubble "Version 0.2.0 available" with a link to its download page, closed for good per version (like the bubble of a new league); mock-up first. Needed before the first release: a player kept on an old version broken by a change of the game (Path of Exile 2 1.0, a new copied text format) or of the trade site wouldn't know it.
- [ ] **16. First release**: executable name (without the `rust-` prefix) and application id from one constant, application icon from the emblem, a readable message when the window can't open (no OpenGL, no X11 / XWayland), README usage (with where the log is), Wayland help, clipboard help and services called, version 0.1.0.
- [ ] **17. Overlay POC**: after the first release, a proof of concept of the price shown over the game, the strongest point of Exiled Exchange 2, to compete with it there too.
- [ ] **18. More settings**: after the first release, the settings the players may want, gathered while building the application; first, turning the pseudo stats on or off among the filters of the rare items (their default to decide then); the roll bar of the stats drawn as the zone searched, for the players coming from Exiled Exchange 2 (the zone between the minimum and the maximum colored, a tick at the roll, the lowest value on the left), instead of the position of the roll, the default (mock-up first: this variant was drawn and compared on 2026-10-27, in step 13e).
- [ ] **19. Automatic updates**: after the first release, if the application finds its players: the new version downloaded in the background and installed at the next launch, like Discord, with a signature checked before installing anything; Velopack (a real installer for Windows and macOS) or the `self_update` crate (the executable replaced), and Flathub for Linux, whose system updates it; to decide with the installers of the first release.
- [ ] **20. Polish**: the last touches, at the very end: a tooltip on the DPS of a weapon telling it counts its quality, modifiers and runes ("With its quality, modifiers and runes"), so that it isn't taken for the DPS of the base; after a search that found no offer priced in Exalted, Chaos or Divine Orbs (none at all, or only offers in other currencies), a price block that no longer says "Search similar offers to estimate it." (mock-up first); the tests of the large modules moved to a file of their own, as for the parser (`src/item/parser/tests.rs`), starting with `src/roll_difference.rs` (some 800 lines, most of them tests), so that reading the code costs fewer tokens; one `#[cfg(test)]` builder of `ItemOffer` in `src/trade/offers.rs` for the tests that each write their own (four of them: `app.rs`, `estimate.rs`, `offers.rs`, `ui/offers_block.rs`).
- [ ] **21. Price fallback**: a plan B, after everything else, for when poe2scout can't be reached: the price of a currency or a unique from the trade site (its currency exchange, or the median of its cheapest offers), on a click only, with a search block of its own in the price block; the cached prices stay shown with their age until then. Within the [Trade site usage](docs/trade-site-usage.md) guardrails: never when an item is copied, never in the background, never a local copy of poe2scout (poe2scout reads the currency exchange API of Grinding Gear Games and the trade site from one server; every player doing it would be far more requests). Mock-up first.
