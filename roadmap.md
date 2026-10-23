# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Technical choices

| Topic | Choice | Reason |
|---|---|---|
| Graphical interface | [egui](https://github.com/emilk/egui) with `eframe` | Pure Rust, one executable per system without an embedded browser. The interface is redrawn every frame with the available size, so the layout can change with the window width in a few lines. Supports resizing, maximizing, always on top, and restoring the window size and position. Tauri was rejected: HTML and JavaScript, and an embedded browser that is often fragile on Linux. |
| Linux display | X11 (XWayland under Wayland) | The game runs through Proton as an X11 client, so an X11 application can read what it copies without having the focus; a native Wayland application can't always read the clipboard in the background. To be verified on Bazzite KDE in step 2. |
| Clipboard | [`arboard`](https://github.com/1Password/arboard), polled every 250 ms | Same API on Windows, macOS and X11. Polling is simple and enough to react before the player looks at the window. |
| Price database | [poe2scout](https://poe2scout.com) public API (`https://api.poe2scout.com`, [Swagger](https://api.poe2scout.com/swagger)) | Covers the currency exchange items and the uniques of Path of Exile 2, gives the number of offers, the 7-day history, the leagues and the Exalted / Divine / Chaos rates. poe.ninja was rejected: its documentation asks desktop applications to go through their own backend rather than call its API from the players' machines. The prices are downloaded per category and cached, behind a trait, so that the source can change. |
| Price estimation | One `trait` implementation per kind of item, chosen by an `enum` of the item kinds | Version 1 implements the fixed-name items; rare items will be a new implementation (search on the trade site with the modifiers), without changing the parser or the interface. |
| Item names in French | Dictionary built from the official trade site data in both languages (`/api/trade2/data/items` and `/data/static` on `www.` and `fr.pathofexile.com`) | The price database only knows the English names. The lists are not in the same order in every category, so the matching rule is defined in step 7. |
| HTTP | [`ureq`](https://github.com/algesten/ureq) in a background thread, `serde` / `serde_json` | Blocking and simple; the requests are rare (once an hour), so an async runtime is not needed. The interface thread is never blocked. |
| Interface translations | [Fluent](https://projectfluent.org) with `fluent-bundle`, files `locales/en.ftl` and `locales/fr.ftl` embedded in the executable | Handles the plural rules, which differ in French ("0 offre") and in English ("0 offers"), and switching the language at runtime, needed by the automatic language. |
| Local files | [`directories`](https://github.com/dirs-dev/directories-rs) for the configuration and cache directories, settings in TOML | Standard locations on each system. |
| Price history chart | [`egui_plot`](https://github.com/emilk/egui_plot) | Made for egui, no other dependency. |
| Releases | GitHub Actions workflow triggered by version tags, portable archives (no installer) | Builds every system on its own runner and publishes the archives with `gh release`, without extra tooling. A portable executable is enough for version 1. The Linux target moves from musl to glibc in step 1: a statically linked musl executable can't load the system's graphics libraries. |

## Steps

- [ ] **1. Application window**: empty resizable egui window with the header (league placeholder, gear button), X11 backend on Linux, no console window on Windows, Linux release target moved to glibc.
- [ ] **2. Clipboard watching**: detect the copied Path of Exile 2 item texts and show them raw in the window; verify on Bazzite KDE with the game, and on Windows.
- [ ] **3. Item parser**: read the item class, rarity, name, base type, stack size and modifiers, in English and in French, with tests on real copied items.
- [ ] **4. Interface translations**: Fluent files in English and French, automatic language from the last copied item.
- [ ] **5. Settings**: gear panel (language, always on top), saved in the configuration directory, window size and position restored.
- [ ] **6. Price data**: poe2scout client, leagues and league setting, download per category, cache, hourly refresh, error handling.
- [ ] **7. French item names**: French to English dictionary from the trade site data, used to find the price of a French item.
- [ ] **8. Price display**: price in the three currencies, stack, offers and uncertainty warning, data age, special cases, trade site link.
- [ ] **9. Responsive layout and price history**: narrow, medium and wide layouts, 7-day history chart in the wide layout.
- [ ] **10. First release**: executable name (without the `rust-` prefix), README usage and Wayland help, version 0.1.0.
