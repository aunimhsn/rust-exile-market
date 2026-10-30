// Section Roll bar (13j2): the bar of a rare or magic modifier over every tier of its base, from the
// lowest roll of its worst tier to the highest of tier 1, so that a full bar means close to the best
// possible; one bar per line of a hybrid, each line having its own field (decided with the owner on
// 2026-10-28, against a segment per tier and the zone of its tier shaded). Also a line fixed in its tier
// but not over the tiers. Only with the advanced copy (13j4). The tiers are those of the modifier
// tables (RePoE), on the real bases.
// The bar of a line over every tier: `tiers`, the best first, each the range of each value of the line
// ([low, high], two for "Adds # to #"): where `values` fall between the lowest roll of the worst tier and
// the highest of tier 1, the average of its values, like the bar of a unique.
function overTiers(values, tiers) {
  const shares = values.map((value, index) => {
    const low = Math.min(...tiers.map(tier => tier[index][0]));
    const high = Math.max(...tiers.map(tier => tier[index][1]));
    return high > low ? Math.min(1, Math.max(0, (value - low) / (high - low))) : 1;
  });
  return { share: shares.reduce((sum, share) => sum + share, 0) / shares.length };
}
// One value per line: its tiers as the ranges of one value.
const one = tiers => tiers.map(range => [range]);
// The rolls of the tooltip of the range field, "10 to 214", and their tiers under them, "T13–T1".
const rollsOver = tiers => ({ bounds: `${tiers[tiers.length - 1][0]} to ${tiers[0][1]}`, boundTiers: [tiers.length, 1] });
// The tiers of Ancestral Mail, a body armour of strength and dexterity.
const BODY_LIFE = [[200, 214], [190, 199], [175, 189], [150, 174], [120, 149], [100, 119], [85, 99], [70, 84], [60, 69], [40, 59], [30, 39], [20, 29], [10, 19]];
const BODY_ARMOUR = [[150, 170], [128, 149], [103, 127], [86, 102], [72, 85], [47, 71], [17, 46], [9, 16]];
const BODY_EVASION = [[142, 161], [120, 141], [95, 119], [79, 94], [65, 78], [42, 64], [11, 41], [6, 10]];
const BODY_ARMOUR_EVASION = [[101, 110], [92, 100], [80, 91], [68, 79], [56, 67], [43, 55], [27, 42], [15, 26]];
const RESISTANCE = [[41, 45], [36, 40], [31, 35], [26, 30], [21, 25], [16, 20], [11, 15], [6, 10]];
const MAIL = { rarity: "rare", name: "Doom Shell", base: "Ancestral Mail", cls: "Body Armours", tags: ["ilvl 82"], props: [["Armour", "512"], ["Evasion", "468"]], req: "Requires Level 65 · 86 Str · 86 Dex", filters: "3 filters", statsLabel: "Stats (5)" };
// `hover` opens the tooltip of the range field of a line, by its index among the rows.
const mailStats = (hover = {}) => [
  { group: "Prefixes", fill: "3 of 3", min: true },
  { mod: "+86 to maximum Life", check: true, min: 77, tier: 7, range: "85–99", over: overTiers([86], one(BODY_LIFE)), ...rollsOver(BODY_LIFE) },
  { mod: "62% increased Armour and Evasion", check: false, min: 56, tier: 5, range: "56–67", over: overTiers([62], one(BODY_ARMOUR_EVASION)), ...rollsOver(BODY_ARMOUR_EVASION) },
  { mod: "+98 to Armour", check: false, min: 88, over: overTiers([98], one(BODY_ARMOUR)), ...rollsOver(BODY_ARMOUR) },
  { mod: "+88 to Evasion Rating", check: false, min: 79, over: overTiers([88], one(BODY_EVASION)), ...rollsOver(BODY_EVASION) },
  { rollOnly: true, tier: 4, range: "86–102 · 79–94" },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "+41% to Cold Resistance", check: true, min: 37, tier: 1, range: "41–45", over: overTiers([41], one(RESISTANCE)), ...rollsOver(RESISTANCE) },
  { mod: "+23% to Lightning Resistance", check: true, min: 21, tier: 5, range: "21–25", over: overTiers([23], one(RESISTANCE)), ...rollsOver(RESISTANCE) },
].map((row, index) => hover[index] ? { ...row, hover: hover[index] } : row);
// The tiers of Gemini Bow: two values for added damage, and a line fixed in each tier but not over them.
const BOW_PHYSICAL = [[170, 179], [155, 169], [135, 154], [110, 134], [85, 109], [65, 84], [50, 64], [40, 49]];
const BOW_ADDED_PHYSICAL = [[[26, 39], [44, 66]], [[21, 31], [36, 53]], [[16, 24], [28, 42]], [[13, 20], [23, 35]], [[10, 15], [18, 26]], [[8, 12], [14, 21]], [[6, 9], [11, 16]], [[4, 6], [7, 11]], [[1, 2], [4, 5]]];
const BOW_ADDED_LIGHTNING = [[[1, 12], [202, 234]], [[1, 10], [157, 196]], [[1, 8], [111, 152]], [[1, 6], [85, 107]], [[1, 4], [63, 82]], [[1, 3], [55, 60]], [[1, 2], [36, 52]], [[1, 2], [20, 30]], [[1, 1], [13, 19]], [[1, 1], [4, 6]]];
const BOW_PROJECTILE_LEVEL = [[4, 4], [3, 3], [2, 2], [1, 1]];
const BOW_ATTACK_SPEED = [[17, 19], [14, 16], [11, 13], [8, 10], [5, 7]];
const BOW_TIERS = { ...BOW, tags: [...BOW.tags], statsLabel: "Stats (5)", filters: "2 filters" };
const bowTierStats = [
  { group: "Prefixes", fill: "3 of 3", min: true },
  { mod: "152% increased Physical Damage", check: false, min: 137, tier: 3, range: "135–154", over: overTiers([152], one(BOW_PHYSICAL)), ...rollsOver(BOW_PHYSICAL) },
  { mod: "Adds 21 to 38 Physical Damage", check: false, tier: 2, range: "21–31 · 36–53", over: overTiers([21, 38], BOW_ADDED_PHYSICAL) },
  { mod: "Adds 3 to 140 Lightning Damage", check: false, tier: 3, range: "1–8 · 111–152", over: overTiers([3, 140], BOW_ADDED_LIGHTNING) },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "+3 to Level of all Projectile Skills", check: true, min: 3, tier: 2, tierOnly: true, over: overTiers([3], one(BOW_PROJECTILE_LEVEL)), ...rollsOver(BOW_PROJECTILE_LEVEL) },
  { mod: "14% increased Attack Speed", check: false, min: 13, tier: 2, range: "14–16", over: overTiers([14], one(BOW_ATTACK_SPEED)), ...rollsOver(BOW_ATTACK_SPEED) },
];
SECTIONS.push(["Roll bar over every tier (13j2): the bar of a rare or magic modifier", [
  ["Advanced copy: a hybrid modifier, a tier 1", win(topBar(), item({ ...MAIL, stats: mailStats() }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["A line of a hybrid hovered: its rolls over every tier", win(topBar(), item({ ...MAIL, stats: mailStats({ 3: "min" }) }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Added damage (two values), a line fixed in its tier", win(topBar(), item({ ...BOW_TIERS, stats: bowTierStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
