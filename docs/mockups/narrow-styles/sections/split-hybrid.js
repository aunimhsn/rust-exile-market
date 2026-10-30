// Section Hybrid of a split (13k2e4b3b): the part of a hybrid in a summed line (section Summed lines in
// doubt) says "Hybrid" after its range, its tooltip naming the other line of the hybrid; that other line
// gets its tier, the range of its tier and its bar over the tiers of the hybrid, "Hybrid" after the range,
// its tooltip naming the part of the summed line ("With 75–79 of 248% increased Physical Damage": the range
// written as under a tier, without the unit of the line, owner's choice on 2026-10-30, since a line of two
// numbers can't write one). Two cases the tables hold, not drawn yet; the tiers and ranges are those of the
// modifier tables on these bases:
// - D, a hybrid prefix added up with a pure suffix: the Stun Threshold of a shield, boots or a body armour
//   is a pure suffix and a line of six hybrid prefixes (the armour, evasion or energy shield of the item
//   with it); the Accuracy of a weapon is a pure prefix and a line of the suffix with the Light Radius. The
//   line goes into Explicit, like the 37% of rarity of decision C, with the other line of its hybrid: "Prefix"
//   and "Hybrid" both apply to that part;
// - E, a hybrid of three lines: the % of Armour and Evasion of an Armour/Evasion body armour is a pure prefix
//   and a line of the hybrid with the flat Armour and Evasion; the other lines are then two, which the
//   tooltip of the summed line names both. The in-game trade writes the flat lines before the % (the owner's
//   body armour, section Summed lines in doubt), so the tier of the other lines goes under the last of them,
//   with their ranges, as under any hybrid.
// Chosen by the owner on 2026-10-30: D1 ("Hybrid prefix", French "Préfixe hybride": one word that reads
// easily, against "Prefix · Hybrid"), E (both other lines named with "and").

// A rare shield, its prefixes and suffixes full: +310 to Stun Threshold is the hybrid T2 (64–94, with 35%
// increased Armour and Evasion, 33–38) and the pure suffix T2 (207–253); read alone, 310 is above the
// highest pure roll (304), and 35% can only be the hybrid T2. With room for a modifier more, a reading of
// three modifiers fits too, and both lines stay without tier. As the application shows it (test
// `a_hybrid_prefix_added_up_with_a_pure_suffix_says_hybrid_prefix_on_both_its_lines`).
const SPLIT_SHIELD = { rarity: "rare", name: "Dread Bulwark", base: "Avian Targe", cls: "Shields",
  tags: ["ilvl 80"], props: [["Armour", "254"], ["Evasion Rating", "231"]], req: "Requires Level 70 · 62 Str · 62 Dex",
  statsLabel: "Stats (8)", filters: "6 filters" };
const SPLIT_SHIELD_FR = { ...SPLIT_SHIELD, name: "Rempart de l'effroi", base: "Targe aviaire", cls: "Boucliers",
  tags: ["niv. 80"], props: [["Armure", "254"], ["Score d'Évasion", "231"]], req: "Niveau requis 70 · 62 For · 62 Dex",
  statsLabel: "Statistiques (8)", filters: "6 filtres",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
// The shield with the words of a variant: `hybrid` after the range of the hybrid, on both of its lines.
const splitShield = (hybrid, { fr = false, hover = "summed" } = {}) => {
  const text = fr
    ? { percent: "35% d'Augmentation de l'Armure et de l'Évasion", stun: "+310 de Seuil d'Étourdissement",
        life: "+132 de Vie maximale", block: "26% d'Augmentation des chances de Bloquer",
        fire: "+38% à la Résistance au feu", cold: "+32% à la Résistance au froid", suffix: "Suffixe",
        pseudo: ["Pseudo", "+70% aux Résistances Élémentaires au total", "+132 de Vie maximale au total"],
        added: "Additionné au préfixe sur cette ligne", with: "Avec", of: "des",
        groups: ["Objet", "Préfixes", "Suffixes", "Explicite"], fill: n => `${n} sur 3`, base: "Base : Targe aviaire",
        level: "Niveau de l'objet : 80", note: "Les paliers sont trouvés d'après les valeurs." }
    : { percent: "35% increased Armour and Evasion", stun: "+310 to Stun Threshold",
        life: "+132 to maximum Life", block: "26% increased Block chance",
        fire: "+38% to Fire Resistance", cold: "+32% to Cold Resistance", suffix: "Suffix",
        pseudo: ["Pseudo", "+70% total Elemental Resistance", "+132 total maximum Life"],
        added: "Added up with the prefix on this line", with: "With", of: "of",
        groups: ["Item", "Prefixes", "Suffixes", "Explicit"], fill: n => `${n} of 3`, base: "Base: Avian Targe",
        level: "Item level: 80", note: SUMMED_NOTE.note };
  const word = (tip, open) => ({ ...hybrid, tip, hover: open });
  return [
    { group: text.groups[0], min: true },
    { mod: text.base, check: true },
    { mod: text.level, check: false, min: 80 },
    { group: text.pseudo[0] },
    { mod: text.pseudo[1], check: true, min: 63 },
    { mod: text.pseudo[2], check: true, min: 119 },
    { group: text.groups[1], fill: text.fill(3) },
    { mod: text.life, check: false, min: 119, tier: 3, range: "120–149", over: { share: .68 } },
    { mod: text.block, check: true, min: 23, tier: 1, range: "25–30", over: { share: .73 } },
    { group: text.groups[2], fill: text.fill(3) },
    { mod: text.fire, check: false, min: 34, tier: 2, range: "36–40", over: { share: .82 } },
    { mod: text.cold, check: false, min: 29, tier: 3, range: "31–35", over: { share: .67 } },
    { group: text.groups[3] },
    { mod: text.percent, check: true, min: 32, tier: 2, range: "33–38", over: { share: .81 },
      after: word(`${text.with} 64–94 ${text.of} ${text.stun}`, hover === "other") },
    { mod: text.stun, check: true, min: 279,
      affixes: [[2, "64–94", word(`${text.with} ${text.percent}`, hover === "summed")], [2, "207–253", { label: text.suffix, tip: text.added }]] },
    { note: text.note },
  ];
};

// A rare body armour, its prefixes full: 115% increased Armour and Evasion is the pure prefix T4 (68–79)
// and the hybrid T1 (39–42, with +50 to Armour, 42–52, and +41 to Evasion Rating, 40–50); read alone, 115%
// is above the highest pure roll (110), and +50 and +41 are in no tier of the hybrid of the flat Armour and
// Evasion, of two lines. As the application shows it (test
// `a_hybrid_of_three_lines_has_its_tier_under_the_last_other_line`).
const SPLIT_ARMOUR = { rarity: "rare", name: "Storm Shell", base: "Ancestral Mail", cls: "Body Armours",
  tags: ["ilvl 80"], props: [["Armour", "512"], ["Evasion Rating", "468"]], req: "Requires Level 70 · 86 Str · 86 Dex",
  statsLabel: "Stats (10)", filters: "7 filters" };
const splitArmour = hover => [
  ...itemFilters({ base: "Ancestral Mail", level: { value: 80, check: false } }),
  { group: "Pseudo" },
  { mod: "+38% total Elemental Resistance", check: true, min: 34 },
  { mod: "+238 total maximum Life", check: true, min: 214 },
  { mod: "+29 total to Strength", check: true, min: 26 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "+180 to maximum Life", check: false, min: 162, tier: 3, range: "175–189", over: { share: .83 } },
  { mod: "+50 to Armour", check: true, min: 45, over: { share: .96 } },
  { mod: "+41 to Evasion Rating", check: true, min: 37, over: { share: .81 } },
  { rollOnly: true, tier: 1, range: "42–52 · 40–50",
    after: { label: "Hybrid", tip: "With 39–42 of 115% increased Armour and Evasion", hover: hover === "other" } },
  { mod: "115% increased Armour and Evasion", check: true, min: 104,
    affixes: [[4, "68–79"], [1, "39–42", { label: "Hybrid", tip: "With +50 to Armour and +41 to Evasion Rating", hover: hover === "summed" }]] },
  { group: "Suffixes", fill: "2 of 3" },
  { mod: "+38% to Fire Resistance", check: false, min: 34, tier: 2, range: "36–40", over: { share: .82 } },
  { mod: "+29 to Strength", check: false, min: 26, tier: 2, range: "28–30", over: { share: .86 } },
  SUMMED_NOTE,
];

// The bow of decision A1, the tooltip of its other line open.
const splitBow = summedBowA([[1, 2], "155–179"]).map(row => row.after ? { ...row, after: { ...row.after, hover: true, end: true } } : row);

// Loaded after the section Summed lines in doubt, whose samples it takes, and put before it at the top.
SECTIONS.unshift(["Hybrid of a split (13k2e4b3b), decision D: a hybrid prefix added up with a pure suffix", [
  ["The bow of A1: the tooltip of its other line, the range without %", win(topBar(), item({ ...SUMMED_BOW, stats: splitBow }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["D1 · Chosen · \"Hybrid prefix\" on both lines of the hybrid", win(topBar(), item({ ...SPLIT_SHIELD, stats: splitShield({ label: "Hybrid prefix" }) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["D1, French · \"Préfixe hybride\"", win(topBar(), item({ ...SPLIT_SHIELD_FR, stats: splitShield({ label: "Préfixe hybride" }, { fr: true, hover: "other" }) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["D2 · \"Prefix · Hybrid\": the affix, then the hybrid with its tooltip", win(topBar(), item({ ...SPLIT_SHIELD, stats: splitShield({ label: "Prefix · Hybrid" }) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]], ["Hybrid of a split (13k2e4b3b), decision E: a hybrid of three lines", [
  ["E · Chosen · The tooltip of the summed line names both other lines, with \"and\"", win(topBar(), item({ ...SPLIT_ARMOUR, stats: splitArmour("summed") }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["E · The tooltip of the other lines, under the last of them", win(topBar(), item({ ...SPLIT_ARMOUR, stats: splitArmour("other") }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
