// Section Unique from the in-game trade (13k2e5): a unique copied from the trade of the game has no range,
// so that nothing tells whether a line varies (`+51 to maximum Energy Shield` of Soul Tether does, `You
// lose 5% of maximum Energy Shield per second` doesn't). From the owner's copies (inputs/13k2c-items.txt,
// 2026-10-28). One open decision, three options:
// - A, as today and as Exiled Exchange 2 (its code, commit cca3066: `hideNotVariableStat`): each value taken
//   for its only roll, every line fixed, the unique searched by its name;
// - B, each line with a number a filter, unchecked, at its value, without bar nor range: the player checks
//   what matters to them, but a line that never varies becomes a filter too;
// - C, the rolls of poe2scout: the price database writes the modifiers of each unique with their ranges
//   (`+(40-60) to maximum Energy Shield`, `You lose (5-5)% …`), in the answer the application already
//   downloads with the prices (checked on the service on 2026-10-30, 85 of 85 accessories). Each line,
//   matched by its stat (so in French too), shows as in the advanced copy: a range of one value is fixed;
//   a line poe2scout doesn't hold (the Vaal modifier, the Intelligence of the French copy) as in B.
// Chosen by the owner on 2026-10-30: C, its note "The rolls are those of poe2scout." without "Copied from
// the in-game trade": the player needs to know where the rolls come from, not why the copy has none, and
// the application only sees a copy without headers, not where it was made. On 2026-10-30 too, the implicits
// with a range checked like every line of a unique that varies (as the application does), and the stats
// that vary first, also in French (the copy writes the Vaal modifier last in French, first in English).

// Soul Tether (the owner's copy, English), corrupted, as copied: its state tag gray.
const SOUL_TETHER = { rarity: "unique", name: "Soul Tether", base: "Long Belt", cls: "Belts",
  tags: [state("Corrupted", "corrupted", false), "ilvl 80"], req: "Requires Level 20", statsLabel: "Stats (7)" };
const SOUL_TETHER_PRICE = { values: ["<0.01", "0.01", "1"], offers: "3981 offers", trend: "Stable over the last 7 days", stable: true };
const soulTetherEnchant = [
  { group: "Enchant" },
  { mod: "15% increased Evasion Rating", check: false },
];
const UNIQUE_ROLLS_NOTE = { note: "The rolls are those of poe2scout." };

// A: every line fixed, no filter.
const soulTetherA = [
  { group: "Enchant" },
  { mod: "15% increased Evasion Rating" },
  { group: "Implicit" },
  { mod: "16% increased Charm Effect Duration" },
  { mod: "Has 3 Charm Slots" },
  { group: "Unique" },
  { mod: "14% increased Energy Shield Recovery rate", origin: "mutated" },
  { mod: "+51 to maximum Energy Shield" },
  { mod: "You lose 5% of maximum Energy Shield per second" },
  { mod: "Excess Life Recovery from Leech is applied to Energy Shield" },
];

// B: each line with a number a filter, unchecked, at its value.
const soulTetherB = [
  ...soulTetherEnchant,
  { group: "Implicit" },
  { mod: "16% increased Charm Effect Duration", check: false, min: 16 },
  { mod: "Has 3 Charm Slots", check: false, min: 3 },
  { group: "Unique" },
  { mod: "14% increased Energy Shield Recovery rate", check: false, min: 14, origin: "mutated" },
  { mod: "+51 to maximum Energy Shield", check: false, min: 51 },
  { mod: "You lose 5% of maximum Energy Shield per second", check: false, min: 5 },
  { mod: "Excess Life Recovery from Leech is applied to Energy Shield" },
];

// C: the rolls of poe2scout, as in the advanced copy; the Vaal modifier, which poe2scout doesn't hold, as in B.
const soulTetherC = [
  ...soulTetherEnchant,
  { group: "Implicit" },
  { mod: "16% increased Charm Effect Duration", check: true, min: 16, range: "15–20", share: .2 },
  { mod: "Has 3 Charm Slots", check: true, min: 3, range: "1–3", share: 1 },
  { group: "Unique" },
  { mod: "14% increased Energy Shield Recovery rate", check: false, min: 14, origin: "mutated" },
  { mod: "+51 to maximum Energy Shield", check: true, min: 51, range: "40–60", share: .55 },
  { mod: "You lose 5% of maximum Energy Shield per second" },
  { mod: "Excess Life Recovery from Leech is applied to Energy Shield" },
  UNIQUE_ROLLS_NOTE,
];

// Soul Tether (the owner's copy, French): a line poe2scout doesn't hold, `+20 d'Intelligence`, as in B.
const SOUL_TETHER_FR = { rarity: "unique", name: "Lanière animique", base: "Ceinture longue", cls: "Ceintures",
  tags: [state("Corrompu", "corrupted", false, { fr: true }), "niv. 80"], req: "Niveau requis 20",
  statsLabel: "Statistiques (8)", filters: "3 filtres", tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const soulTetherCFr = [
  { group: "Enchantement" },
  { mod: "15% d'Augmentation de l'Armure", check: false },
  { group: "Implicite" },
  { mod: "16% d'Augmentation de la Durée de l'Effet des Charmes", check: true, min: 16, range: "15–20", share: .2 },
  { mod: "Possède 2 Emplacements de Charme", check: true, min: 2, range: "1–3", share: .5 },
  { group: "Unique" },
  { mod: "+40 de Bouclier d'énergie maximal", check: true, min: 40, range: "40–60", share: 0 },
  { mod: "+20 d'Intelligence", check: false, min: 20 },
  { mod: "14% d'Augmentation de la Récupération de Bouclier d'énergie", check: false, min: 14, origin: "mutated", originLabel: "Vaal" },
  { mod: "Vous perdez 5% de Bouclier d'énergie par seconde" },
  { note: "Les jets sont ceux de poe2scout." },
];

SECTIONS.unshift(["Unique from the in-game trade (13k2e5): which lines vary, Soul Tether", [
  ["A · Today, like Exiled Exchange 2: every line fixed, the unique searched by its name", win(topBar(), item({ ...SOUL_TETHER, stats: soulTetherA }, true), price(SOUL_TETHER_PRICE), foot())],
  ["B · Each line with a number a filter, unchecked, at its value, without bar nor range", win(topBar(), item({ ...SOUL_TETHER, stats: soulTetherB }, true), price(SOUL_TETHER_PRICE), foot())],
  ["C · Chosen · The rolls of poe2scout: as in the advanced copy, the Vaal modifier as in B", win(topBar(), item({ ...SOUL_TETHER, filters: "3 filters", stats: soulTetherC }, true), price(SOUL_TETHER_PRICE), foot())],
  ["C · French: +20 d'Intelligence, which poe2scout doesn't hold, as in B", win(topBar(), item({ ...SOUL_TETHER_FR, stats: soulTetherCFr }, true), price({ ...SOUL_TETHER_PRICE, offers: "3981 offres", trend: "Stable sur les 7 derniers jours" }, "fr"), foot("poe2scout il y a 12min · GGG trade il y a 2min"))],
]]);
