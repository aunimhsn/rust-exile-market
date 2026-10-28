// Sample items, prices and offers, used by several sections.
// The Item group of a rare, magic or normal item (13h): its base, then its item level, sockets, quality and
// required level when it has them, each `{ value, check }`: the item level first, which every item has, so
// that the rows stay in place from one item to the next. The sockets a base usually has are shown unchecked;
// a normal item without quality has no quality filter. The required level of a rare for leveling is searched
// with a maximum: the offers a character of that level can use.
const itemFilters = ({ base, level, sockets, quality, required }) => [
  { group: "Item", min: true },
  { mod: `Base: ${base}`, check: true },
  level && { mod: `Item level: ${level.value}`, check: level.check, min: level.value },
  sockets && { mod: `Sockets: ${sockets.value}`, check: sockets.check, min: sockets.value },
  quality && { mod: `Quality: ${quality.value}%`, check: quality.check, min: quality.value },
  required && { mod: `Required level: ${required.value}`, check: false, min: required.value, max: required.value, lower: true },
].filter(Boolean);
// Sample items.
const HH = { rarity: "unique", name: "Headhunter", base: "Heavy Belt", cls: "Belts", tags: ["Corrupted", "ilvl 80"], req: "Requires Level 50", statsLabel: "Stats (6)", filters: "3 filters" };
const HH_STATS = [
  { group: "Implicit", min: true },
  { mod: "26% increased Stun Threshold", check: false, min: 26, range: "20–30", share: .6 },
  { mod: "Has 2 Charm Slots", check: false, min: 2, range: "1–3", share: .5 },
  { group: "Unique" },
  { mod: "+55 to maximum Life", check: true, min: 55, range: "40–60", share: .75 },
  { mod: "+34 to Strength", check: true, min: 34, range: "20–40", share: .7 },
  { mod: "+22 to Dexterity", check: true, min: 22, range: "20–40", share: .1 },
  { mod: "When you kill a Rare monster, you gain its Modifiers for 60 seconds" },
  { note: "Corrupted offers only, like this item." },
];
const HH_PRICE = { values: ["12.4", "123", "9K"], offers: "23 offers", trend: "+4% over the last 3 days" };
const HH_OFFERS = { count: "10 of 13 online", columns: ["Price", "Listed"], rows: [
  { v: "6", k: "ex", when: "2h ago", sus: true }, { v: "1", k: "chaos", when: "5d ago", sus: true }, { v: "8", k: "div", when: "3h ago" },
  { v: "10", k: "div", when: "3h ago" }, { v: "11", k: "div", when: "40min ago", away: true }, { v: "12", k: "div", when: "1d ago" }] };
const BOW = { rarity: "rare", name: "Storm Bite", base: "Gemini Bow", cls: "Bows", tags: ["ilvl 81", "Q 20%", "2 sockets"], props: [["Physical DPS", "245"], ["Elemental DPS", "44"]], req: "Requires Level 65 · 145 Dex", statsLabel: "Stats (9)" };
// Rare, magic and jewels: no market price, so the price is estimated from similar offers.
// Minimums are prefilled at 90 % of the copied values, to find comparable items.
const BOW_FILTERS = [
  // A bow usually has two sockets; its quality isn't exceptional.
  ...itemFilters({ base: "Gemini Bow", level: { value: 81, check: false }, sockets: { value: 2, check: false } }),
  { group: "Weapon" },
  { mod: "Physical DPS: 245", check: true, min: 220 },
  { mod: "Elemental DPS: 44", check: false, min: 39 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "152% increased Physical Damage", check: false, min: 137, tier: 2, range: "135–154", share: .8 },
  { mod: "Adds 21 to 38 Physical Damage", check: false, tier: 4, range: "18–42", share: .45 },
  { mod: "Adds 3 to 71 Lightning Damage", check: false, tier: 3, range: "2–80", share: .6 },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+3 to Level of all Projectile Skills", check: true, min: 3, tier: 1, range: "3–3", share: 1 },
  { mod: "14% increased Attack Speed", check: false, min: 13, tier: 2, range: "11–15", share: .82 },
  { mod: "+312 to Accuracy Rating", check: true, min: 281, tier: 4, range: "296–344", share: .5 },
];
const BOW_RARE = { ...BOW, tags: [...BOW.tags], statsLabel: "Stats (6)", filters: "4 filters" };
const RARE_ESTIMATE_BEFORE = { lines: ["No market price for rare items.", "Search similar offers to estimate it."] };
const RING = { rarity: "rare", name: "Storm Spiral", base: "Sapphire Ring", cls: "Rings", tags: ["ilvl 77"], req: "Requires Level 54", statsLabel: "Stats (6)", filters: "5 filters" };
const RING_FILTERS = [
  { group: "Item", min: true },
  { mod: "Base: Sapphire Ring", check: true },
  { mod: "Item level: 77", check: false, min: 77 },
  { group: "Pseudo" },
  { mod: "+83% total Elemental Resistance", check: true, min: 75 },
  { mod: "+65 total maximum Life", check: true, min: 59 },
  { mod: "+12 total to Dexterity", check: true, min: 11 },
  { group: "Implicit" },
  { mod: "+22% to Cold Resistance", check: false, min: 20, range: "20–30", share: .2 },
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "+65 to maximum Life", check: false, min: 59, tier: 5, range: "60–69", share: .4 },
  { mod: "+21 to maximum Energy Shield", check: true, min: 19, tier: 6, range: "17–23", share: .3 },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+38% to Fire Resistance", check: false, min: 34, tier: 2, range: "36–40", share: .82 },
  { mod: "+23% to Lightning Resistance", check: false, min: 21, tier: 4, range: "21–25", share: .44 },
  { mod: "+12 to Dexterity", check: false, min: 11, tier: 6, range: "9–12", share: .25 },
];
const MAGIC = { rarity: "magic", name: "Glimmering Sapphire Ring of the Lynx", cls: "Rings", tags: ["ilvl 68"], req: "Requires Level 44", statsLabel: "Stats (3)", filters: "2 filters" };
// Exceptional base: a normal item whose value is its base, its sockets and its item level.
const STAFF = { rarity: "normal", name: "Exceptional Chiming Staff", cls: "Staves", tags: ["3 sockets", "ilvl 82"], summary: "Grants Skill: Level 18 Sigil of Power", req: "Requires Level 78 · 137 Int", statsLabel: "Stats (0)", filters: "3 filters" };
// Exceptional by its sockets: a staff usually has two. The trade site doesn't know its name.
const STAFF_FILTERS = itemFilters({ base: "Chiming Staff", level: { value: 82, check: true }, sockets: { value: 3, check: true } });
// Every modifier of a jewel has one tier only: no tier, its bar over its own range (13k1).
const JEWEL = { rarity: "rare", name: "Brood Ember", base: "Ruby", cls: "Jewels", tags: ["ilvl 81"], statsLabel: "Stats (4)", filters: "5 filters" };
const JEWEL_FILTERS = [
  { group: "Item", min: true },
  { mod: "Base: Ruby", check: true },
  { mod: "Item level: 81", check: false, min: 81 },
  { group: "Prefixes", fill: "2 of 2" },
  { mod: "12% increased Projectile Damage", check: true, min: 10, single: true, range: "11–13", share: .5 },
  { mod: "10% increased Damage with Bows", check: true, min: 9, single: true, range: "8–11", share: .67 },
  { group: "Suffixes", fill: "2 of 2" },
  { mod: "14% increased Critical Damage Bonus", check: true, min: 12, single: true, range: "13–15", share: .5 },
  { mod: "8% increased Attack Speed", check: true, min: 7, single: true, range: "7–8", share: 1 },
];
