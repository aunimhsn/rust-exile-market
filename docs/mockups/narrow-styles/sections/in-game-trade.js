// Section In-game trade (13k2e): an item copied from the trade of the game (another player's listing),
// without the `{ … }` headers: its lines marked `(implicit)`, `(fractured)`, `(desecrated)`, `(mutated)`,
// `(rune)` or `(enchant)`, no affix, tier nor range. From the owner's copies (inputs/13k2c-items.txt,
// 2026-10-28); the tiers and ranges below are those of the modifier tables on these bases.
// The application finds the affix and the tier of each line from its value, where Exiled Exchange 2 shows
// them without tier:
// - the lines of a hybrid modifier, one after the other with the same marker, read together (the 41% and
//   +34 fractured lines of the tiara: one modifier, since an item has one fractured modifier only);
// - a value in two tiers: the worst tier (specifications, Item detection);
// - an affix the prefixes or suffixes left full rule out (the rarity of the tiara rolls as a prefix too),
//   without tooltip: the room makes it certain (owner, 2026-10-29, step 13k2e3c2); otherwise the line in
//   Explicit, the tier and range of each affix one under the other, the affix after the range, its origin
//   on a line of its own under them (owner, 2026-10-29);
// - the suffixes allowed by a rune (`+1 Suffix Modifier allowed`): 4 of 4 on the bows;
// - a line that adds up two modifiers: the game writes one line for both (the 258% of the English bow, a
//   pure 170–179 and the 75–79 of the hybrid with the Accuracy, whose line stays apart), split into the tier
//   of each, one under the other, "Hybrid" after the range of the hybrid's part and of its other line, its
//   tooltip naming that line (owner, 2026-10-29, step 13k2e4; against "Hybrid" before the tier, which put the
//   tiers out of line, and "with Accuracy" after the range, too long in French).
// Decided (owner, 2026-10-29): a found tier looks like any other, one note under the stats saying the
// tiers are found from the values (against a dashed outline and "≈" before the tier). The checks follow
// the rule of the specifications (Stat filters), as the application does: every line checked, but those
// a pseudo total adds up and the damage a weapon's DPS counts (step 13k2e3c2).

// The note under the stats.
const DEDUCED_NOTE = { note: "The tiers are found from the values." };
const DEDUCED_NOTE_FR = { note: "Les paliers sont trouvés d'après les valeurs." };

// Fractured tiara (the owner's copy, English): a fractured hybrid, the rarity ruled a suffix.
const TIARA_TRADE = { rarity: "rare", name: "Chimeric Visage", base: "Ancestral Tiara", cls: "Helmets",
  tags: [state("Fractured", "fractured", false), "Q 20%", "2 sockets", "ilvl 82"], props: [["Energy Shield", "511"]],
  req: "Requires Level 80 · 115 Int", statsLabel: "Stats (12)", filters: "8 filters" };
// The filters of the tiara above its explicit modifiers, the same in its three windows: its quality of 20 %
// isn't one (only a quality above 20 % is), its two sockets are (checked, as the application does), and its
// pseudo totals count the rune of lightning.
const tiaraTop = [
  ...itemFilters({ base: "Ancestral Tiara", level: { value: 82, check: false }, sockets: { value: 2, check: true } }),
  { group: "Pseudo" },
  { mod: "+105% total Elemental Resistance", check: true, min: 95 },
  { mod: "+34 total maximum Mana", check: true, min: 31 },
  { group: "Enchant" },
  { mod: "Allocates Paragon", check: false },
  { group: "Runes" },
  { mod: "+22% to Lightning Resistance", check: false },
  { mod: "Raven-Touched", check: false },
  { mod: "Bonded: +20 to maximum Life", check: false },
  { mod: "Bonded: +20 to maximum Mana", check: false },
];
const tiaraTradeStats = [
  ...tiaraTop,
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "41% increased Energy Shield", check: true, min: 37, over: { share: .97 }, origin: "fractured" },
  { mod: "+34 to maximum Mana", check: false, min: 31, over: { share: .85 }, origin: "fractured" },
  { rollOnly: true, tier: 1, range: "39–42 · 33–39", origin: "fractured" },
  { mod: "98% increased Energy Shield", check: true, min: 88, tier: 1, range: "92–100", over: { share: .98 } },
  { mod: "+69 to maximum Energy Shield", check: true, min: 62, tier: 1, range: "61–73", over: { share: .94 }, origin: "desecrated" },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "18% increased Rarity of Items found", check: true, min: 16, tier: 1, range: "15–18", over: { share: 1 } },
  { mod: "+43% to Fire Resistance", check: false, min: 39, tier: 1, range: "41–45", over: { share: .95 } },
  { mod: "+40% to Cold Resistance", check: false, min: 36, tier: 2, range: "36–40", over: { share: .87 } },
  DEDUCED_NOTE,
];

// The same tiara with an open prefix (made from the owner's copy): the rarity could be a prefix or a
// suffix, so it goes into Explicit, with the tier and range of both, one under the other (owner,
// 2026-10-29, against "Prefix or suffix" and a tooltip).
const tiaraOpenStats = [
  ...tiaraTop,
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "41% increased Energy Shield", check: true, min: 37, over: { share: .97 }, origin: "fractured" },
  { mod: "+34 to maximum Mana", check: false, min: 31, over: { share: .85 }, origin: "fractured" },
  { rollOnly: true, tier: 1, range: "39–42 · 33–39", origin: "fractured" },
  { mod: "98% increased Energy Shield", check: true, min: 88, tier: 1, range: "92–100", over: { share: .98 } },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "+43% to Fire Resistance", check: false, min: 39, tier: 1, range: "41–45", over: { share: .95 } },
  { mod: "+40% to Cold Resistance", check: false, min: 36, tier: 2, range: "36–40", over: { share: .87 } },
  { group: "Explicit" },
  { mod: "18% increased Rarity of Items found", check: true, min: 16, affixes: [[1, "16–19", { label: "Prefix" }], [1, "15–18", { label: "Suffix" }]] },
  DEDUCED_NOTE,
];

// The same tiara with an open prefix and the rarity desecrated (made from the owner's copy, step 13k2e3c2):
// the origin of a line whose affix stays unknown on a line of its own under its tiers, since the line is
// desecrated whichever affix it is (owner, 2026-10-29: after the word of the last tier, it read as the
// origin of the suffix only; after the text of the line, it wrapped it more and was unlike every other).
const tiaraOpenDesecratedStats = [
  ...tiaraTop,
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "41% increased Energy Shield", check: true, min: 37, over: { share: .97 }, origin: "fractured" },
  { mod: "+34 to maximum Mana", check: false, min: 31, over: { share: .85 }, origin: "fractured" },
  { rollOnly: true, tier: 1, range: "39–42 · 33–39", origin: "fractured" },
  { mod: "98% increased Energy Shield", check: true, min: 88, tier: 1, range: "92–100", over: { share: .98 } },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "+43% to Fire Resistance", check: false, min: 39, tier: 1, range: "41–45", over: { share: .95 } },
  { mod: "+40% to Cold Resistance", check: false, min: 36, tier: 2, range: "36–40", over: { share: .87 } },
  { group: "Explicit" },
  { mod: "18% increased Rarity of Items found", check: true, min: 16, origin: "desecrated",
    affixes: [[1, "16–19", { label: "Prefix" }], [1, "15–18", { label: "Suffix" }]] },
  DEDUCED_NOTE,
];

// Mirrored and fractured bow (the owner's copy, English): 258% adds up two modifiers, 179 + 79.
const BOW_TRADE = { rarity: "rare", name: "Storm Volley", base: "Warmonger Bow", cls: "Bows",
  tags: [state("Mirrored", "mirrored", false), state("Fractured", "fractured", false), "Q 30%", "3 sockets", "ilvl 82"],
  props: [["Physical DPS", "720"]], req: "Requires Level 77 · 163 Dex", statsLabel: "Stats (10)", filters: "8 filters" };
const bowTradeStats = [
  ...itemFilters({ base: "Warmonger Bow", level: { value: 82, check: false }, sockets: { value: 3, check: true }, quality: { value: 30, check: false } }),
  { group: "Weapon" },
  { mod: "Physical DPS: 720", check: true, min: 648 },
  { group: "Runes" },
  { mod: "+1 Suffix Modifier allowed", check: false },
  { mod: "40% increased Physical Damage", check: false },
  { mod: "Bonded: 40% increased effect of Fully Broken Armour", check: false },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "258% increased Physical Damage", check: false, min: 232,
    affixes: [[1, "170–179"], [1, "75–79", { label: "Hybrid", tip: "With +195 to Accuracy Rating", hover: true }]] },
  { mod: "+195 to Accuracy Rating", check: true, min: 176, tier: 1, range: "175–200", over: { share: .97 },
    after: { label: "Hybrid", tip: "With 75–79 of 258% increased Physical Damage" } },
  { mod: "Adds 36 to 56 Physical Damage", check: false, tier: 1, range: "26–39 · 44–66", over: { share: .88 }, origin: "desecrated" },
  { group: "Suffixes", fill: "4 of 4" },
  { mod: "+5% to Critical Hit Chance", check: true, min: 5, tier: 1, range: "4.41–5", over: { share: 1 }, origin: "fractured" },
  { mod: "+25% to Critical Damage Bonus", check: true, min: 23, tier: 1, range: "23–25", over: { share: 1 } },
  { mod: "+4 to Level of all Projectile Skills", check: true, min: 4, tier: 1, tierOnly: true, over: { share: 1 } },
  { mod: "+181% Surpassing chance to fire an additional Arrow", check: true, min: 163, tier: 1, range: "175–200", over: { share: .89 } },
  DEDUCED_NOTE,
];

// Mirrored and fractured bow (the owner's copy, French): a desecrated hybrid, its two lines read together.
const BOW_TRADE_FR = { rarity: "rare", name: "Guide afflictif", base: "Arc de belliciste", cls: "Arcs",
  tags: [state("Reflété", "mirrored", false, { fr: true }), state("Fissuré", "fractured", false, { fr: true }), "Q 30%", "3 châsses", "niv. 82"],
  props: [["DPS physique", "625"]], req: "Niveau requis 77 · 163 Dex", statsLabel: "Statistiques (13)", filters: "8 filtres",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const bowTradeStatsFr = [
  { group: "Objet", min: true },
  { mod: "Base : Arc de belliciste", check: true },
  { mod: "Niveau de l'objet : 82", check: false, min: 82 },
  { mod: "Châsses : 3", check: true, min: 3 },
  { mod: "Qualité : 30%", check: false, min: 30 },
  { group: "Arme" },
  { mod: "DPS physique : 625", check: true, min: 562 },
  { group: "Runes" },
  { mod: "+1 Modificateur suffixe autorisé", check: false },
  { mod: "Les Attaques à l'arc tirent une Flèche supplémentaire", check: false },
  { mod: "50% d'Augmentation des Dégâts d'Attaque contre les Ennemis Rares ou Uniques", check: false },
  { mod: "Lié : 20% d'Augmentation de la Vitesse des Projectiles", check: false },
  { mod: "Lié : +1 au Niveau de toutes les Aptitudes d'Attaque", check: false },
  { group: "Préfixes", fill: "3 sur 3" },
  { mod: "172% d'Augmentation des Dégâts Physiques", check: false, min: 155, tier: 1, range: "170–179", over: { share: .95 } },
  { mod: "Ajoute 27 à 61 Dégâts Physiques", check: false, tier: 1, range: "26–39 · 44–66", over: { share: .8 } },
  { mod: "79% d'Augmentation des Dégâts Physiques", check: false, min: 71, over: { share: 1 }, origin: "desecrated", originLabel: "Profané" },
  { mod: "+193 au Score de Précision", check: true, min: 174, over: { share: .96 }, origin: "desecrated", originLabel: "Profané" },
  { rollOnly: true, tier: 1, range: "75–79 · 175–200", origin: "desecrated", originLabel: "Profané" },
  { group: "Suffixes", fill: "4 sur 4" },
  { mod: "+5% aux Chances de Touche critique", check: true, min: 5, tier: 1, range: "4.41–5", over: { share: 1 }, origin: "fractured", originLabel: "Fissuré" },
  { mod: "+25% au Bonus de Dégâts Critiques", check: true, min: 23, tier: 1, range: "23–25", over: { share: 1 } },
  { mod: "+4 au Niveau de toutes les Aptitudes à Projectiles", check: true, min: 4, tier: 1, tierOnly: true, over: { share: 1 } },
  { mod: "+200% de chances Cumulatives de tirer une Flèche supplémentaire", check: true, min: 180, tier: 1, range: "175–200", over: { share: 1 } },
  DEDUCED_NOTE_FR,
];

SECTIONS.push(["In-game trade (13k2e): copies without headers, tiers found from the values", [
  ["New (13k2e3c2) · The same tiara, the rarity desecrated: its origin on a line of its own", win(topBar(), item({ ...TIARA_TRADE, statsLabel: "Stats (11)", filters: "7 filters", stats: tiaraOpenDesecratedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Fractured tiara: a fractured hybrid, the rarity a suffix", win(topBar(), item({ ...TIARA_TRADE, stats: tiaraTradeStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["The same tiara with an open prefix: the rarity in Explicit", win(topBar(), item({ ...TIARA_TRADE, statsLabel: "Stats (11)", filters: "7 filters", stats: tiaraOpenStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Bow: 258% adds up two modifiers", win(topBar(), item({ ...BOW_TRADE, stats: bowTradeStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Bow, French: a desecrated hybrid on two lines", win(topBar(), item({ ...BOW_TRADE_FR, stats: bowTradeStatsFr }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
