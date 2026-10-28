# Interface texts in English, the reference language.
# Every message must also exist in fr.ftl.

## Title bar

# The mode of the league, said only by the color of its block and in its tooltip.
league-mode-softcore = Softcore
league-mode-hardcore = Hardcore
# Tooltip of the league block: its mode, then what a click does.
league-tooltip = { $mode } · Change the league
# In place of the league, before the leagues arrive at the first launch.
league-loading = Loading…
# Bubble under the league block, when a new league has started: its name, then a pill.
new-league = { $league } has started.
new-league-switch = Switch
# Tooltip of the emblem, which goes back to the item.
home-tooltip = Home
settings-tooltip = Settings
minimize-tooltip = Minimize
close-tooltip = Close

## Status bar

status-bar = { $prices } · { $trade }
# No price of the league, and the price database can't be reached.
status-prices-unavailable = poe2scout: no prices yet
# During the first download of the prices of a league.
status-prices-downloading = poe2scout: downloading prices…
# The age of the prices, e.g. "12min ago".
status-prices = poe2scout { $age }
status-prices-offline = poe2scout { $age } (offline)
status-trade-idle = GGG trade: no search yet
# The age of the last search of the trade site, e.g. "2min ago".
status-trade = GGG trade { $age }
# During a long wait before the next search, e.g. "4min".
status-trade-wait = GGG trade: next search in { $wait }
# While the settings are open: the version and the author; the site, last, is a link.
status-author = { $app } { $version } · Made by { $author } · { $site }

## Window before the first item

# Followed by the keys "Ctrl + C".
empty-copy = Copy an item in the game with
empty-price = The price shows up here.

## Copied item that can't be read

item-unreadable = This item couldn't be read.
item-unreadable-hint = Copy it again, or copy another item.

## Download of the prices

# In place of the invitation to copy an item, while no price of the league is known.
download-title = Downloading the prices of { $league }
# Under the bar, one drawn at random per download.
download-quote-1 = Counting the Exalted Orbs one by one…
download-quote-2 = Haggling with the merchants of Wraeclast…
download-quote-3 = Polishing the Divine Orbs…
download-quote-4 = Checking that no one sells a Mirror for 1 Chaos…
download-quote-5 = Ignoring the "still available?" whispers…
download-quote-6 = Sorting the stash tabs. Again…
download-quote-7 = Converting Chaos into Exalted, the math checks out…
download-quote-8 = Looking for the seller who went AFK in their hideout…
download-quote-9 = One request per second: poe2scout deserves respect…
download-quote-10 = Spotting the bait offers…

## Ages, rounded down to their largest unit

age-now = just now
age-minutes = { $minutes }min ago
age-hours = { $hours }h ago
age-days = { $days }d ago

## Waits, rounded up

# E.g. "8s", "4min".
wait-seconds = { $seconds }s
wait-minutes = { $minutes }min

## Warnings

clipboard-unreadable = The clipboard can't be read.
# The age of the prices, e.g. "3h ago".
prices-unreachable = The price database can't be reached. Last updated { $age }.
prices-unavailable = Prices unavailable, retrying…

## Item block

# Opens the search for the item on the trade site, in the browser.
check-on-trade = Check on trade
# The tags after the item class.
tag-corrupted = Corrupted
tag-twice-corrupted = Twice Corrupted
tag-sanctified = Sanctified
tag-fractured = Fractured
tag-mirrored = Mirrored
tag-unidentified = Unidentified
tag-quality = Q { $value }%
tag-sockets = { $value ->
    [one] { $value } socket
   *[other] { $value } sockets
}
tag-item-level = ilvl { $value }
tag-stack = Stack { $size }/{ $max }
# The main values of the base, each followed by its number in bold: the DPS of a weapon,
# the defences of an armour piece or a shield.
dps-physical = Physical DPS
dps-elemental = Elemental DPS
dps-chaos = Chaos DPS
defence-armour = Armour
defence-evasion = Evasion
defence-energy-shield = Energy Shield
defence-block = Block
# The summary of an item that grants skills, e.g. "Level 18 Sigil of Power", joined by " · ".
granted-skill = Grants Skill: { $skill }
# The requirements line, its parts joined by " · ": "Requires Level 65 · 145 Dex".
requires = Requires { $parts }
requirement-level = Level { $value }
requirement-strength = { $value } Str
requirement-dexterity = { $value } Dex
requirement-intelligence = { $value } Int

## Stats of the item

# The pill under the item block, and once unfolded.
show-stats = { $count ->
    [one] Show { $count } stat
   *[other] Show { $count } stats
}
hide-stats = Hide stats
# The small titles of the stats.
group-enchant = Enchant
group-runes = Runes
group-implicit = Implicit
group-prefixes = Prefixes
group-suffixes = Suffixes
group-explicit = Explicit
# The modifiers of a unique, no prefix nor suffix.
group-unique = Unique
# The filters of the base type, the item level, the sockets, the quality and the required level of a rare, magic or normal item.
group-item = Item
group-weapon = Weapon
# The totals the trade site adds up from several modifiers.
group-pseudo = Pseudo
# Beside Prefixes and Suffixes: how many the item has, of the most it can have.
affix-fill = { $filled } of { $max }
affix-fill-open = { $filled } of { $max } · { $open } open
# The pill of an item without modifiers, which has filters only, and once unfolded.
show-filters = Show filters
hide-filters = Hide filters
# Beside the pill: the filters checked for the search.
active-filters = { $count ->
    [one] { $count } filter
   *[other] { $count } filters
}
# On the right of the first small title, above the range fields of the filters.
filter-range = min – max
# The tooltip of a range field: the rolls it accepts, then for a line better lower, in gray.
filter-rolls = Rolls { $low } to { $high }
filter-lower-better = Lower is better
# The filters of the properties of an item, with their value.
filter-base-type = Base: { $base }
# Under the base type unchecked: the item class of the copied item, searched in its place.
filter-item-class = Class searched: { $class }
filter-item-level = Item level: { $value }
filter-sockets = Sockets: { $value }
filter-quality = Quality: { $value }%
filter-required-level = Required level: { $value }
filter-physical-dps = Physical DPS: { $value }
filter-elemental-dps = Elemental DPS: { $value }
# The filters of the pseudo totals, with their value.
pseudo-elemental-resistance = +{ $value }% total Elemental Resistance
pseudo-chaos-resistance = +{ $value }% total to Chaos Resistance
pseudo-life = +{ $value } total maximum Life
pseudo-mana = +{ $value } total maximum Mana
pseudo-strength = +{ $value } total to Strength
pseudo-dexterity = +{ $value } total to Dexterity
pseudo-intelligence = +{ $value } total to Intelligence
pseudo-all-attributes = +{ $value } total to all Attributes
# The tooltip of a line the trade site can't search (a modifier added by a patch), on two lines.
filter-unsearchable = The trade site can't search this line.
filter-unsearchable-hint = The offers may not have it.
# Under the stats of a corrupted item: the search compares it with corrupted offers only.
corrupted-offers-only = Corrupted offers only, like this item.

## Price block

price-label = Price
# The price of a rare, magic or normal item, from the offers of similar items.
estimated-price-label = Estimated price
# On the right of the price title.
offers = { $count ->
    [one] { $count } offer
   *[other] { $count } offers
}
stack-of = Stack of { $size }
# Under the price of a stack: "Unit price: 0.25 div · 2.49 c · 182 ex".
unit-price = Unit price: { $prices }
# The change first, with a real minus sign.
trend-rise = +{ $percent }% over the last { $days } days
trend-fall = −{ $percent }% over the last { $days } days
trend-stable = Stable over the last { $days } days
uncertain = { $count ->
    [0] No offer lately: the price is uncertain
    [one] Only { $count } offer: the price is uncertain
   *[other] Only { $count } offers: the price is uncertain
}
# On the right of the title of an estimated price: the similar offers found.
similar = { $count ->
    [one] { $count } similar
   *[other] { $count } similar
}
# Under an estimated price from fewer than 5 offers, the suspicious ones left out.
estimate-uncertain = { $count ->
    [one] Only { $count } similar offer: the estimate is uncertain
   *[other] Only { $count } similar offers: the estimate is uncertain
}
# Under an estimated price: how it is made.
estimate-median = { $count ->
    [one] Price of the only similar offer.
   *[other] Median of the { $count } similar offers.
}
estimate-median-cheapest = Median of the { $count } cheapest similar offers.
# An unidentified unique whose base type has several uniques.
identify-to-price = Identify the item to get its price.
possible-uniques = Possible uniques
# In place of the price of a possible unique without offers.
no-offers = No offers
# Rare, magic and normal items, priced from the offers of similar items.
no-market-price-rare = No market price for rare items.
no-market-price-magic = No market price for magic items.
no-market-price-normal = No market price for normal items.
search-to-estimate = Search similar offers to estimate it.
# A fixed-name item whose price isn't known yet, or missing.
prices-downloading = The prices of { $league } are downloading…
# Before the league is known, at the first launch.
prices-downloading-no-league = Downloading the prices…
french-names-downloading = The French item names are downloading…
no-price-found = No price found for this item in { $league }.

## Offers block

offers-label = Offers
# Before the first search of the item, beside the search block.
offers-search-hint = One search per click, within the limits of the trade site.
offers-search-tooltip = Search the offers
offers-searching = Searching the trade site…
# Under the title, e.g. "searched 2min ago".
offers-searched = searched { $age }
# On the right of the title: the offers shown, of those found.
offers-count = { $shown } of { $total } online
# The titles of the columns of the offers.
offers-column-price = Price
offers-column-per-unit = Per unit
offers-column-stock = Stock
offers-column-listed = Listed
offers-again-tooltip = Search again
# The tooltip of the dark block while the answer of the last search or of more offers is
# awaited.
offers-busy-tooltip = Waiting for the answer of the trade site
# The tooltip of the countdown before the next search, e.g. "8s".
offers-next-search-tooltip = Next search in { $wait }
# Under the title, during a long wait, e.g. "4min".
offers-limit = Trade site limit: next search in { $wait }.
offers-none = No offer online with these filters.
offers-none-hint = Uncheck a stat or lower a minimum, then search again.
offers-unreachable = The trade site can't be reached.
offers-identify = Identify the item to search its offers.
# The double chevron under the list.
offers-more-tooltip = Show more offers
# The tooltip of an offer: a price far below the market, the seller status, and the pill
# that copies the whisper message of the trade site.
offers-suspicious = Be careful: far below the market price.
seller-online = Online
seller-away = Away
seller-offline = Offline
copy-whisper = Copy whisper
whisper-copied = Copied

## Settings

settings-title = Settings
# Label above the choices of the language.
settings-language = Language
# The language of the system, when the interface has it.
language-system = System
# Each language is named in itself, so that a player finds theirs in any language.
language-english = English
language-french = Français
settings-always-on-top = Always on top
# Required by Grinding Gear Games for the tools of the community.
disclaimer = This product isn't affiliated with or endorsed by Grinding Gear Games in any way.
