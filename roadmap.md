# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Reference

Read only by the steps that need them:

- [Technical choices](docs/technical-choices.md): the libraries, the services and the main designs, with their reasons.
- [Images](docs/images.md): the game images the application may show, and the rules of Grinding Gear Games.
- [poe2scout usage](docs/poe2scout-usage.md): the limits of the price database and the rules the application follows.
- [Trade site usage](docs/trade-site-usage.md): the risk accepted by the owner, the limits and the guardrails.
- [Roadmap archive](docs/roadmap-archive.md): the detail of the steps done.

## Path of Exile 2 1.0

Path of Exile 2 leaves early access with its version 1.0 on 2026-12-11. The modifiers, their tiers and ranges, the trade site data and maybe the format of the copied text can change. After this release:

- copy real items again (English and French) and rerun the parser tests;
- check the stat ids, the pseudo stats and the filters of the trade site, and the categories of poe2scout;
- check the tiers and ranges given by the advanced copy, and update the modifier tables.

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
- [ ] **10. Item stats and filters**, in three pull requests:
  - **10a, the stats** (done, #20)
  - **Interlude, token economy** (done, #23)
  - **10b, the filters of the uniques**: a checkbox in the column kept for it, and the minimum in the column on the right (the "min" title on the right of the first group); the minimum field takes digits, one point and a leading minus sign only (no letter, not even the `e` of an exponent, also when a text is pasted), and stays within the range of the stat; the runes and the enchantment unchecked by default; a fixed line of a rare keeps its checkbox without a minimum; the corrupted state; the number of active filters beside the pill, and **Show filters** for an item without modifiers. Also: strip the game's "fixed value" marker from the stats (a copied sample from the owner first), and check the English label of the block of a shield (`Block chance`, a guess) on a copied shield.
  - **10c, the small amounts of the price block**: hide the Divine Orb (or Chaos Orb) amount when it is too small to matter (asked by the owner on 2026-10-26: under 0.5 for them, 0.1 suggested); decide on a mock-up the threshold and what the freed third of the row becomes.
- [ ] **11. Trade site client**: search, fetch and currency exchange, limits read from the headers, waiting time before the next search, following the [Trade site usage](docs/trade-site-usage.md) rules.
- [ ] **12. Offers display**: search block, waiting and refresh block, offer rows with the seller status, suspicious offers, currency exchange offers, more offers, searching, long wait and no offer states; the offer tooltip (position, pin, roll differences, Copy whisper, with a note when the whisper is in the seller's language, which the trade site uses) and the hover animations; the log filter that hides all the messages of `arboard` let through its warnings and errors, which explain a whisper that couldn't be copied.
- [ ] **13. Estimated prices**: the roll bar of the rare and magic modifiers over every tier; filters of the rare and magic items and jewels (pseudo totals, weapon DPS, base, item level, minimums at 90 %) and of the normal items (base, sockets, item level, exceptional bases), estimated price from the median of the 5 cheapest similar offers, uncertainty warning; the template of each modifier line (`Adds # to # Lightning Damage`) to match it with the stat ids of the trade site, and the item class as a type rather than a translated name.
- [ ] **14. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
- [ ] **15. First release**: executable name (without the `rust-` prefix) and application id from one constant, application icon from the emblem, a readable message when the window can't open (no OpenGL, no X11 / XWayland), README usage (with where the log is), Wayland help, clipboard help and services called, version 0.1.0.
- [ ] **16. Overlay POC**: after the first release, a proof of concept of the price shown over the game, the strongest point of Exiled Exchange 2, to compete with it there too.
- [ ] **17. More settings**: after the first release, the settings the players may want, gathered while building the application; first, turning the pseudo stats on or off among the filters of the rare items (their default to decide then).
- [ ] **18. Polish**: the last touches, at the very end: a tooltip on the DPS of a weapon telling it counts its quality, modifiers and runes ("With its quality, modifiers and runes"), so that it isn't taken for the DPS of the base.
