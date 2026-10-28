// Section State tags (13k2d): the states of the copied item as filters, and the new kinds of lines found
// in the items the owner copied on 2026-10-28 (13k2c).
// Decided (owner, 2026-10-28): every state tag gray at start, the offers with or without that state; lit
// after a click, the offers of that state only, its text in the light color of the state on a dark
// background of the same hue (light red on dark red for Corrupted); under the pointer, without any
// border, a lighter background and, on a gray tag, a brighter text; a click toggles a tag here; the note
// "Corrupted offers only, like this item." left out of the stats.
// Decided: the Vaal modifier of a corrupted unique (`{ Vaal Unique Modifier }`, French
// `{ Modificateur vaal unique }`; the trade site calls it mutated) says "Vaal" after its range, in every
// language (owner, 2026-10-28); the Runic Ward of a runeforged base, a defence without attribute, in a sky
// blue of its own (owner, 2026-10-28).
// Exiled Exchange 2 adds a row of buttons under the item for the states, and only a tag "Mutated" on the
// filter of a Vaal modifier.

// A state tag: `on` lit, `hover` its tooltip open; `fr` for a French item.
const STATE_TIPS = {
  corrupted: ["Corrupted offers only", "Corrupted or not", "Offres corrompues seulement", "Corrompues ou non"],
  twice: ["Twice corrupted offers only", "Corrupted or not", "Offres doublement corrompues seulement", "Corrompues ou non"],
  sanctified: ["Sanctified offers only", "Sanctified or not", "Offres sanctifiées seulement", "Sanctifiées ou non"],
  fractured: ["Fractured offers only", "Fractured or not", "Offres fissurées seulement", "Fissurées ou non"],
  mirrored: ["Mirrored offers only", "Mirrored or not", "Offres reflétées seulement", "Reflétées ou non"],
};
const state = (label, kind, on, { hover = false, fr = false } = {}) => ({
  label, state: kind === "twice" ? "corrupted" : kind, on, hover, tips: STATE_TIPS[kind].slice(fr ? 2 : 0, fr ? 4 : 2),
});

// Mirrored and fractured bow (the owner's copy, English), as copied: both gray, Mirrored under the pointer.
const BOW_MIRRORED = { rarity: "rare", name: "Storm Volley", base: "Warmonger Bow", cls: "Bows",
  tags: [state("Mirrored", "mirrored", false, { hover: true }), state("Fractured", "fractured", false), "Q 30%", "3 sockets", "ilvl 82"],
  props: [["Physical DPS", "720"]], req: "Requires Level 77 · 163 Dex", statsLabel: "Stats (7)", filters: "5 filters" };
const bowMirroredStats = [
  ...itemFilters({ base: "Warmonger Bow", level: { value: 82, check: false }, sockets: { value: 3, check: true }, quality: { value: 30, check: false } }),
  { group: "Weapon" },
  { mod: "Physical DPS: 720", check: true, min: 648 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "258% increased Physical Damage", check: true, min: 232, tier: 1, range: "170–179", over: { share: .95 } },
  { mod: "+181% Surpassing chance to fire an additional Arrow", check: false, min: 163, tier: 1, range: "150–200", over: { share: .6 } },
  { mod: "Adds 36 to 56 Physical Damage", check: false, tier: 2, range: "30–41 · 52–60", over: { share: .75 }, origin: "desecrated" },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+5% to Critical Hit Chance", check: false, min: 5, tier: 1, range: "4.1–5", over: { share: .95 }, origin: "fractured" },
  { mod: "+25% to Critical Damage Bonus", check: false, min: 23, tier: 2, range: "25–29", over: { share: .7 } },
  { mod: "+4 to Level of all Projectile Skills", check: true, min: 4, tier: 1, range: "4–4", over: { share: 1 } },
];

// Corrupted unique with a Vaal modifier (the owner's copy, French), after a click: Corrompu lit, under the
// pointer.
const BELT_VAAL_FR = { rarity: "unique", name: "Héritage du couard", base: "Ceinture en mailles", cls: "Ceintures",
  tags: [state("Corrompu", "corrupted", true, { hover: true, fr: true }), "niv. 79"], req: "Niveau requis 40",
  statsLabel: "Statistiques (6)", filters: "3 filtres", tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const beltVaalStatsFr = [
  { group: "Implicite", min: true },
  { mod: "11% de Réduction du nombre de Charges de Flacon utilisées", check: false, min: 11, range: "10–15", share: .2 },
  { mod: "Possède 1 Emplacement de Charme", check: false, min: 1, range: "1–3", share: 0 },
  { group: "Unique" },
  { mod: "36% d'Augmentation de la Récupération de Vie et de Mana des Flacons", check: true, min: 36, range: "30–40", share: .6 },
  { mod: "-18 de Force", check: false, range: "-20 – -10", share: .2 },
  { mod: "+112 au Score d'Évasion tant que votre Vie est Basse", check: true, min: 112, range: "100–150", share: .24, origin: "mutated" },
  { mod: "Votre Vie est désormais considérée comme Basse tant que vous avez 75% de votre Vie maximale ou moins" },
];

// Corrupted and fractured ring, after a click on each: both lit, Fractured under the pointer; the lit
// tags count among the active filters (4 checked filters and 2 tags).
const RING_STATES = { rarity: "rare", name: "Vortex Coil", base: "Prismatic Ring", cls: "Rings",
  tags: [state("Corrupted", "corrupted", true), state("Fractured", "fractured", true, { hover: true }), "ilvl 81"],
  req: "Requires Level 64", statsLabel: "Stats (5)", filters: "6 filters" };
const ringStatesStats = [
  ...itemFilters({ base: "Prismatic Ring", level: { value: 81, check: false } }),
  { group: "Implicit" },
  { mod: "+9% to all Elemental Resistances", check: false, min: 8, range: "7–10", share: .6 },
  { group: "Prefixes", fill: "2 of 3" },
  { mod: "+94 to maximum Mana", check: true, min: 85, tier: 3, range: "90–104", over: { share: .6 } },
  { mod: "+31 to maximum Life", check: true, min: 28, tier: 6, range: "25–34", over: { share: .4 } },
  { group: "Suffixes", fill: "2 of 3" },
  { mod: "+38% to Fire Resistance", check: true, min: 34, tier: 2, range: "36–40", over: { share: .8 }, origin: "fractured" },
  { mod: "+22 to Intelligence", check: false, min: 20, tier: 4, range: "21–24", over: { share: .5 } },
];

// Runeforged helmet (the owner's copy, English): the Runic Ward among the defences.
const HELMET_RUNEFORGED = { rarity: "rare", name: "Carrion Glance", base: "Runeforged Divine Crown", cls: "Helmets", tags: ["ilvl 80"],
  props: [["Armour", "314"], ["Energy Shield", "88"], ["Runic Ward", "56"]], req: "Requires Level 75 · 59 Str · 59 Int",
  statsLabel: "Stats (5)", filters: "5 filters" };
const helmetRuneforgedStats = [
  ...itemFilters({ base: "Runeforged Divine Crown", level: { value: 80, check: false } }),
  { group: "Prefixes", fill: "2 of 3" },
  { mod: "+94 to maximum Mana", check: true, min: 85, tier: 3, range: "90–104", over: { share: .6 } },
  { mod: "69% increased Armour and Energy Shield", check: true, min: 62, tier: 3, range: "68–79", over: { share: .55 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+28% to Cold Resistance", check: true, min: 25, tier: 4, range: "26–30", over: { share: .6 } },
  { mod: "15% increased Rarity of Items found", check: false, min: 14, tier: 1, range: "15–18", over: { share: .8 } },
  { mod: "35% increased Presence Area of Effect", check: true, min: 32, single: true, range: "25–35", share: 1, origin: "desecrated" },
];

// Kalandra's Touch (the owner's copy, French), after a click: Reflété lit; its only line shown without
// filter.
const KALANDRA_FR = { rarity: "unique", name: "Toucher de Kalandra", base: "Bague", cls: "Bagues",
  tags: [state("Reflété", "mirrored", true, { fr: true }), "niv. 80"], statsLabel: "Statistiques (1)", filters: "1 filtre",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const kalandraStatsFr = [
  { group: "Unique" },
  { mod: "Reflète la Bague opposée" },
];

SECTIONS.push(["State tags (13k2d): the states as filters, the Vaal modifier, the Runic Ward", [
  ["Mirrored and fractured, as copied: every state gray", win(topBar(), item({ ...BOW_MIRRORED, stats: bowMirroredStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Corrupted unique with a Vaal modifier, French: Corrompu clicked", win(topBar(), item({ ...BELT_VAAL_FR, stats: beltVaalStatsFr }, true), price(HH_PRICE), foot())],
  ["Corrupted and fractured: both clicked", win(topBar(), item({ ...RING_STATES, stats: ringStatesStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Runeforged base: the Runic Ward", win(topBar(), item({ ...HELMET_RUNEFORGED, stats: helmetRuneforgedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Mirrored unique, French: Reflété clicked, a line without filter", win(topBar(), item({ ...KALANDRA_FR, stats: kalandraStatsFr }, true), price(HH_PRICE), foot())],
]]);
