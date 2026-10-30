// Section Item filters (13h): the sockets, the quality and the required level, in the Item group.
// An exceptional base is searched by what makes it exceptional, its sockets above those its base usually has
// or its quality above 20 %: the trade site doesn't know its name. On a rare, magic item or jewel, the sockets
// are checked above those of the base or on a corrupted item, the quality above 20 % except on a rare (of a
// charm from 10 %, of a flask from 20 %, checked above), and the required level of a rare for leveling (75 or under, item level too) is unchecked, searched as a maximum.
const TARGE = { rarity: "normal", name: "Exceptional Mammoth Targe", cls: "Shields", props: [["Armour", "139"], ["Evasion", "126"], ["Block", "25%"]], summary: "Grants Skill: Raise Shield", req: "Requires Level 65 · 50 Str · 50 Dex", tags: ["Q 24%", "ilvl 82"], statsLabel: "Stats (0)", filters: "3 filters" };
const TARGE_FILTERS = [
  // A shield usually has one socket: this one has none, its quality makes it exceptional.
  ...itemFilters({ base: "Mammoth Targe", level: { value: 82, check: true }, quality: { value: 24, check: true } }),
];
const CROWN = { rarity: "normal", name: "Superior Divine Crown", cls: "Helmets", props: [["Armour", "174"], ["Energy Shield", "60"]], req: "Requires Level 75 · 67 Str · 67 Int", tags: ["Q 9%", "1 socket", "ilvl 81"], statsLabel: "Stats (0)", filters: "3 filters" };
const CROWN_FILTERS = [
  // A normal item: its sockets checked, its quality unchecked under 20 %.
  ...itemFilters({ base: "Divine Crown", level: { value: 81, check: true }, sockets: { value: 1, check: true }, quality: { value: 9, check: false } }),
];
const JACKET = { rarity: "rare", name: "Hate Shell", base: "Waxed Jacket", cls: "Body Armours", props: [["Evasion", "120"]], req: "Requires Level 40 · 30 Dex", tags: ["Q 23%", "3 sockets", "ilvl 45"], statsLabel: "Stats (2)", filters: "4 filters" };
// A rare for leveling: three sockets where a body armour usually has two, an exceptional quality left
// unchecked on a rare, and its required level as a maximum. `hover` opens the tooltip of that field.
const jacketFilters = (hover = false) => [
  ...itemFilters({ base: "Waxed Jacket", level: { value: 45, check: false }, sockets: { value: 3, check: true }, quality: { value: 23, check: false }, required: { value: 40 } })
    .map(row => hover && row.mod?.startsWith("Required") ? { ...row, hover: "max" } : row),
  { group: "Pseudo" },
  { mod: "+20% total Elemental Resistance", check: true, min: 18 },
  { mod: "+80 total maximum Life", check: true, min: 72 },
  { group: "Explicit" },
  { mod: "+80 to maximum Life", check: false, min: 72 },
  { mod: "+20% to Fire Resistance", check: false, min: 18 },
];
const MITTS = { rarity: "magic", name: "Rogue's Vaal Mitts of the Fox", cls: "Gloves", props: [["Evasion", "96"]], req: "Requires Level 70 · 88 Dex", tags: ["Q 23%", "1 socket", "ilvl 72"], statsLabel: "Stats (2)", filters: "3 filters" };
const MITTS_FILTERS = [
  // An exceptional quality checked on a magic item; gloves usually have one socket.
  ...itemFilters({ base: "Vaal Mitts", level: { value: 72, check: false }, sockets: { value: 1, check: false }, quality: { value: 23, check: true } }),
  { group: "Pseudo" },
  { mod: "+20 total to Dexterity", check: true, min: 18 },
  { group: "Prefixes", fill: "1 of 1" },
  { mod: "+46 to Evasion Rating", check: true, min: 41, tier: 4, range: "36–60", noBar: true },
  { group: "Suffixes", fill: "1 of 1" },
  { mod: "+20 to Dexterity", check: false, min: 18, tier: 5, range: "17–20", noBar: true },
];
const CHARM = { rarity: "magic", name: "Ruby Charm of the Fox", cls: "Charms", summary: "Lasts 4 Seconds · Consumes 40 of 80 Charges on use", req: "Requires Level 40", tags: ["Q 12%", "ilvl 60"], statsLabel: "Stats (1)", filters: "3 filters" };
const CHARM_FILTERS = [
  // A charm: its quality checked from 10 % (a flask: from 20 %, checked above), like Exiled Exchange 2.
  ...itemFilters({ base: "Ruby Charm", level: { value: 60, check: false }, quality: { value: 12, check: true } }),
  { group: "Suffixes", fill: "1 of 1" },
  { mod: "+28% to Fire Resistance", check: true, min: 25, tier: 3, range: "26–30", noBar: true },
];
const TREAD = { rarity: "rare", name: "Gale Tread", base: "Lattice Sandals", cls: "Boots", props: [["Energy Shield", "64"]], req: "Requires Level 52 · 41 Int", tags: ["Corrupted", "1 socket", "ilvl 80"], statsLabel: "Stats (2)", filters: "4 filters" };
const TREAD_FILTERS = [
  // Corrupted: no socket can be added any more, so the one it has counts.
  ...itemFilters({ base: "Lattice Sandals", level: { value: 80, check: false }, sockets: { value: 1, check: true } }),
  { group: "Prefixes", fill: "1 of 3 · 2 open" },
  { mod: "30% increased Movement Speed", check: true, min: 27, tier: 1, tierOnly: true, noBar: true },
  { group: "Suffixes", fill: "1 of 3 · 2 open" },
  { mod: "+38% to Cold Resistance", check: true, min: 34, tier: 2, range: "36–40", noBar: true },
  { note: "Corrupted offers only, like this item." },
];
SECTIONS.push(["Item filters (13h): the sockets, the quality and the required level", [
  ["Exceptional base by its sockets", win(topBar(), item({ ...STAFF, stats: STAFF_FILTERS }, true), foot())],
  ["Exceptional base by its quality", win(topBar(), item({ ...TARGE, stats: TARGE_FILTERS }, true), foot())],
  ["Normal base: quality under 20 %", win(topBar(), item({ ...CROWN, stats: CROWN_FILTERS }, true), foot())],
  ["Rare for leveling: sockets above the base, required level", win(topBar(), item({ ...JACKET, stats: jacketFilters() }, true), foot())],
  ["The same, the required level hovered", win(topBar(), item({ ...JACKET, stats: jacketFilters(true) }, true), foot())],
  ["Magic item: exceptional quality, usual sockets", win(topBar(), item({ ...MITTS, stats: MITTS_FILTERS }, true), foot())],
  ["Magic charm: its quality from 10 %", win(topBar(), item({ ...CHARM, stats: CHARM_FILTERS }, true), foot())],
  ["Corrupted rare: its sockets checked", win(topBar(), item({ ...TREAD, stats: TREAD_FILTERS }, true), foot())],
  ["Rare bow: usual sockets, quality 20 %", win(topBar(), item({ ...BOW_RARE, stats: BOW_FILTERS }, true), foot())],
]]);
