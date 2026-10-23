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
| Offers | Official trade site API: `/api/trade2/search` then `/api/trade2/fetch` for unique items, `/api/trade2/exchange` for the currency exchange (to verify in step 10) | The only source of individual offers: poe2scout only gives aggregates. Accepted with guardrails despite the Terms of Use risk: see [Trade site usage](#trade-site-usage). The same search engine will price the rare items. |
| Item names in French | Trade site data in both languages (`/api/trade2/data/items`, `/data/static` and `/data/stats` on `www.` and `fr.pathofexile.com`), downloaded at startup and cached for a day | The price database only knows the English names. The trade site gives the names exactly as in each game client; `/data/static` and `/data/stats` share language-independent ids between the two languages. The item lists are not in the same order in every category, so they are matched by id, or by name for the uniques. |
| Images | Orb icons bundled from `assets/`, item icons loaded at runtime from the Grinding Gear Games CDN and cached | See [Images](#images). |
| HTTP | [`ureq`](https://github.com/algesten/ureq) in a background thread, `serde` / `serde_json` | Blocking and simple; the requests are few (one price refresh per hour, one trade site search per copied item), so an async runtime is not needed. The interface thread is never blocked. |
| Interface translations | [Fluent](https://projectfluent.org) with `fluent-bundle`, files `locales/en.ftl` and `locales/fr.ftl` embedded in the executable | Handles the plural rules, which differ in French ("0 offre") and in English ("0 offers"), and switching the language at runtime, needed by the automatic language. |
| Local files | [`directories`](https://github.com/dirs-dev/directories-rs) for the configuration and cache directories, settings in TOML | Standard locations on each system. |
| Price history chart | [`egui_plot`](https://github.com/emilk/egui_plot) | Made for egui, no other dependency. |
| Releases | GitHub Actions workflow triggered by version tags, portable archives (no installer) | Builds every system on its own runner and publishes the archives with `gh release`, without extra tooling. A portable executable is enough for version 1. The Linux target moves from musl to glibc in step 1: a statically linked musl executable can't load the system's graphics libraries. |

## Images

The Grinding Gear Games [developer guidelines](https://www.pathofexile.com/developer/docs) forbid using their intellectual property to make money, imitating their official platforms with their logos, and presenting a tool as made or approved by them; every public tool must show the sentence "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." Showing the game icons in a free tool fits these rules, like poe2db, poe.ninja and poe2scout do. The application follows these rules:

- `assets/` holds only the icons that are always visible: the Exalted, Divine and Chaos Orbs of the price table. They are embedded in the executable, so that they show offline. [assets/NOTICE.md](assets/NOTICE.md) states that they belong to Grinding Gear Games and are not covered by the project license.
- The icons of the copied items are never added to the repository: the application loads them from the Grinding Gear Games CDN (`web.poecdn.com`), at the address given by the price database, and keeps them in the cache directory. Each icon is downloaded once.
- No icon is extracted from the game files.
- The application icon is an original creation, without the Path of Exile logo.
- The disclaimer is shown in the README and in the application settings.

## poe2scout usage

poe2scout limits its API to 60 requests per minute per IP address, in a sliding window, without a queue ([source](https://github.com/poe2scout/poe2scout/blob/main/net/Poe2scout.Api/Ioc/ServiceCollectionExtensions.cs)). The limit applies to each player's machine, not to the application as a whole. A full refresh is about 20 requests (20 categories, 250 items per page), once an hour; copying an item makes no request. The application still follows these rules:

- Space the requests of a refresh: one per second at most.
- When the API refuses a request, wait and retry at the next attempt, without insisting.
- Keep the loaded price histories in memory: copying an item again makes no new request.
- Send a `User-Agent` with the application name, its version and the repository address.
- Tell the poe2scout maintainers about the application, on their Discord (linked from [poe2scout.com](https://poe2scout.com)), when the releases reach 100 downloads in total (`gh release view` gives the count of each archive).

## Trade site usage

The trade site API is not in the [Grinding Gear Games developer documentation](https://www.pathofexile.com/developer/docs), and their Terms of Use (section 7i) forbid reverse-engineering undocumented endpoints; the sanction is the closing of the accounts of the developer and of the users. The whole community uses it (Awakened PoE Trade, Exiled Exchange 2) and Grinding Gear Games tolerates it, but it is not allowed. The owner accepted this risk on 2026-10-23, with the guardrails below.

The trade site announces its limits per IP address in the `X-Rate-Limit-*` headers of each response (`hits:period:penalty`, in seconds). Measured on 2026-10-23:

| Policy | Limits |
|---|---|
| Search (`trade-search-request-limit`) | 5 per 10 s (60 s penalty), 15 per 60 s (300 s), 30 per 300 s (1,800 s), 600 per 6 h (3,600 s) |
| Fetch (`trade-fetch-request-limit`), 10 offers per request | 12 per 4 s (10 s penalty), 16 per 12 s (300 s), 50 per 300 s (300 s), 1,000 per 6 h (1,800 s) |

Guardrails:

- A search only runs when the player clicks **Search the offers**: copying an item never searches by itself (sorting the loot copies many items in a row). One click makes one search and one fetch; **Show the next 10** makes one more fetch. Nothing runs in the background.
- After each search, the button stays disabled for at least 10 seconds, and longer until every window of the limits allows a new search. The limits are read from the headers of every response (the IP address may share them with a browser or another tool), never from constants; after a refusal (HTTP 429), the button waits for the `Retry-After` delay, and the application never retries by itself.
- Never send a `POESESSID` session cookie: the searches work without an account.
- Send a `User-Agent` with the application name, its version and the repository address.
- The README states which services the application calls, and this risk.

## Steps

- [ ] **0. Mock-ups**: narrow layout first (mobile first): editable Excalidraw wireframe in `docs/wireframes/`, then an interactive HTML mock-up in `docs/mockups/` of the narrow layout and of the settings panel, for each kind of item (currency stack, unique, rare not priced yet, errors), with the item stats and the trade site offers; then the style guide (colors, sizes, typography) in specs.md. Each block (item, price, offers, history) stays independent, so that the wider layouts only rearrange them.
- [ ] **1. Application window**: empty resizable egui window with the header (league placeholder, gear button), X11 backend on Linux, no console window on Windows, Linux release target moved to glibc.
- [ ] **2. Clipboard watching**: detect the copied Path of Exile 2 item texts and show them raw in the window; verify on Bazzite KDE with the game, and on Windows.
- [ ] **3. Item parser**: read the item class, rarity, name, base type, stack size and modifiers, in English and in French, with tests on real copied items.
- [ ] **4. Interface translations**: Fluent files in English and French, automatic language from the last copied item.
- [ ] **5. Settings**: gear panel (language, always on top), saved in the configuration directory, window size and position restored.
- [ ] **6. Price data**: poe2scout client, leagues and league setting, download per category, cache, hourly refresh, error handling, following the [poe2scout usage](#poe2scout-usage) rules.
- [ ] **7. French item names**: trade site data in both languages, cached for a day, and the French to English dictionary used to find the price of a French item.
- [ ] **8. Price display**: headline price, price in the three currencies, stack, offer count and uncertainty warning, special cases, trade site button, status bar, icons (orbs embedded from `assets/`, item icons loaded from the CDN and cached, following the [Images](#images) rules).
- [ ] **9. Item stats**: requirements, description, implicit and explicit modifiers, Corrupted, roll bars from the advanced copy.
- [ ] **10. Trade site client**: search, fetch and currency exchange, limits read from the headers, waiting time before the next search, following the [Trade site usage](#trade-site-usage) rules.
- [ ] **11. Offers display**: **Search the offers** button and its waiting time, offers of unique items with their rolls, bulk offers of currency items, suspicious offers, next 10, searching and long wait states.
- [ ] **12. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the style guide, then these layouts in the application, with the rolls of each offer and the 7-day history chart in the wide layout.
- [ ] **13. First release**: executable name (without the `rust-` prefix), application icon, README usage, Wayland help and services called, version 0.1.0.
