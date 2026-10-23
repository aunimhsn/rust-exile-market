# Specifications

## Purpose

rust-exile-market is a small desktop application that gives the price of a Path of Exile 2 item, with its stats and the offers of the trade site, as soon as the player copies it in the game with Ctrl+C.

- Users: Path of Exile 2 players on Windows, macOS and Linux, including Wayland desktops such as Bazzite (KDE Plasma), where the game runs through Proton.
- Game client: English or French. The application understands the item text in both languages.
- Volume: one item at a time, compared against the prices of a few thousand items of the selected league, and one search on the trade site when the player asks for the offers.
- Version 1 prices the items with a fixed name: currency and the other items traded on the currency exchange (fragments, essences, runes, soul cores, omens, uncut gems…), and unique items, and shows their offers on the trade site. Rare, magic and normal items are recognized, with their stats, but not priced yet: they come in a later version, which must not require rewriting the rest of the application.

## Design principles

- One small window, a companion to the game: it can stay beside the game on the same screen, or be maximized on a second screen.
- The layout adapts to the size of the window: the larger the window, the more information it shows (see [Window layout](#window-layout)).
- Dark theme, close to the game's interface. The item name uses its rarity color, as in the game.
- Short, factual messages. No pop-ups: errors and warnings are shown inside the window.
- The interface is available in English and in French; English is the reference language. The French messages use the formal "vous", like the French game client.

## Features

### Item detection

- The application watches the clipboard while it runs. The player does not need to give it the focus: they copy the item in the game with Ctrl+C (or Ctrl+Alt+C, the advanced copy), and the window updates.
- Only texts that look like a Path of Exile 2 item are taken into account: they start with the item class line, in English (`Item Class:`) or in French. Any other copied text is ignored, and the window keeps showing the last item.
- Copying the same item again shows it again, with the latest known price.
- From the copied text, the application reads the item class, the rarity, the name, the base type, the stack size, the requirements and the modifiers, with their ranges when the item was copied with Ctrl+Alt+C. The modifiers are shown with the item (see [Item stats](#item-stats)); version 1 does not use them for the price, but rare items will.

### Price

For a supported item, the window shows:

- the item icon, as in the game; rare and unidentified items, which have no icon of their own, show the first letter of their item class;
- the item name, in its rarity color, and its base type for a unique item;
- its price in large, in the most readable currency: Divine Orbs from 1 Divine Orb, Exalted Orbs below;
- below it, its price in **Exalted Orbs**, **Divine Orbs** and **Chaos Orbs**, each with its icon, so that the price is readable at any time of the league, whichever currency the economy uses at that moment;
- for a stack (e.g. `Stack Size: 12/20`), the unit price and the price of the whole stack;
- the number of offers behind the price, to judge whether the item sells: with fewer than 5 offers, a warning says that the price is uncertain;
- an **Open on the trade site** button, which opens the browser on the search for this item in the selected league, on the French trade site when the item was copied in French.

Special cases:

| Situation | Behavior |
|---|---|
| Rare, magic or normal item | Its name and base type are shown, with the message "Rare items aren't priced yet" (or magic, normal), and the trade site link. |
| Unidentified unique item | If its base type has only one unique, that unique is priced. Otherwise, the message "Identify the item to get its price" is shown, with the list of the possible uniques. |
| Supported item missing from the price data | The message "No price found for this item in <league>". |
| Corrupted unique item | Priced like the others; the word "Corrupted" is shown next to its name. |

### Item stats

The stats of the copied item are shown under its name, as in the game tooltip:

- the requirements (e.g. "Requires: Level 50");
- the description of a currency item (e.g. "Removes a random modifier from an item");
- the implicit modifiers, then the explicit modifiers, in the game's modifier color;
- "Corrupted", in red, for a corrupted item.

Each modifier with a range shows a bar of where its value falls in the range, and the range itself (e.g. `40–60`). The ranges of unique items come from the price database; for the other items, they come from the advanced copy (Ctrl+Alt+C). In the narrow layout, the stats are folded under "Stats (6)"; a click unfolds them.

### Stat filters (unique items)

The value of a unique item depends on its rolls: the offers search uses the stats of the copied item as filters, like the trade site, so that all the offers found are comparable.

- In the stats of a unique item, each modifier with a range has a checkbox and a minimum value. They are checked by default, with the value of the copied item as minimum; the maximum is empty. The player can uncheck a stat or change its minimum before searching.
- Modifiers without a range (e.g. "When you kill a Rare monster, you gain its Modifiers for 60 seconds") are always the same on every copy: they are not filters.
- The corrupted state is a filter too: a corrupted item is compared with corrupted offers only, a clean one with clean offers only.
- The header of the stats shows the number of active filters (e.g. "Stats (6) · 3 filters"), also when the stats are folded.
- Changing a filter doesn't search: the player clicks **Search again**, within its waiting time.

### Offers

The offers block of a currency or unique item shows a **Search the offers** button. A click searches the official trade site for the item, in the selected league, among the sellers who are online, from the cheapest offer. It shows the first 10 offers; **Show the next 10** loads more. Copying an item never starts a search by itself: the player often copies many items in a row while sorting the loot.

- Unique item: the offers that match the [stat filters](#stat-filters-unique-items); for each offer, the asked price with its currency icon and the listing time (e.g. "2 h ago"). The filters make the offers comparable, so their stats are not shown.
- Currency exchange item: for each offer, the price per unit, the stock and the listing time.
- An offer under a quarter of the poe2scout price is struck through and marked with "?": it is usually bait or a mistake.
- The header shows the count (e.g. "10 of 13 online") and a **Trade site** link to the same search in the browser.
- While the search runs, "Searching the trade site…".
- After a search, the button becomes **Search again** and stays disabled for at least 10 seconds, longer when the limits announced by the trade site require it, with the remaining time on the button (e.g. "Search again · 8 s"). When the wait is long, a line explains it (e.g. "Trade site limit: next search in 4 min"). The wait applies to every item: copying another item doesn't reset it.
- Rare, magic, normal and unidentified items are not searched in version 1.
- The application never signs in to the player's account.

### Price history

When the window is wide enough, it also shows a chart of the item's price over the last 7 days, with the number of offers for each day. The history is loaded only when it is shown.

### Window layout

| Window width | Layout |
|---|---|
| Narrow (under about 500 px) | One column: the item with its stats folded, the price, the offers with their price and listing time only. |
| Medium | Two columns: the item and its stats on the left, the price and the offers on the right. |
| Wide (about 1000 px and more, e.g. maximized on a second screen) | Three columns: the item and its stats, the price and its 7-day history, the offers. |

- The window can be resized, maximized and moved to another screen. Its size and position are restored at the next launch.
- Before the first item, the window explains how to use the application: "Copy an item in the game with Ctrl+C".
- A status bar at the bottom shows the age of the prices (e.g. "Prices updated 12 minutes ago") and the data sources ("Prices: poe2scout · Offers: trade site").

### Settings

A gear button at the top right opens the settings:

- **League**: the list of the current leagues. By default, the current challenge league in softcore trade. The selected league is shown in the window header.
- **Language**: *Automatic* (default), *English* or *Français*.
  - *Automatic*: the interface starts in English, then follows the language of the last copied item: a French item switches it to French, an English item back to English.
  - *English* or *Français*: the interface stays in that language, whatever the language of the items.
- **Always on top**: keeps the window above the other windows, for players with a single screen. Off by default.

The settings are saved as soon as they change. The bottom of the settings shows the sentence "This product isn't affiliated with or endorsed by Grinding Gear Games in any way."

### Data sources

- The prices, the offer counts and the history come from a public price database of the Path of Exile 2 economy (see [roadmap.md](roadmap.md)). The application downloads the prices of the selected league at startup and when the league changes, then refreshes them every hour. A copied item is priced instantly from these local data, without a network request.
- The offers and the item names in English and French come from the official trade site. The application searches only when the player clicks **Search the offers**, never in the background, and respects the limits announced by the trade site.
- The application identifies itself to both, and keeps its requests to a minimum.

## Local files

| File | Location | Content |
|---|---|---|
| Settings | The user's configuration directory (e.g. `~/.config/rust-exile-market/` on Linux, `%APPDATA%\rust-exile-market\` on Windows) | League, language, always on top, window size and position. |
| Price cache | The user's cache directory | The last downloaded prices, used at startup and when the network is unavailable. |
| Icon cache | The user's cache directory | The icons of the items already shown, downloaded once each. |
| Item names cache | The user's cache directory | The item and stat names of the trade site in English and French, refreshed once a day. |

None of these files are in the Git repository.

## Error handling

| Situation | Behavior |
|---|---|
| No network or price database unreachable at startup | The cached prices are used, with their age; without a cache, the message "Prices unavailable, retrying…" and a new attempt every minute. |
| Refresh failure | The previous prices are kept, with their age; a new attempt at the next refresh. |
| Selected league no longer exists (end of league) | The default league is selected, with the message "<league> has ended, switched to <default league>". |
| Clipboard unreadable (e.g. a Wayland desktop that forbids it) | The message "The clipboard can't be read" with a link to the help in the README. |
| Copied text that starts like an item but can't be read | The message "This item couldn't be read", without crashing; the next copy works normally. |
| Trade site unreachable | The offers block shows "The trade site can't be reached"; the price, the stats and the history are shown normally. |
| Trade site refuses a search (HTTP 429, e.g. the same IP address also used the trade site in a browser) | The button waits for the delay given by the trade site (`Retry-After`), with the remaining time on it; the application never retries by itself. |
| Item icon unavailable (no network, missing image) | The first letter of the item class stands in for the icon; the price is shown normally. |
| Settings file unreadable | The default settings are used, and the file is rewritten at the next change. |
