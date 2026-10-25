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
# The number of prefixes and suffixes at tier 1, the best one.
tag-tier-1 = { $value } T1
# The summary of a weapon, its parts joined by " · ".
dps-physical = Physical DPS { $dps }
dps-elemental = Elemental DPS { $dps }
dps-chaos = Chaos DPS { $dps }
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
group-implicit = Implicit
group-prefixes = Prefixes
group-suffixes = Suffixes
group-explicit = Explicit
# Beside Prefixes and Suffixes: how many the item has, of the most it can have.
affix-fill = { $filled } of { $max }
affix-fill-open = { $filled } of { $max } · { $open } open

## Price block

price-label = Price
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
