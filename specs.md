# Specifications

## Purpose

rust-exile-market is a small desktop application that gives the price of a Path of Exile 2 item, with its stats and the offers of the trade site, as soon as the player copies it in the game with Ctrl+C.

- Users: Path of Exile 2 players on Windows, macOS and Linux, including Wayland desktops such as Bazzite (KDE Plasma), where the game runs through Proton.
- Game client: English or French. The application understands the item text in both languages.
- Volume: one item at a time, compared against the prices of a few thousand items of the selected league, and one search on the trade site when the player asks for the offers.
- Version 1 prices every kind of item:
  - **fixed-name items** (currency and the other items of the currency exchange: fragments, essences, runes, soul cores, omens, uncut gems…, and unique items) from a public price database;
  - **rare, magic and normal items and jewels**, which have no market price, from the offers of similar items on the trade site.

The single source of truth of the interface is the mock-up [docs/mockups/narrow-styles.html](docs/mockups/narrow-styles.html) and its folder `docs/mockups/narrow-styles/` (style B · Slate). The wireframe [docs/wireframes/window-layouts.excalidraw](docs/wireframes/window-layouts.excalidraw) only fixed the first layout and isn't kept up to date.

## Design principles

- One small window, a companion to the game: beside the game on the same screen, or on a second screen.
- Minimalist: nothing that doesn't help to decide (no redundant text, no counter that says nothing, no decoration).
- Dark interface made of full-width bands, without frames or cards.
- White is the color of the main action; the other colors are kept for a meaning: the item rarities, the league (softcore or hardcore), the warnings, better or worse.
- Short, factual messages. No pop-ups for errors: they are shown inside the window.
- The interface is available in English and in French; English is the reference language. The French messages use the formal "vous", like the French game client. The tiers of the modifiers keep the English word the players use, in French too: T1, T2, T3… (the French client says "palier").
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
| Softcore league | background `#13263a` (`#1a3350` on hover) | League block and list. |
| Hardcore league | background `#351417` (`#4a1b20` on hover) | League block and list, background of the Corrupted tag. |
| Warning band | background `#8a6a10`, text `#f8edc2` | Errors under the title bar. |
| Warning text | `#f0b85a` | Uncertain price, suspicious offer, away seller. |
| Rise / fall | `#5fcf80` / `#e66a5c` | Trend, roll differences, seller online / offline. |
| Rarities | normal `#d0d0d0`, magic `#8888ff`, rare `#ffff77`, unique `#c26a2c`, currency `#b4a787`, gem `#1ba29b` | Item names, as in the game. |
| Modifiers | `#8f8fff` | Implicit and explicit modifiers, as in the game. |
| Attributes | strength `#e0473f`, dexterity `#3fbf4f`, intelligence `#5aa7f0` | The attributes of the requirements and the defences they raise (armour, evasion, energy shield), as in the game; a little apart from the rise and fall colors. |
| Tier 1 | `#ff8c1a` | Tier of a modifier at its best tier: a vivid orange that stands out. |

### Typography

- **Manrope** for all the text: item name 20 px weight 800, body 13.5 px, labels 11 px uppercase with letter spacing, small texts 11.5 to 12 px.
- **JetBrains Mono** for the numbers that line up (prices, offers), weight 600.
- Both fonts are under the SIL Open Font License and embedded in the executable.
- Numbers: a point between the integer and the decimal parts in every language, no thousands separator (`12.4`, `950`). At most three significant digits and two decimals, without trailing zeros (`12.4`, `0.25`, `123`); Exalted Orbs without decimals from 10 (`17`, but `0.1` for a cheap item); under 0.01, `<0.01`. From 1000, an amount is shortened with `K`, rounded to one decimal, without a trailing `.0` (`1.3K`, `27.6K`, `9K` for 8990); from 100 000, without decimals (`125K`); from a million, with `M` (`1.2M`). The same in every language.
- Currency abbreviations, the same in every language, as the players write them: `div`, `c`, `ex` ("Unit price: 0.25 div · 2.49 c · 182 ex").
- Durations: the number glued to its unit, without a space, in every language: `8s`, `12min`, `3h`, `5d` ("poe2scout 12min ago", "il y a 3h").

### Shapes

- **Action blocks are flat and square**, the height of their band, glued to its edge: the league block in the title bar, the search and refresh blocks in the offers band. The icons of the title bar have no block: only the icon.
- **Small information elements are rounded by 3 px**: tags, inputs, checkboxes (15 px, drawn check mark), pills.
- **Secondary actions are pills** with a verb: "Check on trade", "Show 6 stats", "Copy whisper".
- The tooltip of an offer is the only floating element: radius 4 px, shadow.

### Icons and logo

- Icons are drawn as lines (stroke about 1.8 px), in the color of the text: search, refresh, double chevron, check mark, minimize, close; the gear is the "settings" icon of [Lucide](https://lucide.dev) (ISC license).
- Icons are aligned on the lowercase letters next to them, by eye on the real screen (e.g. text 0.5 px up next to an icon), not on the box of the text.
- The title bar shows the **emblem** (`assets/logo/emblem.svg`, white, 16 px) without the name; the empty window shows the emblem as a faint watermark behind its text (190 px, 2.25 % opacity), so that it takes no height. The full **logo** (`assets/logo/exile-market.png`) is kept for the README and the application icon.
- The Exalted, Divine and Chaos Orbs come from `assets/currencies/`.

### Hover and motion

- Pills and the league block: background one shade lighter on hover.
- Icons of the title bar: in the weak text color, white on hover; the gear stays white while the settings are open.
- Search block: the icon grows by 15 %; refresh block: the icon grows by 12 % and turns a quarter; show more: the double chevron slides down 2 px. About 0.2s, a slight overshoot.
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
  - The league block shows the league name on a background tinted with the color of its trade (blue for softcore, red for hardcore), with a white chevron pointing right, which turns down while the list of the leagues is open. The color is the only sign of the trade on screen; the tooltip says it: "Softcore · Change the league". A hardcore league is shown without its "HC " prefix ("HC Forbidden Rites" shows as "Forbidden Rites", in red). A click opens the list of the leagues under the block: one line per league, its softcore trade on the left and its hardcore trade on the right, each on the color of its mode, glued together without a panel, 94 % opaque so that the item shows through a little, the corners of the list rounded by 3 px, with the shadow of the bubbles, 4 px under the title bar; without lines between them; the chosen league keeps the lighter background of the hover; the current leagues first, then the two permanent ones, both shown as "Standard". A name too long for its half is cut with "…" and shown whole in its tooltip. A click on a league chooses it and closes the list; a click elsewhere, Escape or the emblem closes it too. By default, the first current softcore league given by the price database.
  - The emblem goes back to the item (or to the empty window before the first item) from any screen, like the logo of a site to its home page: clickable from the left edge to the league block, with the tooltip "Home".
  - The gear opens the [settings](#settings), and closes them.
- **Status bar** (band A): the age of each source, e.g. "poe2scout 12min ago · GGG trade 2min ago"; "poe2scout 3h ago (offline)" while the price database can't be reached; "poe2scout: downloading prices…" while the prices of a league are downloaded and none of that league are known yet (first launch, or another league chosen without a cache); "poe2scout: no prices yet" when it can't be reached and no price is known; "GGG trade: next search in 4min" during a long wait; the version and the author while the settings are open. The ages follow the time without a new event.
- Warnings appear as a dark yellow band right under the title bar (see [Error handling](#error-handling)). Several warnings at once share the band, one line each, 4 px apart.
- The window can be resized and moved to another screen. Its size and position are restored at the next launch, on a screen that is still connected.
- Before the first item, the window shows the full logo and "Copy an item in the game with Ctrl + C. The price shows up here."
- While the prices of the league are downloaded and none of that league are known yet, this invitation would be wrong: it gives way to "Downloading the prices of Forbidden Rites" (the league name in the text color), a bar of 220 × 4 px that grows with each request (white on the color of the controls, corners rounded by 2 px), and a short funny sentence on money and the market in faint text under it, drawn at random at each download among ten (like Discord), e.g. "Counting the Exalted Orbs one by one…". An item copied meanwhile shows normally.

### Item block

- The item name, in its rarity color; for an exceptional base, the full name ("Exceptional Chiming Staff").
- A **Check on trade** pill at the top right ("Voir sur l'échange" in French), which opens the browser on the search for this item in the selected league, sellers online: a unique by its name and base type, an item of the currency exchange on its exchange page, the other items by their base type (their stats come with the filters of step 13); on the French trade site when the item was copied in French.
- The base type of a rare or unique item, with "Exceptional" for an exceptional base, then the item class in gray on the same line ("Wanderer Coat · Body Armours"); the class alone for the other items, named by their base.
- Then the facts of the item, 12 px under the base type, 6 px apart: the main values of the base, the skills it grants, the requirements, then the tags.
- The main values of the base on one line, the most important when choosing a weapon or an armour, each label in gray and its number in white bold: for a weapon, its DPS ("Physical DPS **245**  Elemental DPS **44**  Chaos DPS **30**", each kind it deals); for an armour piece or a shield, its defences ("Armour **412**  Evasion **380**", "Energy Shield **44**", "Block **26%**"), the label of each defence in the color of the attribute it comes from: armour red, evasion green, energy shield blue. The values of the game, with the quality, the modifiers and the runes. For a staff or a shield, the skills it grants, on a line of their own.
- The requirements on a gray line, the size of the summary, always visible: "Requires Level 65 · 145 Dex", each attribute in its color as in the game: strength red, dexterity green, intelligence blue; 3 px under the base values, which look farther apart otherwise.
- The tags: Corrupted, Twice Corrupted, Sanctified, Fractured, Mirrored, Unidentified, quality (`Q 20%`), sockets (`2 sockets`), item level (`ilvl 81`), stack (`Stack 7/20`). The tiers and the open affixes show in the stats only. The tags wrap to a new line when needed.
- A **Show 6 stats** pill, counting the modifiers (singular for one stat; **Show filters** for an item without modifiers), and next to it the number of active filters ("3 filters"). Unfolded, the pill reads **Hide stats** and the stats appear; they stay unfolded for the next items copied, until the player folds them (not kept between two launches):
  - grouped under small titles: for a rare or magic item, **Enchant**, **Runes**, **Implicit**, then the prefixes and suffixes (below), with the filter groups of step 13 on top (Item, Weapon, then **Pseudo**, the totals the trade site adds up from several modifiers, like Exiled Exchange 2); for a unique, **Enchant**, **Runes**, **Implicit** (the implicit modifiers of its base) and **Unique** (its own modifiers, no prefix nor suffix); "min", in small gray, on the right of the first title, above the minimums;
  - in each group, the stats that vary first, then those that never vary;
  - a stat that is a filter starts with its checkbox; a stat of a unique that never varies isn't a filter (the name of the unique already fixes it), has no checkbox and starts at the left edge;
  - the text of a stat on two lines at most, cut with "…", and whole in its tooltip; the lines of a modifier with several stats stay together;
  - the minimum on the line of the checkbox, in a column on the right kept for it (also on the lines without a minimum, so that the texts stay aligned): the text never runs under it; the bar and the range under the text;
  - each modifier in the game's modifier color, without the "fixed value" marker of the game (too long for the narrow layout), with a bar of where its value falls, only when its value varies: a line without number, or whose range is one value (`(5-5)`), has no bar; on a rare or magic item it is still a filter (e.g. a fixed implicit, "5% increased Movement Speed", to search every base that has it), with a checkbox and no minimum;
    - for a unique, in its fixed range, written next to the bar (`40–60`): from the advanced copy, or for an item copied with Ctrl+C from the modifiers of the unique in the price database, matched with the copied lines (in English only: an item copied in French gets its ranges from the advanced copy);
    - for a rare or magic item or a jewel, over every tier of the modifier, so that a full bar means close to the best possible; next to it, its tier and the range of that tier (`T2 135–154`, from the advanced copy). Tier 1 is written in a vivid orange, tiers 2 and 3 in white bold, the others in gray. For a modifier of several stats (a hybrid, e.g. "+20 to Evasion Rating" and "+10% increased Evasion Rating"), its lines follow each other, each with its checkbox and its minimum, and its tier and the ranges of all its lines go under the last one (`T5 15–25 · 8–12`), so that it reads as one modifier. Without the advanced copy, the game gives no tier: only the value is shown. Until the modifier tables (step 13), only the tier and its range, without a bar;
    - for an implicit modifier of a rare or magic item, in the range its base gives it, like a unique;
  - for rare and magic items and jewels, the explicit modifiers are grouped into **Prefixes** and **Suffixes** with their fill ("2 of 3 · 1 open");
  - for filters, a checkbox on the left and the minimum value on the right (see [Stat filters](#stat-filters)); the runes and the enchantment are filters too (the trade site searches them), unchecked by default, a rune being easy to change.

### Price block

**Fixed-name items** (currency exchange and uniques):

- The price in **Divine Orbs**, **Chaos Orbs** and **Exalted Orbs**, side by side, each with its orb icon, so that the price is readable at any time of the league.
- The large price is always the value of what was copied: for a stack, the value of the whole stack, with "Stack of 7" on the right of the title and the unit price on a small line ("Unit price: 0.25 div · 2.49 c · 182 ex").
- On the right of the title, for a unique, the number of offers behind the price ("23 offers"); for a currency exchange item, which always has plenty of offers, the stack size instead ("Stack of 7"; nothing for a single item, without a unit price line).
- The trend, the change first, in green when it rises and in red when it falls, without icon: "+4% over the last 3 days", "−11% over the last 7 days" (a real minus sign). It compares the price with the oldest known price of the last 7 days: 7 days, or fewer for an item listed lately. Under 1 % of change, "Stable over the last 7 days" in gray. No trend without a past price.
- With fewer than 5 offers, the warning "Only 4 offers: the price is uncertain".

**Estimated price** (rare, magic and normal items, jewels):

- Before the search: "No market price for rare items. Search similar offers to estimate it."
- After the search, the title becomes **Estimated price**: the median of the 5 cheapest similar offers (suspicious offers excluded), in the three currencies, with the number of similar offers on the right ("18 similar") and "Median of the 5 cheapest similar offers." under it. With fewer than 5 similar offers, the median of all of them and the warning "Only 3 similar offers: the estimate is uncertain".

**Special cases:**

| Situation | Behavior |
|---|---|
| Unidentified unique item | If its base type has only one unique, that unique is priced. Otherwise, "Identify the item to get its price." with the possible uniques and the price of each, in Divine Orbs from one Divine Orb and in Exalted Orbs under it; "No offers" in gray for a unique without a price. Its offers aren't searched. |
| Fixed-name item missing from the price data | "No price found for this item in <league>." |
| Fixed-name item whose price isn't known yet | The reason, in one line: "The prices of <league> are downloading…" during the download of a league without a cache, "Prices unavailable, retrying…" when the price database can't be reached and no price is known, "The French item names are downloading…" for an item copied in French (or an unidentified unique) before the names of the trade site arrive. |

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
- The minimum field takes a number only: digits, one decimal point, a minus sign first for a negative stat; no letter, not even the `e` of an exponent, also when a text is pasted. It stays within the range of every tier of the stat (the range of a unique; from the lowest roll of the worst tier to the highest of the best tier for a rare), not only the tier of the copied item.
- The player can check or uncheck a filter and change its minimum. Changing a filter doesn't search: the player clicks the search or refresh block, within its waiting time.

### Offers block

- **Before the first search**: the title "OFFERS", "One search per click, within the limits of the trade site.", and a white **search block** with a magnifying glass, glued to the right edge of the band. Copying an item never starts a search by itself: the player often copies many items in a row while sorting the loot.
- **While searching**: "Searching the trade site…".
- **After the search**: the title, "searched 2min ago" under it, the count on the right ("10 of 13 online"), and in the top right corner, at the place of the search block, the **waiting block**: the countdown on a dark background ("8s"), then, once a new search is allowed, the refresh icon on white.
  - The waiting time lasts at least 10 seconds after a search, longer when the limits announced by the trade site require it; it applies to every item. When it is long, "Trade site limit: next search in 4min." under the title.
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

- The prices of the fixed-name items, their offer counts and their history come from a public price database of the Path of Exile 2 economy (see [the technical choices](docs/technical-choices.md)). The application downloads the prices of the selected league at startup and when the league changes, then refreshes them every hour. A copied item is priced instantly from these local data, without a network request. During the download of a league without cache, the prices arrive category by category, the currency first, then the other items of the currency exchange, then the uniques: a copied orb is priced after a few seconds, without waiting for the whole download (about 26 seconds, one request per second).
- The offers, the estimates and the item names in English and French come from the official trade site. The application searches only when the player clicks the search or refresh block, never in the background, and respects the limits announced by the trade site.
- The application identifies itself to both, and keeps its requests to a minimum.

## Local files

| File | Location | Content |
|---|---|---|
| Settings | The user's configuration directory (e.g. `~/.config/exile-market/` on Linux, `%APPDATA%\exile-market\config\` on Windows) | `settings.toml`: league, language, always on top. |
| Window | The same directory | `window.ron`: the size and position of the window. |
| Price cache | The user's cache directory | The last downloaded prices, used at startup and when the network is unavailable. |
| Item names cache | The user's cache directory | The item names of the trade site in English and French (the stat names come with the filters), refreshed once a day; the old names stay while the trade site can't be reached, with a new attempt every hour (every minute while no name is known, unless the trade site refused). |
| Log | `~/.local/state/exile-market/` on Linux, `%LOCALAPPDATA%\exile-market\data\` on Windows, `~/Library/Logs/exile-market/` on macOS | `exile-market.log`: what happened during the session (start, items copied by their class line, clipboard errors, panics), to follow the application and find the cause of a bug; the log of the previous session is kept as `exile-market.previous.log`. The texts copied outside the game are never logged. |

None of these files are in the Git repository.

## Error handling

| Situation | Behavior |
|---|---|
| No network or price database unreachable at startup | The cached prices are used, with the warning band "The price database can't be reached. Last updated 3h ago."; without a cache, "Prices unavailable, retrying…". A new attempt every minute: a refresh stops at its first failed request, so an outage costs one request per minute. |
| Refresh failure | The previous prices are kept, with the same warning band and their age in the status bar ("poe2scout 3h ago (offline)"); a new attempt every minute, and the band goes away at the first success. |
| Selected league no longer exists (end of league) | The permanent league of the same mode is selected, without a message: it is where the game moves the characters. Both show as "Standard", softcore or hardcore told by their color. |
| A new challenge league has started while the player is on Standard | A bubble under the league block: "Rise of the Abyssal has started.", with a **Switch** pill that selects it and a close icon. Closed, it doesn't come back for that league (kept in the settings). |
| Clipboard unreadable (e.g. a Wayland desktop that forbids it) | The warning band "The clipboard can't be read." The help to give (a README section or a panel in the window) is to be defined later. |
| Copied text that starts like an item but can't be read | "This item couldn't be read. Copy it again, or copy another item.", without crashing; the next copy works normally. |
| Trade site unreachable | The offers block shows "The trade site can't be reached"; the price and the stats are shown normally. |
| Trade site refuses a search (HTTP 429, e.g. the same IP address also used the trade site in a browser) | The waiting block waits for the delay given by the trade site (`Retry-After`); the application never retries by itself. |
| Settings file unreadable | The default settings are used, and the file is rewritten at the next change. |
