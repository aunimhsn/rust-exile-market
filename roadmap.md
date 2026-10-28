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
- [ ] **13. Estimated prices**, in sixteen pull requests, from the [game data](docs/game-data.md):
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
  - [x] **13j, the modifier tables**, in five pull requests:
    - [x] **13j1, the tables** (#53)
    - [x] **13j2, the roll bar and the range field over every tier** (#55)
    - [x] **13j3, the item class as a type** (#58)
    - [x] **13j4, the advanced copy only**, decided with the owner on 2026-10-28, like Exiled Exchange 2 (which presses the advanced copy itself): an item with modifiers copied without the advanced description shows a message in its place, asking to copy it with Ctrl+Alt+C, and nothing else (no price, no search). Its impact, audited on 2026-10-28, in two pull requests:
      - [x] **13j4a, the message** (#60)
      - [x] **13j4b, the plain copy taken out of the code**: the parser returns `ParseError::WithoutDetails` itself, with the detection of 13j4a (an implicit line without header, or a plain section after the item level, with a number, on an identified magic, rare or unique item without any header); gone: the plain explicit modifiers (`Section::Explicit`, `Format`, `Item::copied_without_details`), a line matched as a prefix or a suffix (`modifier_tiers` gives none for `Explicit(None)`), the ranges of a unique from the price database (`src/rolls.rs`, `RangeSource`, the modifiers of the uniques read from poe2scout, the price cache in version 5). The **Explicit** group stays, decided with the owner, for a modifier whose header doesn't tell its affix (a header the parser doesn't know), without tier. The fixtures written as plain copies with modifiers (`ui/item_stats.rs`, `trade/stat_filters.rs`, `tier_rolls.rs`) written again as advanced copies; `en-plain-rare-ring.txt` and `en-plain-unique-jewel.txt` kept as the copies the message answers.
  - [ ] **13k, the modifiers of the endgame mechanics**: every edge case of a modifier tied to a mechanic, studied first in Exiled Exchange 2, RePoE and the trade site, with copied items asked to the owner (English and French, advanced copy, with a screenshot) at the start of the step:
    - the **desecrated** modifiers: revealed, searched by their explicit id when they have one, like Exiled Exchange 2 (to check on the trade site), or by their desecrated id; unrevealed (`Desecrated Prefix`), a filter of the number of unrevealed modifiers (`misc_filters.veiled`, `pseudo.pseudo_number_of_unrevealed_mods`) in place of their line; those of the lords of the Abyss (Amanamu, Kurgal, Ulaman), outside the modifier tables;
    - the **fractured** modifiers, searched today by their fractured id; the **sanctified** items;
    - the **quality of each kind of item**: 20 % at most, more on an exceptional base (30 % said the owner), the quality of a catalyst on a ring or an amulet (`Quality (Caster Modifiers): +49%`, up to 50 % seen) and on the Breach rings (75 %, the highest of any item, seen on the trade site on 2026-10-28); the parser doesn't read the quality of a catalyst yet (it reads `Quality: ` and `Qualité: ` only, in English and French: such an item has no filter of its quality), the field of the quality stays within 0–75 % until then, and the `quality` filter of the trade site doesn't tell the kind of quality (an amulet of 49 % of caster quality finds those of attack quality too);
    - the modifiers of the **Breach** (`breach_desecration` among the tags of RePoE) and its rings, and those of the other endgame mechanics found on the way;
    - the **Time-Lost jewels**: their modifier tables, left out of the generator in 13j1 (159 tables), since the game writes their lines within `Small Passive Skills in Radius also grant …` or `Notable Passive Skills in Radius also grant …` while RePoE gives the bare line; a Time-Lost jewel copied by the owner tells which;
    - the **domain of a modifier table**, not kept in 13j1: the 5 tables of the flask charges (`["default", 1]`) roll on any base for the application, harmless while no other item writes their lines (checked on 2026-10-28); keep the domain (or the classes of the bases) in the tables;
    - the **copy without details** checked on real copies (code review of 13j4a, 2026-10-28): a normal ring with its implicit, a charm, a waystone, a skill gem and a normal item with a rune, each copied with Ctrl+C and Ctrl+Alt+C, added to the test items, so that none of them shows the message by mistake;
    - and every other edge case found while studying them: this step is the polish of step 13;
    - their style in the stats (a tag, a color): mock-up first.
- [ ] **14. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
- [ ] **15. Update notice**: at launch, ask the GitHub releases for the latest version, and when it is newer than the running one, a bubble "Version 0.2.0 available" with a link to its download page, closed for good per version (like the bubble of a new league); mock-up first. Needed before the first release: a player kept on an old version broken by a change of the game (Path of Exile 2 1.0, a new copied text format) or of the trade site wouldn't know it.
- [ ] **16. First release**: executable name (without the `rust-` prefix) and application id from one constant, application icon from the emblem, a readable message when the window can't open (no OpenGL, no X11 / XWayland), README usage (copy with Ctrl+Alt+C; with where the log is), Wayland help, clipboard help and services called, version 0.1.0.
- [ ] **17. Overlay POC**: after the first release, a proof of concept of the price shown over the game, the strongest point of Exiled Exchange 2, to compete with it there too; its hotkey can press the advanced copy itself, like Exiled Exchange 2, so that the player no longer has to. To solve there: the hotkey and the keys sent to the game under Wayland (the game, an X11 client through XWayland, may let an X11 application hear and send keys while it has the focus, as Exiled Exchange 2 does on Linux; otherwise the global shortcuts portal of the desktop).
- [ ] **18. More settings**: after the first release, the settings the players may want, gathered while building the application; first, turning the pseudo stats on or off among the filters of the rare items (their default to decide then); the roll bar of the stats drawn as the zone searched, for the players coming from Exiled Exchange 2 (the zone between the minimum and the maximum colored, a tick at the roll, the lowest value on the left), instead of the position of the roll, the default (mock-up first: this variant was drawn and compared on 2026-10-27, in step 13e); and for the rare and magic modifiers, two other bars over every tier, drawn and compared on 2026-10-28 in step 13j2 and found interesting by the owner: one segment per tier, the worst on the left, those below its tier full and its own filled to the roll within it; or filled to the roll over the zone of its tier, faint.
- [ ] **19. Automatic updates**: after the first release, if the application finds its players: the new version downloaded in the background and installed at the next launch, like Discord, with a signature checked before installing anything; Velopack (a real installer for Windows and macOS) or the `self_update` crate (the executable replaced), and Flathub for Linux, whose system updates it; to decide with the installers of the first release.
- [ ] **20. Polish**: the last touches, at the very end: a tooltip on the DPS of a weapon telling it counts its quality, modifiers and runes ("With its quality, modifiers and runes"), so that it isn't taken for the DPS of the base; after a search that found no offer priced in Exalted, Chaos or Divine Orbs (none at all, or only offers in other currencies), a price block that no longer says "Search similar offers to estimate it." (mock-up first); the tests of the large modules moved to a file of their own, as for the parser (`src/item/parser/tests.rs`), starting with `src/roll_difference.rs` (some 800 lines, most of them tests), so that reading the code costs fewer tokens; one `#[cfg(test)]` builder of `ItemOffer` in `src/trade/offers.rs` for the tests that each write their own (four of them: `app.rs`, `estimate.rs`, `offers.rs`, `ui/offers_block.rs`); the base type and the category of a search in one type, so that a query can't hold both (code review of 13j3, 2026-10-28): `ItemQuery` (`src/trade/query.rs`) has two independent fields since 13j3, `base_type: Option<String>` and `category: Option<&'static str>`, and only the two assignments of `link::query` (`src/trade/link.rs`) keep them exclusive (a base type, or a category of the trade site when the player unchecks the base, or neither for a base the game data doesn't know when another filter narrows the search); replace them with one `enum ItemType { Base(String), Category(&'static str), Any }` (one base type, `Gemini Bow`; every base of a category, `weapon.bow`; any base), matched exhaustively by `ItemQuery::to_json` (`type` or `type_filters.category`), so that the compiler checks the rule; it touches about ten test fixtures that write an `ItemQuery` (`query.rs`, `link.rs`, `client.rs`, `offers.rs`, `app.rs`); the item class searched in faint under the base type unchecked ("Class searched: Masses à deux mains", `stat_text` in `src/ui/item_stats.rs`), cut with "…" past its column: if a class of the game doesn't fit in the narrow layout, its whole text in a tooltip, like the other lines cut in the application (`galley.elided` and `on_hover_text`, as in `src/ui/price_block.rs`), mock-up first (second code review of 13j3, 2026-10-28: the longest class checked, "Masses à deux mains", fits).
- [ ] **21. Price fallback**: a plan B, after everything else, for when poe2scout can't be reached: the price of a currency or a unique from the trade site (its currency exchange, or the median of its cheapest offers), on a click only, with a search block of its own in the price block; the cached prices stay shown with their age until then. Within the [Trade site usage](docs/trade-site-usage.md) guardrails: never when an item is copied, never in the background, never a local copy of poe2scout (poe2scout reads the currency exchange API of Grinding Gear Games and the trade site from one server; every player doing it would be far more requests). Mock-up first.
