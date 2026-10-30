// Section Summed lines in doubt (13k2e4b): a line of an in-game trade copy that adds up two modifiers of
// the same stat (section In-game trade, the 258% of the bow), when its values leave a doubt. Three open
// decisions, each with its options; the tiers and ranges are those of the modifier tables on these bases:
// - A, the pure part in two neighbouring tiers, the hybrid sure: the 248% of the owner's bow is a pure
//   170–173 with the hybrid at 75–78, or a pure 169 with the hybrid at 79; the +195 of Accuracy makes the
//   hybrid T1 (75–79 · 175–200), the pure part T1 (170–179) or T2 (155–169);
// - B, two splits, the other line of the hybrid a sum too (fourth code review of 13k2e4a): the 217% and
//   +200 of a bow with a prefix open are the hybrid T1 and a pure T3 (2 prefixes), or a pure T1, the
//   hybrid T5 (45–54 · 98–123) and a pure Accuracy T8 (61–84) (3 prefixes);
// - C, a prefix and a suffix of the same stat: the 37% of rarity of a tiara is the prefix T1 (16–19) and
//   the suffix T1 (15–18), the prefix first; "Prefix" and "Suffix" one under the other in Explicit already
//   mean one or the other (section In-game trade), hence the tooltip of "Suffix" when it means both.
// Chosen by the owner on 2026-10-29: A1 ("T1/T2", a slash since a dash reads as a span of tiers), B1, C1
// with the tooltip (against C2: a group holds only what its title says, and Explicit already holds the lines
// of no one group).
// A split never shows the amount of each part, which the sum doesn't tell (owner, 2026-10-29): only the
// tier of each and the range of that tier, without bar; "T1/T2" only when one part is sure and the other
// in two neighbouring tiers; when both parts are in doubt (33% of rarity: T1 and T2, or T2 and T1), the line
// has no tier (B1).

// The note under the stats.
const SUMMED_NOTE = { note: "The tiers are found from the values." };

// The owner's bow, its summed line at 248% instead of 258%.
const SUMMED_BOW = { rarity: "rare", name: "Storm Volley", base: "Warmonger Bow", cls: "Bows",
  tags: [state("Mirrored", "mirrored", false), state("Fractured", "fractured", false), "Q 30%", "3 sockets", "ilvl 82"],
  props: [["Physical DPS", "700"]], req: "Requires Level 77 · 163 Dex", statsLabel: "Stats (10)", filters: "8 filters" };
const summedBowTop = [
  ...itemFilters({ base: "Warmonger Bow", level: { value: 82, check: false }, sockets: { value: 3, check: true }, quality: { value: 30, check: false } }),
  { group: "Weapon" },
  { mod: "Physical DPS: 700", check: true, min: 630 },
  { group: "Runes" },
  { mod: "+1 Suffix Modifier allowed", check: false },
  { mod: "40% increased Physical Damage", check: false },
  { mod: "Bonded: 40% increased effect of Fully Broken Armour", check: false },
];
const summedBowSuffixes = [
  { group: "Suffixes", fill: "4 of 4" },
  { mod: "+5% to Critical Hit Chance", check: true, min: 5, tier: 1, range: "4.41–5", over: { share: 1 }, origin: "fractured" },
  { mod: "+25% to Critical Damage Bonus", check: true, min: 23, tier: 1, range: "23–25", over: { share: 1 } },
  { mod: "+4 to Level of all Projectile Skills", check: true, min: 4, tier: 1, tierOnly: true, over: { share: 1 } },
  { mod: "+181% Surpassing chance to fire an additional Arrow", check: true, min: 163, tier: 1, range: "175–200", over: { share: .89 } },
  SUMMED_NOTE,
];
// The bow of decision A, with the pure part of its 248% as `pure`.
const summedBowA = pure => [
  ...summedBowTop,
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "248% increased Physical Damage", check: false, min: 223,
    affixes: [pure, [1, "75–79", { label: "Hybrid", tip: "With +195 to Accuracy Rating" }]] },
  { mod: "+195 to Accuracy Rating", check: true, min: 176, tier: 1, range: "175–200", over: { share: .97 },
    after: { label: "Hybrid", tip: "With 75–79 of 248% increased Physical Damage" } },
  { mod: "Adds 36 to 56 Physical Damage", check: false, tier: 1, range: "26–39 · 44–66", over: { share: .88 }, origin: "desecrated" },
  ...summedBowSuffixes,
];

// The bow of decision B: 217% and +200, a prefix open; both lines without tier in Explicit, as a line
// split two ways is today, `word` under each.
const TWO_SPLITS = "Two splits fit: a pure T3 135–154 with the hybrid T1 75–79 · 175–200, or a pure T1 170–179 with the hybrid T5 45–54 · 98–123 and a pure Accuracy T8 61–84.";
const summedBowB = word => [
  ...summedBowTop.map(row => row.mod === "Physical DPS: 700" ? { ...row, mod: "Physical DPS: 610", min: 549 } : row),
  { group: "Explicit" },
  { mod: "217% increased Physical Damage", check: false, min: 195, after: word && { ...word, hover: true } },
  { mod: "+200 to Accuracy Rating", check: true, min: 180, after: word },
  ...summedBowSuffixes,
];

// The owner's tiara with an open prefix, its rarity at 37%: a prefix and a suffix added up.
const SUMMED_TIARA = { rarity: "rare", name: "Chimeric Visage", base: "Ancestral Tiara", cls: "Helmets",
  tags: [state("Fractured", "fractured", false), "Q 20%", "2 sockets", "ilvl 82"], props: [["Energy Shield", "511"]],
  req: "Requires Level 80 · 115 Int", statsLabel: "Stats (11)", filters: "7 filters" };
const summedTiaraTop = [
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
const summedTiaraPrefixes = [
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "41% increased Energy Shield", check: true, min: 37, over: { share: .97 }, origin: "fractured" },
  { mod: "+34 to maximum Mana", check: false, min: 31, over: { share: .85 }, origin: "fractured" },
  { rollOnly: true, tier: 1, range: "39–42 · 33–39", origin: "fractured" },
  { mod: "98% increased Energy Shield", check: true, min: 88, tier: 1, range: "92–100", over: { share: .98 } },
];
const summedTiaraSuffixes = [
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+43% to Fire Resistance", check: false, min: 39, tier: 1, range: "41–45", over: { share: .95 } },
  { mod: "+40% to Cold Resistance", check: false, min: 36, tier: 2, range: "36–40", over: { share: .87 } },
];
// C1: in Explicit, "Prefix" then "Suffix", as a line whose affix stays unknown; the tooltip of "Suffix" tells
// they are added up, which the line whose affix stays unknown doesn't have.
const summedTiaraC1 = [
  ...summedTiaraTop,
  ...summedTiaraPrefixes,
  ...summedTiaraSuffixes,
  { group: "Explicit" },
  { mod: "37% increased Rarity of Items found", check: true, min: 33,
    affixes: [[1, "16–19", { label: "Prefix" }], [1, "15–18", { label: "Suffix", tip: "Added up with the prefix on this line", hover: true }]] },
  SUMMED_NOTE,
];
// C2: in Prefixes, its first part, the suffix part marked as the hybrid part of the bow is; the fill of the
// suffixes counts it.
const summedTiaraC2 = [
  ...summedTiaraTop,
  ...summedTiaraPrefixes,
  { mod: "37% increased Rarity of Items found", check: true, min: 33,
    affixes: [[1, "16–19"], [1, "15–18", { label: "Suffix", tip: "A suffix of the same stat, added up on this line", hover: true }]] },
  ...summedTiaraSuffixes,
  SUMMED_NOTE,
];

// The owner's body armour (inputs/13k2e4b-body-armour.txt, bought on 2026-10-29), copied from the in-game trade in
// French: 47% is the 23% of a triple hybrid (+23 Armour, +19 Evasion) and the 24% of a hybrid with Life, whose
// other line is a sum too (+93 = a pure 70 and its 23). A line that adds up two hybrids is beyond the splits
// of 13k2e4a (a pure modifier and another): the lines it can't read stay without tier, the others keep theirs.
// The copy sorts its lines by stat (the Strength before the cold resistance, against the advanced copy).
const SUMMED_ARMOUR = { rarity: "rare", name: "Tenue démoniaque", base: "Armure de lâche", cls: "Armures",
  tags: ["niv. 67"], props: [["Armure", "479"], ["Score d'Évasion", "434"]], req: "Niveau requis 65 · 67 For · 67 Dex",
  statsLabel: "Statistiques (8)", filters: "7 filtres",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const summedArmourStats = [
  { group: "Objet", min: true },
  { mod: "Base : Armure de lâche", check: true },
  { mod: "Niveau de l'objet : 67", check: false, min: 67 },
  { group: "Pseudo" },
  { mod: "+158 de Vie maximale au total", check: true, min: 142 },
  { mod: "+31% aux Résistances Élémentaires au total", check: true, min: 28 },
  { group: "Implicite" },
  { mod: "+65 de Vie maximale", check: false, min: 59, range: "60–80", share: .25 },
  { group: "Suffixes", fill: "3 sur 3" },
  { mod: "+26 de Force", check: true, min: 23, tier: 3, range: "25–27", over: { share: .8 } },
  { mod: "+31% à la Résistance au froid", check: false, min: 28, tier: 3, range: "31–35", over: { share: .74 } },
  { mod: "12.2 de Vie Régénérée par seconde", check: true, min: 11, tier: 6, range: "9.1–13", over: { share: .4 } },
  { group: "Explicite" },
  { mod: "+23 à l'Armure", check: true, min: 21 },
  { mod: "+19 au Score d'Évasion", check: true, min: 17 },
  { mod: "47% d'Augmentation de l'Armure et de l'Évasion", check: true, min: 42 },
  { mod: "+93 de Vie maximale", check: false, min: 84 },
  { note: "Les paliers sont trouvés d'après les valeurs." },
];

// One section per decision, so that the windows of each fit the width of the page.
SECTIONS.push(["Summed lines in doubt (13k2e4b), decision A: 248%, the pure part T1 or T2, the hybrid sure", [
  ["A1 · Chosen · Both tiers, one or the other, over the range of both: \"T1/T2 155–179\"", win(topBar(), item({ ...SUMMED_BOW, stats: summedBowA([[1, 2], "155–179"]) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["A2 · The worst tier, as a value in two tiers of one modifier: \"T2 155–169\"", win(topBar(), item({ ...SUMMED_BOW, stats: summedBowA([2, "155–169"]) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["A3 · The worst tier, \"or T1\" after its range, with a tooltip", win(topBar(), item({ ...SUMMED_BOW, stats: summedBowA([2, "155–169", { label: "or T1", tip: "248% is a pure 170–173 with the hybrid at 75–78, or a pure 169 with the hybrid at 79", hover: true }]) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
SECTIONS.push(["Summed lines in doubt (13k2e4b), decision B: a line split two ways, or beyond the splits found", [
  ["B1 · Chosen · 217% and +200 split two ways: both lines without tier, in Explicit, as today", win(topBar(), item({ ...SUMMED_BOW, props: [["Physical DPS", "610"]], statsLabel: "Stats (9)", stats: summedBowB(null) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["B2 · The same, \"Two splits\" under each line, with a tooltip", win(topBar(), item({ ...SUMMED_BOW, props: [["Physical DPS", "610"]], statsLabel: "Stats (9)", stats: summedBowB({ label: "Two splits", tip: TWO_SPLITS }) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["B · Your body armour (B1 or B2): the 4 prefix lines summed by two hybrids, without tier", win(topBar(), item({ ...SUMMED_ARMOUR, stats: summedArmourStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
SECTIONS.push(["Summed lines in doubt (13k2e4b), decision C: 37% of rarity, a prefix and a suffix", [
  ["C1 · Chosen · In Explicit, \"Prefix\" then \"Suffix\", the tooltip of \"Suffix\" saying they are added up", win(topBar(), item({ ...SUMMED_TIARA, stats: summedTiaraC1 }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["C2 · In Prefixes, the suffix part marked \"Suffix\"", win(topBar(), item({ ...SUMMED_TIARA, stats: summedTiaraC2 }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
