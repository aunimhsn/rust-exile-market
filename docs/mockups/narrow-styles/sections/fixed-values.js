// Section Fixed values (13k3b2): a line of a prefix or suffix whose value is fixed in its tier
// (`+2 to Level of all Minion Skills`) shows otherwise in the two copies, as the application does today
// (second code review of 13k3b2, 2026-10-30). The advanced copy writes no range for a fixed value, so
// nothing follows its tier; the in-game trade copy gets the range of its tier from the modifier tables,
// written as one number: "T2 2" under the ordinary `of the Despot` of an amulet (T2 of 3), and "2" under
// the `of the Despot` of the Genesis tree on a belt, whose tier the game doesn't write, which also loses
// its range field there (fixed in its tier, as `Reinforcing` stays within 19–22) while the advanced copy
// keeps one. A modifier of one tier only does the same (`+1 to Level of all Projectile Skills` of a quiver:
// nothing in the advanced copy, "1" from the in-game trade). One open decision, two options, each the
// same in both copies:
// - A, nothing under a fixed value, as the advanced copy and the game write it: the number would repeat
//   the line; the tier stays for a modifier of several tiers ("T2"), a range of one value other than the
//   value stays too (`29(30)%` of a sanctified item, "T2 30");
// - B, the value under it in both copies, the advanced copy taking it from the tables: every line of a
//   prefix or suffix the same shape, but a number already on the line.
// Either way, the line of the Genesis tree has no range field in both copies: fixed in its tier.
// Chosen by the owner on 2026-10-30: A, also for a fixed value the advanced copy writes with its range
// (`50(50)% reduced Amount Recovered` of a flask); the other fixed values of the sheet drawn so ("T1",
// not "T1 4").

// A rare amulet, the owner's `of the Despot` without its desecration.
const FIXED_AMULET = { rarity: "rare", name: "Doom Idol", base: "Gold Amulet", cls: "Amulets",
  tags: ["ilvl 79"], req: "Requires Level 56", statsLabel: "Stats (6)", filters: "4 filters" };
// `minionLevels`: the row of `+2 to Level of all Minion Skills`, after its bar over the three tiers.
const fixedAmulet = (minionLevels, note) => [
  ...itemFilters({ base: "Gold Amulet", level: { value: 79, check: false } }),
  { group: "Pseudo" },
  { mod: "+33% total Elemental Resistance", check: true, min: 30 },
  { group: "Prefixes", fill: "1 of 3" },
  { mod: "+75 to maximum Energy Shield", check: true, min: 68, tier: 2, range: "71–79", over: { share: .83 } },
  { group: "Suffixes", fill: "2 of 3" },
  { mod: "+33% to Lightning Resistance", check: false, min: 30, tier: 3, range: "31–35", over: { share: .69 } },
  { mod: "+2 to Level of all Minion Skills", check: true, min: 2, tier: 2, over: { share: .5 }, ...minionLevels },
  note && FIXED_NOTE,
].filter(Boolean);

// A rare belt grown by the Genesis tree: `Reinforcing` (19–22, the third of six tiers) and `of the Despot`
// (+2, the best of two), whose tiers the game doesn't write. The in-game trade copy sorts its lines by
// stat: the Fire Resistance first.
const FIXED_BELT = { rarity: "rare", name: "Grip Clasp", base: "Heavy Belt", cls: "Belts",
  tags: ["ilvl 79"], req: "Requires Level 52", statsLabel: "Stats (8)", filters: "5 filters" };
const fixedBelt = (minionLevels, note) => {
  const levels = { mod: "+2 to Level of all Minion Skills", check: true, ...minionLevels };
  const fire = { mod: "+38% to Fire Resistance", check: false, min: 34, tier: 2, range: "36–40", over: { share: .82 } };
  return [
    ...itemFilters({ base: "Heavy Belt", level: { value: 79, check: false } }),
    { group: "Pseudo" },
    { mod: "+38% total Elemental Resistance", check: true, min: 34 },
    { mod: "+85 total maximum Life", check: true, min: 77 },
    { group: "Prefixes", fill: "2 of 3" },
    { mod: "Minions have 21% increased maximum Life", check: true, min: 19, single: true, range: "19–22", share: .67 },
    { mod: "+85 to maximum Life", check: false, min: 77, tier: 4, range: "85–99", over: { share: .46 } },
    { group: "Suffixes", fill: "2 of 3" },
    ...(note ? [fire, levels] : [levels, fire]),
    note && FIXED_NOTE,
  ].filter(Boolean);
};

const FIXED_NOTE = { note: "The tiers are found from the values." };
const fixedWindow = (it, stats) => win(topBar(), item({ ...it, stats }, true), price(RARE_ESTIMATE_BEFORE), foot());

// Two sections, so that the windows of each fit the width of the page: the options first, then today put
// above them.
SECTIONS.unshift(["Fixed values (13k3b2), the decision: A or B, each the same in both copies", [
  ["A · Chosen · Nothing under a fixed value, both copies: \"T2\" alone", fixedWindow(FIXED_AMULET, fixedAmulet({ tierOnly: true }, true))],
  ["A · Belt, both copies: no field, nothing under", fixedWindow(FIXED_BELT, fixedBelt({}, true))],
  ["B · The value under it, both copies: \"T2 2\"", fixedWindow(FIXED_AMULET, fixedAmulet({ range: "2" }, true))],
  ["B · Belt, both copies: no field, \"2\" under it", fixedWindow(FIXED_BELT, fixedBelt({ single: true, range: "2", noBar: true }, true))],
]]);
SECTIONS.unshift(["Fixed values (13k3b2), today: +2 to Level of all Minion Skills differs between the copies", [
  ["Amulet, advanced copy: nothing after \"T2\"", fixedWindow(FIXED_AMULET, fixedAmulet({ tierOnly: true }, false))],
  ["Amulet, in-game trade: \"T2 2\"", fixedWindow(FIXED_AMULET, fixedAmulet({ range: "2" }, true))],
  ["Belt of the Genesis tree, advanced copy: a field, nothing under", fixedWindow(FIXED_BELT, fixedBelt({ min: 2 }, false))],
  ["Belt of the Genesis tree, in-game trade: no field, \"2\" under it", fixedWindow(FIXED_BELT, fixedBelt({ single: true, range: "2", noBar: true }, true))],
]]);
