# Roadmap

Implementation steps of the features described in [specs.md](specs.md). Each step is delivered in its own pull request, which checks its box.

## Technical choices

| Topic | Choice | Reason |
|---|---|---|
| Graphical interface | [egui](https://github.com/emilk/egui) with `eframe` | Pure Rust, one executable per system without an embedded browser. The interface is redrawn every frame with the available size, so the layout can change with the window width in a few lines. Supports resizing, always on top, restoring the window size and position, and animating a value over time (`ctx.animate_bool`), for the hover effects. Tauri was rejected: HTML and JavaScript, and an embedded browser that is often fragile on Linux. The OpenGL renderer (`glow`) is used rather than `wgpu`: lighter, and enough for a flat 2D interface. In debug builds, the dependencies are optimized (`[profile.dev.package."*"]`), so that the images decode and the window stays smooth during development. |
| Visual style | The tokens of the [Visual style](specs.md#visual-style) in one Rust module, applied to the egui `Style` and `Visuals` | The mock-up and the application share the same values; changing a color or a size happens in one place. |
| Fonts | Manrope and JetBrains Mono, embedded in the executable from `assets/fonts/` | Both under the SIL Open Font License; the interface looks the same on every system, without depending on the installed fonts. |
| Icons | White SVG files tinted at runtime (`assets/icons/`, from [Lucide](https://lucide.dev), ISC license), the logo and the emblem from `assets/logo/`, rendered with `egui_extras` (SVG and PNG loaders); the simplest marks (chevron) are drawn with egui lines | SVG icons stay sharp at any display scale, and tinting one white file gives each state its color. |
| Linux display | X11 (XWayland under Wayland), chosen with `winit` when the event loop is built | The game runs through Proton as an X11 client, so an X11 application can read what it copies without having the focus; a native Wayland application can't always read the clipboard in the background. To be verified on Bazzite KDE in step 2. winit would pick Wayland otherwise (a dependency of eframe enables it), and the OpenGL renderer is only built for X11. |
| Clipboard | [`arboard`](https://github.com/1Password/arboard), polled every 250 ms | Same API on Windows, macOS and X11, for reading the items and writing the whispers. Polling is simple and enough to react before the player looks at the window. |
| Price database | [poe2scout](https://poe2scout.com) public API (`https://api.poe2scout.com`, [Swagger](https://api.poe2scout.com/swagger)) | Covers the currency exchange items and the uniques of Path of Exile 2, gives the number of offers, the 7-day history, the leagues and the Exalted / Divine / Chaos rates. poe.ninja was rejected: its documentation asks desktop applications to go through their own backend rather than call its API from the players' machines. The prices are downloaded per category and cached, behind a trait, so that the source can change. |
| Price estimation | One `trait` implementation per way of pricing, chosen by an `enum` of the item kinds: fixed-name items from the price database, the others estimated from similar offers | Each kind of item gets its price its own way, without changing the parser or the interface. |
| Offers | Official trade site API: `/api/trade2/search` then `/api/trade2/fetch` for items, `/api/trade2/exchange` for the currency exchange (to verify in step 10) | The only source of individual offers: poe2scout only gives aggregates. The fetch returns the whole listed item (modifiers, item level, seller status, whisper message), used by the tooltip without another request. Accepted with guardrails despite the Terms of Use risk: see [Trade site usage](#trade-site-usage). |
| Stat filters | The stats of the copied item become filters of the search, as in Exiled Exchange 2: stat ids from `/api/trade2/data/stats` (explicit, implicit and pseudo groups), matched with the modifier texts; equipment filters (`pdps`, `edps`, sockets, item level, quality) from `/api/trade2/data/filters`; ranges from poe2scout for uniques | Offers are only comparable with similar stats; the same filters price the uniques, the rare, magic and normal items and the jewels. How to search an exceptional base exactly (its name or its number of sockets) is checked in step 12. |
| Modifier tiers | The tier and its range from the advanced copy; the range of every tier of a modifier from the Path of Exile 2 modifier tables (source chosen in step 12, e.g. an export of the game data such as poe2db or a RePoE fork), embedded and updated with the leagues | The roll bar of a rare modifier covers all its tiers, so that a full bar means close to the best possible; the advanced copy only gives the range of the current tier. |
| Estimated price | Median of the 5 cheapest similar offers, suspicious offers excluded | A median resists the bait offers and the few overpriced ones; 5 offers are enough to be meaningful without extra requests (one fetch returns 10). |
| Item names in French | Trade site data in both languages (`/api/trade2/data/items`, `/data/static` and `/data/stats` on `www.` and `fr.pathofexile.com`), downloaded at startup and cached for a day | The price database only knows the English names. The trade site gives the names exactly as in each game client; `/data/static` and `/data/stats` share language-independent ids between the two languages. The item lists are not in the same order in every category, so they are matched by id, or by name for the uniques. |
| Images | Orb icons, logo and emblem bundled from `assets/` | See [Images](#images). |
| HTTP | [`ureq`](https://github.com/algesten/ureq) in a background thread, `serde` / `serde_json` | Blocking and simple; the requests are few (one price refresh per hour, one trade site search per click), so an async runtime is not needed. The interface thread is never blocked. |
| Interface translations | [Fluent](https://projectfluent.org) with `fluent-bundle`, files `locales/en.ftl` and `locales/fr.ftl` embedded in the executable | Handles the plural rules ("1 stat", "6 stats"; "0 offre", "0 offers"), and switching the language at runtime, needed by the automatic language. |
| Local files | [`directories`](https://github.com/dirs-dev/directories-rs) for the configuration and cache directories, settings in TOML | Standard locations on each system. |
| Price history chart | [`egui_plot`](https://github.com/emilk/egui_plot) | Made for egui, no other dependency (wide layout, step 13). |
| Releases | GitHub Actions workflow triggered by version tags, portable archives (no installer) | Builds every system on its own runner and publishes the archives with `gh release`, without extra tooling. A portable executable is enough for version 1. The Linux executable targets glibc rather than musl: a statically linked musl executable can't load the system's graphics libraries (OpenGL, X11). |

## Path of Exile 2 1.0

Path of Exile 2 leaves early access with its version 1.0 on 2026-12-11. The modifiers, their tiers and ranges, the trade site data and maybe the format of the copied text can change. After this release:

- copy real items again (English and French) and rerun the parser tests;
- check the stat ids, the pseudo stats and the filters of the trade site, and the categories of poe2scout;
- check the tiers and ranges given by the advanced copy, and update the modifier tables.

## Images

The Grinding Gear Games [developer guidelines](https://www.pathofexile.com/developer/docs) forbid using their intellectual property to make money, imitating their official platforms with their logos, and presenting a tool as made or approved by them; every public tool must show the sentence "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." Showing the game icons in a free tool fits these rules, like poe2db, poe.ninja and poe2scout do. The application follows these rules:

- `assets/currencies/` holds the Exalted, Divine and Chaos Orbs, the only game icons shown. They are embedded in the executable, so that they show offline. [assets/NOTICE.md](assets/NOTICE.md) states that they belong to Grinding Gear Games and are not covered by the project license.
- The window shows no item icon: no other game image is downloaded or added to the repository, and none is extracted from the game files.
- `assets/logo/` holds the logo and the emblem of the application, original creations of the project, without the Path of Exile logo.
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

- A search only runs when the player clicks the search or refresh block: copying an item never searches by itself (sorting the loot copies many items in a row). One click makes one search and one fetch; the double chevron makes one more fetch. Nothing runs in the background.
- After each search, the waiting block lasts at least 10 seconds, and longer until every window of the limits allows a new search. The limits are read from the headers of every response (the IP address may share them with a browser or another tool), never from constants; after a refusal (HTTP 429), the block waits for the `Retry-After` delay, and the application never retries by itself.
- Never send a `POESESSID` session cookie: the searches work without an account. So no direct whisper and no travel to a hideout; **Copy whisper** only copies the whisper text of the listing to the clipboard.
- Send a `User-Agent` with the application name, its version and the repository address.
- The README states which services the application calls, and this risk.

## Steps

- [x] **0. Mock-ups**: narrow layout first (mobile first): editable Excalidraw wireframe in `docs/wireframes/`, then the interactive HTML mock-up `docs/mockups/narrow-styles.html` (three styles compared with tabs, style B · Slate chosen and iterated) of every kind of item, state and error, with the item stats, the filters, the offers and their tooltip; the visual style, the logo and the emblem written in specs.md and `assets/logo/`.
- [x] **1. Application window**: egui window with the visual style (tokens, embedded fonts), the header (emblem and name, league block, gear block), the status bar and the empty state (logo, Ctrl + C), texts translated with Fluent in English and French, X11 backend on Linux, no console window on Windows, Linux release target moved to glibc.
- [ ] **2. Clipboard watching**: detect the copied Path of Exile 2 item texts and show them raw in the window; the clipboard is read in `App::logic` with a repaint requested every 250 ms (eframe only redraws on events, and the game has the focus), `App::ui` only draws; verify on Bazzite KDE with the game, and on Windows.
- [ ] **3. Item parser**: read the item class, rarity, name, base type (without the "Exceptional" prefix), stack size, item level, quality, sockets, requirements, granted skill, states and modifiers, in English and in French, with tests on real copied items (including an exceptional base in both languages).
- [ ] **4. Interface translations**: automatic language from the last copied item, switched at runtime (the Fluent files exist since step 1).
- [ ] **5. Settings**: settings panel (language, always on top, disclaimer) opened by the gear, the header returning the clicked block as an action to the application, saved in the configuration directory, window size and position restored.
- [ ] **6. Price data**: poe2scout client, leagues and the league block of the header (softcore and hardcore colors, list of the leagues on click), download per category, cache, hourly refresh, error handling, following the [poe2scout usage](#poe2scout-usage) rules.
- [ ] **7. French item names**: trade site data in both languages, cached for a day, and the French to English dictionary used to find the price of a French item.
- [ ] **8. Price display**: item block (name, Check on trade, tags, summary, requirements) and price block of the fixed-name items (three currencies with their orbs, stack value and unit price, offer count or stack size, trend, uncertainty warning, special cases), status bar.
- [ ] **9. Item stats and filters**: Show stats pill, modifiers with their roll bars, filters of the uniques (checkboxes, minimums, corrupted state).
- [ ] **10. Trade site client**: search, fetch and currency exchange, limits read from the headers, waiting time before the next search, following the [Trade site usage](#trade-site-usage) rules.
- [ ] **11. Offers display**: search block, waiting and refresh block, offer rows with the seller status, suspicious offers, currency exchange offers, more offers, searching, long wait and no offer states; the offer tooltip (position, pin, roll differences, Copy whisper, with a note when the whisper is in the seller's language, which the trade site uses) and the hover animations.
- [ ] **12. Estimated prices**: filters of the rare and magic items and jewels (pseudo totals, weapon DPS, base, item level, minimums at 90 %) and of the normal items (base, sockets, item level, exceptional bases), estimated price from the median of the 5 cheapest similar offers, uncertainty warning.
- [ ] **13. Responsive layout and price history**: wireframes then mock-up of the medium and wide layouts with the visual style, then these layouts in the application, with the 7-day history chart in the wide layout.
- [ ] **14. First release**: executable name (without the `rust-` prefix) and application id from one constant, application icon from the emblem, a readable message when the window can't open (no OpenGL, no X11 / XWayland), README usage, Wayland help, clipboard help and services called, version 0.1.0.
