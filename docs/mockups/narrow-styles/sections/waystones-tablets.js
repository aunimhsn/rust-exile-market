// Section Waystones and tablets (13k3e): their modifiers have one tier per band of waystones (tiers 1–5,
// 6–10, 11–15, 16) and one per tablet, so the rule of 13k1 applies: no tier, the range of the copy, the bar
// over it. Found from the values in a copy of the in-game trade as well, which today finds none (no table)
// and puts every line in Explicit. The owner's copies (`inputs/polish-13-en.txt`, `inputs/polish-13-fr.txt`).
// A line without number (`Monsters are Armoured`, 35–55% of their strength added to their armour, which the
// game doesn't write) has no range nor bar.
// One open decision, the fill of a corrupted waystone: seen on the trade site on 2026-10-31 (40 offers,
// league Runes of Aldur), a rare waystone has 3 prefixes and 3 suffixes at most, a corrupted one up to 4
// and 4, or 6 of one affix and none of the other (13 of 40), never more than 8 modifiers:
// - A, the fill over 6, the most of one affix: "6 of 6", "4 of 6 · 2 open";
// - B, the count alone on a corrupted waystone, which nothing can change any more: "6".
// Today: "6 of 3". Chosen by the owner on 2026-10-31: B; option A kept below it, for the record.

// The owner's waystone of tier 12, from the advanced copy.
const WAYSTONE = { rarity: "rare", name: "Arcane Choice", base: "Waystone (Tier 12)", cls: "Waystones",
  tags: ["ilvl 80"], statsLabel: "Stats (5)", filters: "4 filters" };
const waystoneStats = [
  ...itemFilters({ base: "Waystone (Tier 12)", level: { value: 80, check: false } }),
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "Monsters have 91% increased Stun Buildup", check: true, min: 90, single: true, range: "90–100", share: .1 },
  { mod: "18% increased Monster Damage", check: true, min: 16, single: true, range: "15–19", share: .75 },
  { group: "Suffixes", fill: "1 of 3 · 2 open" },
  { mod: "Monster have 106% increased Elemental Ailment Application", check: true, min: 100, single: true, range: "100–119", share: .32 },
];
// Today: the advanced copy writes T1, the in-game trade nothing.
const waystoneStatsBefore = [
  ...itemFilters({ base: "Waystone (Tier 12)", level: { value: 80, check: false } }),
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "Monsters have 91% increased Stun Buildup", check: true, min: 82, tier: 1, range: "90–100", noBar: true },
  { mod: "18% increased Monster Damage", check: true, min: 16, tier: 1, range: "15–19", noBar: true },
  { group: "Suffixes", fill: "1 of 3 · 2 open" },
  { mod: "Monster have 106% increased Elemental Ailment Application", check: true, min: 95, tier: 1, range: "100–119", noBar: true },
];

// The owner's French waystone of tier 12, copied from the in-game trade.
const WAYSTONE_FR = { rarity: "rare", name: "Intention génitrice", base: "Pierre de téléportation (Palier 12)",
  cls: "Pierres de téléportation", tags: ["niv. 78"], statsLabel: "Statistiques (6)", filters: "5 filtres",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const waystoneStatsFr = [
  { group: "Objet", min: true },
  { mod: "Base : Pierre de téléportation (Palier 12)", check: true },
  { mod: "Niveau de l'objet : 78", check: false, min: 78 },
  { group: "Préfixes", fill: "2 sur 3 · 1 libre" },
  { mod: "Les Monstres ont 13% d'Augmentation de la Vitesse dAttaque, d'incantation et de déplacement", check: true, min: 12, single: true, range: "10–15", share: .6 },
  { mod: "24% Davantage de Vie des Monstres", check: true, min: 22, single: true, range: "20–25", share: .8 },
  { group: "Suffixes", fill: "2 sur 3 · 1 libre" },
  { mod: "Les Monstres ont 101% d'Augmentation d'Application des Altérations Élémentaires", check: true, min: 100, single: true, range: "100–119", share: .05 },
  { mod: "+33% à la Résistance élémentaire des Monstres", check: true, min: 30, single: true, range: "30–34", share: .75 },
  { note: "Les paliers sont trouvés d'après les valeurs." },
];

// The owner's magic tablet, its base found among the names of the trade site.
const TABLET = { rarity: "magic", name: "Bountiful Irradiated Tablet", cls: "Tablet",
  tags: ["ilvl 77"], statsLabel: "Stats (4)", filters: "3 filters" };
const tabletStats = [
  ...itemFilters({ base: "Irradiated Tablet", level: { value: 77, check: false } }),
  { group: "Implicit" },
  { mod: "Adds Irradiated to a Map" },
  { mod: "10 uses remaining", check: true, min: 9 },
  { group: "Prefixes", fill: "1 of 1" },
  { mod: "30% increased Gold found in Map", check: true, min: 27, single: true, range: "25–35", share: .5 },
];

// A corrupted waystone of tier 15 with six suffixes, as the trade site showed several, copied from the
// in-game trade; `fill` is the fill of its suffixes.
const WAYSTONE_CORRUPTED = { rarity: "rare", name: "Grim Path", base: "Waystone (Tier 15)", cls: "Waystones",
  tags: ["Corrupted", "ilvl 82"], statsLabel: "Stats (8)", filters: "7 filters" };
const corruptedStats = fill => [
  ...itemFilters({ base: "Waystone (Tier 15)", level: { value: 82, check: false } }),
  { group: "Suffixes", fill },
  { mod: "47% less effect of Curses on Monsters", check: true, min: 42, single: true, range: "40–50", share: .7 },
  { mod: "Players have 25% less Cooldown Recovery Rate", check: true, min: 25, max: 28, lower: true, single: true, range: "25–30", share: 0 },
  { mod: "Players have 40% less Recovery Rate of Life and Energy Shield", check: true, min: 40, max: 40, lower: true, single: true, range: "36–40", share: 1 },
  { mod: "Area has patches of Shocked Ground", check: true },
  { mod: "Monsters are Evasive", check: true },
  { mod: "Monsters are Armoured", check: true },
  SUMMED_NOTE,
];
// The same waystone with four prefixes and four suffixes, for decision A.
const corruptedEightStats = (prefixes, suffixes) => [
  ...itemFilters({ base: "Waystone (Tier 15)", level: { value: 82, check: false } }),
  { group: "Prefixes", fill: prefixes },
  { mod: "Monsters have 229% increased Critical Hit Chance", check: true, min: 220, over: { share: .73 } },
  { mod: "Monsters have +22% Critical Damage Bonus", check: true, min: 21, over: { share: .25 } },
  { rollOnly: true, range: "200–240 · 21–25" },
  { mod: "Monsters deal 19% of Damage as Extra Lightning", check: true, min: 17, single: true, range: "15–19", share: 1 },
  { mod: "24% more Monster Life", check: true, min: 22, single: true, range: "20–25", share: .8 },
  { mod: "Monsters have 12% increased Attack, Cast and Movement Speed", check: true, min: 11, single: true, range: "10–15", share: .4 },
  { group: "Suffixes", fill: suffixes },
  { mod: "Monsters have 76% increased Ailment Threshold", check: true, min: 70, over: { share: .67 } },
  { mod: "Monsters have 64% increased Stun Threshold", check: true, min: 60, over: { share: .44 } },
  { rollOnly: true, range: "70–79 · 60–69" },
  { mod: "+31% Monster Elemental Resistances", check: true, min: 30, single: true, range: "30–34", share: .25 },
  { mod: "Monsters are Evasive", check: true },
  { mod: "Players gain 31% reduced Flask Charges", check: true, min: 28, single: true, range: "25–35", share: .6 },
  SUMMED_NOTE,
];
const waystoneWin = (it, stats) => win(topBar(), item({ ...it, stats }, true), price(RARE_ESTIMATE_BEFORE), foot());

SECTIONS.unshift(["Waystones and tablets (13k3e): their modifiers without tier, and the fill of a corrupted waystone", [
  ["Waystone, advanced copy: no tier, its bar over its range", waystoneWin(WAYSTONE, waystoneStats)],
  ["Waystone, French, in-game trade: its prefixes and suffixes found", waystoneWin(WAYSTONE_FR, waystoneStatsFr)],
  ["Tablet, magic: its prefix without tier", win(topBar(), item({ ...TABLET, stats: tabletStats }, true), price({ lines: ["No market price for magic items.", "Search similar offers to estimate it."] }), foot())],
  ["Chosen · B, a corrupted waystone: the count alone", waystoneWin(WAYSTONE_CORRUPTED, corruptedStats("6"))],
  ["Chosen · B, four and four", waystoneWin({ ...WAYSTONE_CORRUPTED, statsLabel: "Stats (12)", filters: "11 filters" }, corruptedEightStats("4", "4"))],
  ["A, over 6, the most of one affix", waystoneWin(WAYSTONE_CORRUPTED, corruptedStats("6 of 6"))],
  ["A, four and four", waystoneWin({ ...WAYSTONE_CORRUPTED, statsLabel: "Stats (12)", filters: "11 filters" }, corruptedEightStats("4 of 6 · 2 open", "4 of 6 · 2 open"))],
  ["Before · the advanced copy writes T1", waystoneWin(WAYSTONE, waystoneStatsBefore)],
]]);
