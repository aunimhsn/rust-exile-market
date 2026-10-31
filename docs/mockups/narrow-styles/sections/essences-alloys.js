// Section Essences and alloys (13k3d2): a modifier only an essence adds and one an alloy of Verisium adds,
// both of one tier, whose tier the game doesn't write. The advanced copy writes the alloy crafted
// (`{ Crafted Suffix Modifier "of the Stars" }`, French `{ Artisanat Suffixe : « des Étoiles » }`, the
// owner's gloves) and the essence as an ordinary suffix (`{ Suffix Modifier "of the Essence" — Gold }`);
// the trade site files both `crafted` (`S0`). One open decision, three options:
// - A, as the game writes them and as the application shows them today: the essence without word, like
//   an ordinary modifier of one tier, the alloy "Crafted" in the cyan of the crafted modifiers;
// - B, the currency named after the range, in the cyan of the crafted modifiers: "Essence", "Alloy"
//   (French "Essence", "Alliage"), from the modifier tables, so that a line without tier says why;
// - C, both "Crafted", as the trade site files them.
// Chosen by the owner on 2026-10-31: B, "so that we know where the affix comes from"; the other options
// kept below it, for the record.

// The owner's gloves (`inputs/verisium.txt`), with a suffix of an essence of Greed added.
const ALLOY_GLOVES = { rarity: "rare", name: "Pandemonium Grip", base: "Adorned Wraps", cls: "Gloves",
  tags: ["ilvl 80"], props: [["Evasion Rating", "147"], ["Energy Shield", "46"]],
  req: "Requires Level 59 · 41 Dex · 41 Int", statsLabel: "Stats (8)", filters: "3 filters" };
// `essence` and `alloy`: what follows the range of each line, `{ origin, originLabel }`.
const alloyGlovesStats = ({ essence, alloy }) => [
  ...itemFilters({ base: "Adorned Wraps", level: { value: 80, check: false } }),
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "+73 to Evasion Rating", check: false, min: 66, over: { share: .9 } },
  { mod: "+23 to maximum Energy Shield", check: false, min: 21, over: { share: .9 } },
  { rollOnly: true, tier: 1, range: "65–78 · 22–25" },
  { mod: "+137 to maximum Life", check: true, min: 123, tier: 1, range: "120–149", over: { share: .9 } },
  { mod: "Adds 1 to 29 Lightning damage to Attacks", check: false, tier: 5, range: "1 · 28–32", over: { share: .45 } },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "11% increased Attack Speed while missing Runic Ward", check: true, min: 10, single: true, range: "10–15", share: .2, ...alloy },
  { mod: "13% increased Quantity of Gold Dropped by Slain Enemies", check: false, min: 12, single: true, range: "10–15", share: .6, ...essence },
];
const ALLOY_GLOVES_FR = { ...ALLOY_GLOVES, name: "Prise pandémoniaque", base: "Mitaines adornées", cls: "Gants",
  tags: ["niv. 80"], props: [["Score d'Évasion", "147"], ["Bouclier d'énergie", "46"]],
  req: "Niveau requis 59 · 41 Dex · 41 Int", statsLabel: "Statistiques (8)", filters: "3 filtres",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const alloyGlovesStatsFr = ({ essence, alloy }) => [
  { group: "Objet", min: true },
  { mod: "Base : Mitaines adornées", check: true },
  { mod: "Niveau de l'objet : 80", check: false, min: 80 },
  { group: "Préfixes", fill: "3 sur 3" },
  { mod: "+73 au Score d'Évasion", check: false, min: 66, over: { share: .9 } },
  { mod: "+23 de Bouclier d'énergie maximal", check: false, min: 21, over: { share: .9 } },
  { rollOnly: true, tier: 1, range: "65–78 · 22–25" },
  { mod: "+137 de Vie maximale", check: true, min: 123, tier: 1, range: "120–149", over: { share: .9 } },
  { mod: "Ajoute 1 à 29 Dégâts de Foudre aux Attaques", check: false, tier: 5, range: "1 · 28–32", over: { share: .45 } },
  { group: "Suffixes", fill: "2 sur 3 · 1 libre" },
  { mod: "11% d'Augmentation de Vitesse d'Attaque tant qu'il vous manque de la Sauvegarde runique", check: true, min: 10, single: true, range: "10–15", share: .2, ...alloy },
  { mod: "13% d'Augmentation de la Quantité d'Or cédée par les Ennemis Tués", check: false, min: 12, single: true, range: "10–15", share: .6, ...essence },
];

const CRAFTED = { origin: "crafted" };
const gloves = (essence, alloy) => win(topBar(), item({ ...ALLOY_GLOVES, stats: alloyGlovesStats({ essence, alloy }) }, true), price(RARE_ESTIMATE_BEFORE), foot());
const glovesFr = (essence, alloy) => win(topBar(), item({ ...ALLOY_GLOVES_FR, stats: alloyGlovesStatsFr({ essence, alloy }) }, true), price(RARE_ESTIMATE_BEFORE), foot());

SECTIONS.unshift(["Essences and alloys (13k3d2): the word after a modifier of an essence or an alloy", [
  ["Chosen · B, the currency named: \"Essence\", \"Alloy\"", gloves({ ...CRAFTED, originLabel: "Essence" }, { ...CRAFTED, originLabel: "Alloy" })],
  ["Chosen · B, French", glovesFr({ ...CRAFTED, originLabel: "Essence" }, { ...CRAFTED, originLabel: "Alliage" })],
  ["A, before: essence without word, alloy \"Crafted\"", gloves({}, CRAFTED)],
  ["C, both \"Crafted\", as the trade site", gloves(CRAFTED, CRAFTED)],
]]);
