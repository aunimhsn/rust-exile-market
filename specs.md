# Specifications

## Purpose

rust-exile-market is a small desktop application that gives the price of a Path of Exile 2 item, with its stats and the offers of the trade site, as soon as the player copies it in the game with Ctrl+C.

- Users: Path of Exile 2 players on Windows, macOS and Linux, including Wayland desktops such as Bazzite (KDE Plasma), where the game runs through Proton.
- Game client: English or French. The application understands the item text in both languages.
- Volume: one item at a time, compared against the prices of a few thousand items of the selected league, and one search on the trade site when the player asks for the offers.
- Version 1 prices every kind of item:
  - **fixed-name items** (currency and the other items of the currency exchange: fragments, essences, runes, soul cores, omens, uncut gems…, and unique items) from a public price database;
  - **rare, magic and normal items and jewels**, which have no market price, from the offers of similar items on the trade site.

The reference of the interface is the mock-up [docs/mockups/narrow-styles.html](docs/mockups/narrow-styles.html) (style B · Slate). The wireframe [docs/wireframes/window-layouts.excalidraw](docs/wireframes/window-layouts.excalidraw) only fixed the first layout; where they differ, the mock-up is right.

## Design principles

- One small window, a companion to the game: beside the game on the same screen, or on a second screen.
- Minimalist: nothing that doesn't help to decide (no redundant text, no counter that says nothing, no decoration).
- Dark interface made of full-width bands, without frames or cards.
- White is the color of the main action; the other colors are kept for a meaning: the item rarities, the league (softcore or hardcore), the warnings, better or worse.
- Short, factual messages. No pop-ups for errors: they are shown inside the window.
- The interface is available in English and in French; English is the reference language. The French messages use the formal "vous", like the French game client.
- Version 1 has the narrow layout only (see [Window layout](#window-layout)).

## Visual style

### Colors

| Token | Value | Use |
|---|---|---|
| Band A | `#0f1216` | Title bar, price block, status bar. |
| Band B | `#151a20` | Item block, offers block, settings. |
| Alternate row | `#1a2028` | Every other row of the offers. |
| Control | `#1d242d` | Pills, inputs, gear block, waiting countdown, tooltip (`#20272f`). |
| Control line | `#2a3440` | Input borders, separators, hover of the pills. |
| Text | `#e3e8ef` | Main text. |
| Weak text | `#8692a3` | Labels, secondary lines. |
| Faint text | `#556070` | Status bar, search age, roll ranges. |
| Main action | `#f2f5f9` (white), text on it `#0f1216` | Search and refresh blocks, checked boxes, opened gear, "Copied". |
| Softcore league | background `#13263a` (`#1a3350` on hover), line `#6aa7de` | League block. |
| Hardcore league | background `#351417` (`#4a1b20` on hover), line `#e0706a` | League block, background of the Corrupted tag. |
| Warning band | background `#8a6a10`, text `#f8edc2` | Errors under the title bar. |
| Warning text | `#f0b85a` | Uncertain price, suspicious offer, away seller. |
| Rise / fall | `#5fcf80` / `#e66a5c` | Trend, roll differences, seller online / offline. |
| Rarities | normal `#d0d0d0`, magic `#8888ff`, rare `#ffff77`, unique `#c26a2c`, currency `#b4a787` | Item names, as in the game. |
| Modifiers | `#8f8fff` | Implicit and explicit modifiers, as in the game. |
| Tier 1 | `#ff8c1a` | Tier of a modifier at its best tier: a vivid orange that stands out. |

### Typography

- **Manrope** for all the text: item name 20 px weight 800, body 13.5 px, labels 11 px uppercase with letter spacing, small texts 11.5 to 12 px.
- **JetBrains Mono** for the numbers that line up (prices, offers), weight 600.
- Both fonts are under the SIL Open Font License and embedded in the executable.
- Numbers: a point between the integer and the decimal parts in every language, no thousands separator (`12.4`, `8990`). Exalted Orbs without decimals, Divine and Chaos Orbs with up to 2.

### Shapes

- **Action blocks are flat and square**, the height of their band, glued to its edge: the league block in the title bar, the search and refresh blocks in the offers band. The icons of the title bar have no block: only the icon.
- **Small information elements are rounded by 3 px**: tags, inputs, checkboxes (15 px, drawn check mark), pills.
- **Secondary actions are pills** with a verb: "Check on trade", "Show 6 stats", "Copy whisper".
- The tooltip of an offer is the only floating element: radius 4 px, shadow.

### Icons and logo

- Icons are drawn as lines (stroke about 1.8 px), in the color of the text: search, refresh, double chevron, check mark, minimize, close; the gear is the "settings" icon of [Lucide](https://lucide.dev) (ISC license).
- Icons are aligned on the lowercase letters next to them, by eye on the real screen (e.g. text 0.5 px up next to an icon), not on the box of the text.
- The title bar shows the **emblem** (`assets/logo/emblem.svg`, white, 16 px) without the name; the empty window shows the full **logo** (`assets/logo/exile-market.png`).
- The Exalted, Divine and Chaos Orbs come from `assets/currencies/`.

### Hover and motion

- Pills and the league block: background one shade lighter on hover.
- Icons of the title bar: in the weak text color, white on hover; the gear stays white while the settings are open.
- Search block: the icon grows by 15 %; refresh block: the icon grows by 12 % and turns a quarter; show more: the double chevron slides down 2 px. About 0.2 s, a slight overshoot.
- No motion when the system asks for reduced motion.
- No specific design for the keyboard focus: the default of the toolkit is kept.

## Features

### Item detection

- The application watches the clipboard while it runs. The player does not need to give it the focus: they copy the item in the game with Ctrl+C (or Ctrl+Alt+C, the advanced copy), and the window updates.
- Only texts that look like a Path of Exile 2 item are taken into account: they start with the item class line, in English (`Item Class: `) or in French (`Classe d'objet: `). Any other copied text is ignored, including the text the application copies itself (a whisper), and the window keeps showing the last item.
- Copying the same item again after another text shows it again, with the latest known price. Two copies of the same item in a row leave the same text in the clipboard: the window doesn't change, and already shows that item.
- From the copied text, the application reads the item class, the rarity, the name, the base type (without the "Exceptional" prefix of an exceptional base), the stack size, the item level, the quality, the sockets, the requirements, the granted skill, the corrupted and other states, and the modifiers, with their ranges when the item was copied with Ctrl+Alt+C.

### Window layout

The window has a title bar, then three bands (item, price, offers), then a status bar.

| Window width | Layout |
|---|---|
| Narrow (version 1) | One column: item, price, offers. |
| Medium and wide (later) | The same blocks rearranged in two or three columns, with the price history in the wide layout (see the roadmap). |

- **Title bar** (band A, 28 px), in place of the title bar of the system: the emblem, the league block, then on the right the Settings, Minimize and Close icons, without borders (each clickable on 36 px across). There is no Maximize: the narrow window doesn't need it. The empty space of the bar moves the window; the edges of the window still resize it.
  - The league block shows the league name on a background tinted with the color of its trade (blue for softcore, red for hardcore), with a 2 px line of that color on the left and a white chevron. The color is the only sign of the trade on screen; the tooltip says it: "Softcore · Change the league". A click opens the list of the current leagues. By default, the current challenge league in softcore trade.
  - The gear opens the [settings](#settings), and closes them.
- **Status bar** (band A): the age of each source, e.g. "poe2scout 12 min ago · GGG trade 2 min ago"; "GGG trade: next search in 4 min" during a long wait; the version and the author while the settings are open.
- Warnings appear as a dark yellow band right under the title bar (see [Error handling](#error-handling)).
- The window can be resized and moved to another screen. Its size and position are restored at the next launch, on a screen that is still connected.
- Before the first item, the window shows the full logo and "Copy an item in the game with Ctrl + C. The price shows up here."

### Item block

- The item name, in its rarity color; for an exceptional base, the full name ("Exceptional Chiming Staff").
- A **Check on trade** pill at the top right ("Voir sur l'échange" in French), which opens the browser on the search for this item in the selected league, on the French trade site when the item was copied in French.
- The base type, then the item class followed by tags: Corrupted, Twice Corrupted, Sanctified, Fractured, Mirrored, Unidentified, quality (`Q 20%`), sockets (`2 sockets`), item level (`ilvl 81`), stack (`Stack 7/20`), and for rare items the number of modifiers in tiers 1 to 3 (`4 top tiers`) and the open affixes (`1 open prefix`). The tags wrap to a new line when needed.
- For a weapon, a summary line: "Physical DPS 245 · Elemental DPS 44"; for a staff, the granted skill.
- The requirements on a small gray line, always visible: "Requires Level 65 · 145 Dex".
- A **Show 6 stats** pill (singular for one stat; **Show filters** for an item without modifiers), and next to it the number of active filters ("3 filters"). Unfolded, the pill reads **Hide stats** and the stats appear:
  - grouped under small titles when they are filters: Item, Weapon, Pseudo, Implicit, Explicit;
  - each modifier in the game's modifier color, with a bar of where its value falls:
    - for a unique, in its fixed range, written next to the bar (`40–60`, from the price database);
    - for a rare or magic item or a jewel, over every tier of the modifier, so that a full bar means close to the best possible; next to it, its tier and the range of that tier (`T2 135–154`, from the advanced copy). Tier 1 is written in a vivid orange, tiers 2 and 3 in white bold, the others in gray. Without the advanced copy, the game gives no tier: only the value is shown;
  - for rare and magic items and jewels, the explicit modifiers are grouped into **Prefixes** and **Suffixes** with their fill ("2 of 3 · 1 open");
  - for filters, a checkbox on the left and the minimum value on the right (see [Stat filters](#stat-filters)).

### Price block

**Fixed-name items** (currency exchange and uniques):

- The price in **Divine Orbs**, **Chaos Orbs** and **Exalted Orbs**, side by side, each with its orb icon, so that the price is readable at any time of the league.
- The large price is always the value of what was copied: for a stack, the value of the whole stack, with "Stack of 7" on the right of the title and the unit price on a small line ("Unit price: 0.25 div · 2.49 c · 182 ex").
- On the right of the title, for a unique, the number of offers behind the price ("23 offers"); for a currency exchange item, which always has plenty of offers, the stack size instead.
- The trend, in green or red without icon: "Rising over the last 3 days (+4%)".
- With fewer than 5 offers, the warning "Only 4 offers: the price is uncertain".

**Estimated price** (rare, magic and normal items, jewels):

- Before the search: "No market price for rare items. Search similar offers to estimate it."
- After the search, the title becomes **Estimated price**: the median of the 5 cheapest similar offers (suspicious offers excluded), in the three currencies, with the number of similar offers on the right ("18 similar") and "Median of the 5 cheapest similar offers." under it. With fewer than 5 similar offers, the median of all of them and the warning "Only 3 similar offers: the estimate is uncertain".

**Special cases:**

| Situation | Behavior |
|---|---|
| Unidentified unique item | If its base type has only one unique, that unique is priced. Otherwise, "Identify the item to get its price." with the possible uniques and the price of each; its offers aren't searched. |
| Fixed-name item missing from the price data | "No price found for this item in <league>." |

### Stat filters

The offers search uses the stats of the copied item as filters, like the trade site, so that the offers found are comparable.

- **Unique items**: each modifier with a range is a filter, checked by default, with the value of the copied item as minimum. Modifiers without a range are the same on every copy and aren't filters.
- **Rare and magic items, jewels**: every modifier is a filter, with a minimum at **90 % of the copied value**, to find comparable items and not only identical ones. Default choices:
  - **pseudo totals** (total elemental resistance, total maximum life, total attributes…) checked, and the modifiers they add up unchecked;
  - for a weapon, the **physical and elemental DPS**, the physical one checked, and the damage and attack speed modifiers unchecked since the DPS already counts them;
  - the modifiers that remain checked; for a jewel, all its modifiers;
  - the **base type** checked; the **item level** unchecked.
- **Normal items** (e.g. an exceptional base): the base type, the number of sockets and the item level, checked; the quality, unchecked.
- The corrupted state is always a filter: a corrupted item is compared with corrupted offers only, a clean one with clean offers only.
- The player can check or uncheck a filter and change its minimum. Changing a filter doesn't search: the player clicks the search or refresh block, within its waiting time.

### Offers block

- **Before the first search**: the title "OFFERS", "One search per click, within the limits of the trade site.", and a white **search block** with a magnifying glass, glued to the right edge of the band. Copying an item never starts a search by itself: the player often copies many items in a row while sorting the loot.
- **While searching**: "Searching the trade site…".
- **After the search**: the title, "searched 2 min ago" under it, the count on the right ("10 of 13 online"), and in the top right corner, at the place of the search block, the **waiting block**: the countdown on a dark background ("8 s"), then, once a new search is allowed, the refresh icon on white.
  - The waiting time lasts at least 10 seconds after a search, longer when the limits announced by the trade site require it; it applies to every item. When it is long, "Trade site limit: next search in 4 min." under the title.
- **The list**: the first 10 offers, from the cheapest, among the sellers online or away. Each row shows the asked price with its orb and the listing time, with a dot for the seller status: green online, yellow away. Every other row is lighter.
  - Currency exchange items: the price per unit, the stock and the listing time.
  - An offer under a quarter of the reference price (the poe2scout price, or the estimate) is struck through and marked with "?": it is usually bait or a mistake, and it isn't counted in the estimate.
  - A double chevron under the list loads 10 more offers; the whole width under the list is clickable.
- **No offer**: "No offer online with these filters. Uncheck a stat or lower a minimum, then search again."
- **The tooltip of an offer**:
  - A row lights up one shade under the pointer, which shows it can be clicked. Hovering a row opens the tooltip of its item above the row, or below it when there is no room above. It stays open a few tenths of a second after the pointer leaves the row, and while the pointer is inside it. A click on the row pins it; a click elsewhere or Escape closes it.
  - It shows the name, the price with its orb, the base, item level and states, then the implicit and explicit modifiers, each variable one with its difference from the copied item: green when better, red when worse. Higher is better, except for a list of stats where lower is better (costs, "reduced" penalties…); a stat whose direction is unknown gets no color.
  - A suspicious offer adds "Be careful: far below the market price." in yellow under the name.
  - At the bottom, the seller status (dot and "Online", "Away" or "Offline") and, when the seller is online or away, a **Copy whisper** pill: it copies the whisper message of the trade site ("@seller Hi, I would like to buy your…") to the clipboard, then reads "Copied" with a check mark for 2 seconds, with "Paste it in the game chat: Enter, Ctrl+V, Enter." under it.
- The application never signs in to the player's account, so it never sends a direct whisper and never travels to a hideout.

### Settings

The gear opens the settings, which replace the content of the window in the narrow layout; the gear stays lit while they are open. A second click on the gear, or Escape, closes them. An item copied in the game closes them too, so that its price shows up.

- **Language**: *System* (default), *English* or *Français*.
  - *System*: the language of the system, when the interface has it (one of the preferred languages of the system, in their order); English otherwise.
  - *English* or *Français*: the interface stays in that language.
  - The language of the copied items never changes the interface, so that it never changes without the player asking: it only chooses the item names used to find the prices and the trade site to search (`fr.pathofexile.com` for a French item).
- **Always on top**: keeps the window above the other windows, for players with a single screen, when the game is in windowed or borderless mode. Off by default.

The settings are saved as soon as they change. Their bottom shows the sentence "This product isn't affiliated with or endorsed by Grinding Gear Games in any way." While they are open, the status bar shows the version and the author in place of the age of the sources: "Exile Market 0.1.0 · Made by Sylent · sylent.art" ("Créé par Sylent" in French), where `sylent.art` opens the author's site in the browser.

### Data sources

- The prices of the fixed-name items, their offer counts and their history come from a public price database of the Path of Exile 2 economy (see [roadmap.md](roadmap.md)). The application downloads the prices of the selected league at startup and when the league changes, then refreshes them every hour. A copied item is priced instantly from these local data, without a network request.
- The offers, the estimates and the item names in English and French come from the official trade site. The application searches only when the player clicks the search or refresh block, never in the background, and respects the limits announced by the trade site.
- The application identifies itself to both, and keeps its requests to a minimum.

## Local files

| File | Location | Content |
|---|---|---|
| Settings | The user's configuration directory (e.g. `~/.config/exile-market/` on Linux, `%APPDATA%\exile-market\config\` on Windows) | `settings.toml`: league, language, always on top. |
| Window | The same directory | `window.ron`: the size and position of the window. |
| Price cache | The user's cache directory | The last downloaded prices, used at startup and when the network is unavailable. |
| Item names cache | The user's cache directory | The item and stat names of the trade site in English and French, refreshed once a day. |
| Log | `~/.local/state/exile-market/` on Linux, `%LOCALAPPDATA%\exile-market\data\` on Windows, `~/Library/Logs/exile-market/` on macOS | `exile-market.log`: what happened during the session (start, items copied by their class line, clipboard errors, panics), to follow the application and find the cause of a bug; the log of the previous session is kept as `exile-market.previous.log`. The texts copied outside the game are never logged. |

None of these files are in the Git repository.

## Error handling

| Situation | Behavior |
|---|---|
| No network or price database unreachable at startup | The cached prices are used, with the warning band "The price database can't be reached: showing the prices from 3 hours ago."; without a cache, "Prices unavailable, retrying…" and a new attempt every minute. |
| Refresh failure | The previous prices are kept, with their age in the status bar; a new attempt at the next refresh. |
| Selected league no longer exists (end of league) | The default league is selected, with the warning band "<league> has ended, switched to <default league>." |
| Clipboard unreadable (e.g. a Wayland desktop that forbids it) | The warning band "The clipboard can't be read." The help to give (a README section or a panel in the window) is to be defined later. |
| Copied text that starts like an item but can't be read | "This item couldn't be read. Copy it again, or copy another item.", without crashing; the next copy works normally. |
| Trade site unreachable | The offers block shows "The trade site can't be reached"; the price and the stats are shown normally. |
| Trade site refuses a search (HTTP 429, e.g. the same IP address also used the trade site in a browser) | The waiting block waits for the delay given by the trade site (`Retry-After`); the application never retries by itself. |
| Settings file unreadable | The default settings are used, and the file is rewritten at the next change. |
