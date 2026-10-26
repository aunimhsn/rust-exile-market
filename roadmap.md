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
- [ ] **13. Estimated prices**, in eight pull requests, from the [game data](docs/game-data.md):
  - [x] **13a, the game data** (#PR): the sources of the stat ids and of the tiers compared on the copied items, and chosen.
  - [ ] **13b, the stat ids**: the generator of the embedded game data (stats of Exiled Exchange 2 in English and French, modifiers and bases of RePoE, their notices), and the template of each modifier line (`Adds # to # Lightning Damage`, sign, `reduced` variants, local stats, lines of several stats) matched with its trade id in the group of the line (explicit, implicit, fractured, desecrated, rune, enchant); the trade site templates (`/data/stats`, with the names) as a second lookup; a line found nowhere written to the log; the item class as a type rather than a translated name; tested on every line of the copied items. Whether the `better` side of the stats replaces the words read by the offer tooltip.
  - [ ] **13c, the uniques searched with their filters**: the checked filters and their minimums in the search and in the link of Check on trade (until then, a unique is searched by its name, base and corrupted state); a line of several numbers searched at their average; what a line without trade id shows (to decide with the owner).
  - [ ] **13d, the filters of the rare and magic items and jewels**: every modifier at 90 % of the copied value (since 10b), the base type checked, the item level unchecked (the **Item** group); for a weapon, the **Weapon** group: the physical DPS checked, the elemental DPS and the damage and attack speed modifiers unchecked; for a jewel, all its modifiers.
  - [ ] **13e, the pseudo totals**: the **Pseudo** group (total elemental resistance, maximum life, attributes…), its totals added up from the modifiers, checked, and the modifiers they add up unchecked; a table of rules with its tests.
  - [ ] **13f, the normal items**: the base type, the sockets and the item level checked, the quality unchecked; how to search an exceptional base exactly (its name or its number of sockets), checked on the trade site.
  - [ ] **13g, the estimated price**: the text before the search, then **Estimated price**: the median of the 5 cheapest similar offers, suspicious offers excluded, "18 similar", "Median of the 5 cheapest similar offers.", the uncertainty warning under 5; how an offer is suspicious when the reference price is the estimate itself (to decide with the owner).
  - [ ] **13h, the modifier tables**: the roll bar of the rare and magic modifiers over every tier of the base, the minimum kept within the range of every tier, their tier and range read from the tables when the item was copied without the advanced copy (to decide).
- [ ] **14. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
- [ ] **15. Update notice**: at launch, ask the GitHub releases for the latest version, and when it is newer than the running one, a bubble "Version 0.2.0 available" with a link to its download page, closed for good per version (like the bubble of a new league); mock-up first. Needed before the first release: a player kept on an old version broken by a change of the game (Path of Exile 2 1.0, a new copied text format) or of the trade site wouldn't know it.
- [ ] **16. First release**: executable name (without the `rust-` prefix) and application id from one constant, application icon from the emblem, a readable message when the window can't open (no OpenGL, no X11 / XWayland), README usage (with where the log is), Wayland help, clipboard help and services called, version 0.1.0.
- [ ] **17. Overlay POC**: after the first release, a proof of concept of the price shown over the game, the strongest point of Exiled Exchange 2, to compete with it there too.
- [ ] **18. More settings**: after the first release, the settings the players may want, gathered while building the application; first, turning the pseudo stats on or off among the filters of the rare items (their default to decide then).
- [ ] **19. Automatic updates**: after the first release, if the application finds its players: the new version downloaded in the background and installed at the next launch, like Discord, with a signature checked before installing anything; Velopack (a real installer for Windows and macOS) or the `self_update` crate (the executable replaced), and Flathub for Linux, whose system updates it; to decide with the installers of the first release.
- [ ] **20. Polish**: the last touches, at the very end: a tooltip on the DPS of a weapon telling it counts its quality, modifiers and runes ("With its quality, modifiers and runes"), so that it isn't taken for the DPS of the base.
