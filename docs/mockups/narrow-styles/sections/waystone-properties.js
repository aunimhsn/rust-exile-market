// Section Waystone properties (13k5c): the properties of a waystone, which make its price while the
// application reads none today: `Revives Available`, `Item Rarity`, `Pack Size`, `Monster Rarity`,
// `Monster Effectiveness`, `Waystone Drop Chance` (French `Résurrections disponibles`, `Rareté des objets`,
// `Taille des Groupes`, `Rareté des Monstres`, `Efficacité des Monstres`, `Chances de trouver des Pierres de
// téléportation`). The trade site searches each of them (`map_filters`, checked on 2026-10-31: `map_revives`,
// `map_iir`, `map_packsize`, `map_rare_monsters` now named "Monster Rarity", `map_magic_monsters` now named
// "Monster Effectiveness", `map_bonus`); the tier is in the base, already searched.
// Exiled Exchange 2 (commit cca3066) shows them unchecked, the value as minimum, and on a waystone unchecks
// and hides every modifier but the desecrated ones: a buyer pays for the bonuses, not for the monsters.
// Two open decisions:
// - where: 1, in the item block like the defences of an armour piece, and as filters in a group of their
//   own, like the DPS of a weapon (its "Weapon" group); 2, as filters only, the item block unchanged;
// - what is checked, the modifiers unchecked in each (today checked): A, nothing but the base, as Exiled
//   Exchange 2; B, the drop chance and the item rarity, what the players farm a map for; C, every property.
// A percentage is searched from 90 % of its value, rounded down, as a modifier; the revives from their number.
// Chosen by the owner on 2026-10-31: 1 and B; the other options kept below, for the record.
// The owner's addition: the tier of a waystone written short after its name, "Waystone · T12" (French
// "Pierre de téléportation · T12", the dot in gray as before the item class), in the base under the name, in
// bold: T1 to T5 in white, T6 to T10 in the yellow of the warning band, T11 to T15 in the orange of tier 1 of
// a modifier, T16 (a corrupted waystone raised one tier) in the corrupted red; without its item class after
// it, which only repeats its name (not "Waystone (Tier 12) · Waystones").
// In the item block, two names written short, their tooltip giving the name of the game (owner, 2026-10-31):
// "Revives" (French "Résurrections") and, in French only, "Pierres trouvées" for "Chances de trouver des
// Pierres de téléportation", too long; the other names as the game writes them. The group of the filters is
// "Waystone" (French "Pierre de téléportation"), the filters named in full.

// The base of a waystone of tier `tier`, its tier written short in its style.
const waystoneBase = (name, tier) => {
  const style = tier >= 16 ? " is-top" : tier >= 11 ? " is-high" : tier >= 6 ? " is-mid" : "";
  return `${name} <span class="cls">·</span> <span class="app-waystone-tier${style}">T${tier}</span>`;
};
const BASE_T12 = waystoneBase("Waystone", 12);
const BASE_T12_FR = waystoneBase("Pierre de téléportation", 12);
// The filter of the base (the owner's request on 2026-10-31): its text without the tier ("Base: Waystone"),
// a bar of the tier over 1 to 16 under it, like a modifier, the tier after the bar in its style ("T12"); the
// bar in the color of the tier (owner's choice, against white as every bar and that color softened: this bar
// tells a tier, not where a roll falls). Not chosen: the scale after the tier, "T12 1–16" ("a good player
// knows it"). The search still sends the name of the trade site, `Waystone (Tier 12)`.
const tierBand = tier => tier >= 16 ? "top" : tier >= 11 ? "high" : tier >= 6 ? "mid" : "low";
const tierRange = (tier, scale) => `<span class="app-waystone-tier is-${tierBand(tier)}">T${tier}</span>${scale ? " 1–16" : ""}`;
// `white`: the bar white as every bar, kept for the record.
const tierBaseRow = (label, tier, { scale = false, white = false } = {}) => ({ mod: label, check: true, single: true,
  range: tierRange(tier, scale), share: (tier - 1) / 15, cls: white ? "" : `app-tier-bar-${tierBand(tier)}` });
const withTierBar = (rows, label, tier, options) => rows.map(row => row.mod?.startsWith("Base") ? tierBaseRow(label, tier, options) : row);

const WAYSTONE_PROPS = { ...WAYSTONE, base: BASE_T12, noClass: true, props: [["Revives", "3", "Revives Available"], ["Item Rarity", "+11%"], ["Monster Rarity", "+25%"],
  ["Monster Effectiveness", "+13%"], ["Waystone Drop Chance", "+50%"]], filters: "1 filter" };
// `checked`: the properties checked, by their order on the item.
const waystonePropertyStats = (checked = []) => [
  ...withTierBar(itemFilters({ base: "Waystone", level: { value: 80, check: false } }), "Base: Waystone", 12),
  { group: "Waystone" },
  { mod: "Revives available: 3", check: checked.includes(0), min: 3 },
  { mod: "Item rarity: +11%", check: checked.includes(1), min: 9 },
  { mod: "Monster rarity: +25%", check: checked.includes(2), min: 22 },
  { mod: "Monster effectiveness: +13%", check: checked.includes(3), min: 11 },
  { mod: "Waystone drop chance: +50%", check: checked.includes(4), min: 45 },
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "Monsters have 91% increased Stun Buildup", check: false, min: 90, single: true, range: "90–100", share: .1 },
  { mod: "18% increased Monster Damage", check: false, min: 16, single: true, range: "15–19", share: .75 },
  { group: "Suffixes", fill: "1 of 3 · 2 open" },
  { mod: "Monster have 106% increased Elemental Ailment Application", check: false, min: 100, single: true, range: "100–119", share: .32 },
];
// The windows of four tiers, the filter of the base alone, for the color of the bar.
const tierWins = options => [[16, WAYSTONE_T16, 82], [12, WAYSTONE_PROPS, 80], [8, WAYSTONE_T8, 74], [5, WAYSTONE_T5, 70]]
  .map(([tier, it, level]) => win(topBar(), item({ ...it, statsLabel: "Stats (0)", filters: "1 filter",
    stats: [{ group: "Item", min: true }, tierBaseRow("Base: Waystone", tier, options), { mod: `Item level: ${level}`, check: false, min: level }] }, true), foot())).join("");
// The owner's window, its stats changed by `change`.
const propsWinWith = change => waystoneWin({ ...WAYSTONE_PROPS, filters: "3 filters" }, change(waystonePropertyStats([1, 4])));
const filterCount = checked => `${1 + checked.length} filter${checked.length ? "s" : ""}`;
const propsWin = (checked, withProps = true) => waystoneWin(
  { ...(withProps ? WAYSTONE_PROPS : WAYSTONE), filters: filterCount(checked) }, waystonePropertyStats(checked));

// The owner's French waystone, from the in-game trade, in B.
const WAYSTONE_PROPS_FR = { ...WAYSTONE_FR, base: BASE_T12_FR, noClass: true, filters: "3 filtres", props: [["Résurrections", "2", "Résurrections disponibles"], ["Rareté des objets", "+25%"],
  ["Taille des Groupes", "+9%"], ["Rareté des Monstres", "+23%"], ["Pierres trouvées", "+65%", "Chances de trouver des Pierres de téléportation"]] };
const waystonePropertyStatsFr = [
  { group: "Objet", min: true },
  tierBaseRow("Base : Pierre de téléportation", 12),
  { mod: "Niveau de l'objet : 78", check: false, min: 78 },
  { group: "Pierre de téléportation" },
  { mod: "Résurrections disponibles : 2", check: false, min: 2 },
  { mod: "Rareté des objets : +25%", check: true, min: 22 },
  { mod: "Taille des groupes : +9%", check: false, min: 8 },
  { mod: "Rareté des monstres : +23%", check: false, min: 20 },
  { mod: "Chances de trouver des pierres de téléportation : +65%", check: true, min: 58 },
  { group: "Préfixes", fill: "2 sur 3 · 1 libre" },
  { mod: "Les Monstres ont 13% d'Augmentation de la Vitesse dAttaque, d'incantation et de déplacement", check: false, min: 12, single: true, range: "10–15", share: .6 },
  { mod: "24% Davantage de Vie des Monstres", check: false, min: 22, single: true, range: "20–25", share: .8 },
  { group: "Suffixes", fill: "2 sur 3 · 1 libre" },
  { mod: "Les Monstres ont 101% d'Augmentation d'Application des Altérations Élémentaires", check: false, min: 100, single: true, range: "100–119", share: .05 },
  { mod: "+33% à la Résistance élémentaire des Monstres", check: false, min: 30, single: true, range: "30–34", share: .75 },
  { note: "Les paliers sont trouvés d'après les valeurs." },
];

// The four styles of the tier, folded: a magic waystone of tier 5, one of tier 8, the owner's of tier 12, a
// corrupted one of 16.
const WAYSTONE_T5 = { rarity: "magic", name: "Burning Waystone", base: waystoneBase("Waystone", 5), cls: "Waystones", noClass: true,
  props: [["Revives", "5", "Revives Available"], ["Item Rarity", "+4%"], ["Waystone Drop Chance", "+10%"]], tags: ["ilvl 70"],
  statsLabel: "Stats (1)", filters: "1 filter" };
// A normal waystone, named by its base without its tier.
const WAYSTONE_T3 = { rarity: "normal", name: "Waystone", base: waystoneBase("Waystone", 3), cls: "Waystones", noClass: true,
  props: [["Revives", "6", "Revives Available"]], tags: ["ilvl 66"], statsLabel: "Stats (0)", filters: "1 filter" };
const WAYSTONE_T8 = { ...WAYSTONE_T5, name: "Shocking Waystone", base: waystoneBase("Waystone", 8), tags: ["ilvl 74"] };
const WAYSTONE_T16 = { ...WAYSTONE_CORRUPTED, base: waystoneBase("Waystone", 16), noClass: true, props: [["Revives", "0", "Revives Available"],
  ["Item Rarity", "+38%"], ["Pack Size", "+12%"], ["Monster Rarity", "+41%"], ["Waystone Drop Chance", "+120%"]],
  filters: "3 filters" };
const folded = it => win(topBar(), item(it), price(it.rarity === "rare" ? RARE_ESTIMATE_BEFORE
  : { lines: [`No market price for ${it.rarity} items.`, "Search similar offers to estimate it."] }), foot());

SECTIONS.unshift(["Waystone properties (13k5c): 1 and B chosen, the tier after a dot in its style, without the item class, the bar of the tier on the filter of the base", [
  ["Chosen · 1 · B, in the item block and in a group of their own, the bar of the tier", propsWin([1, 4])],
  ["Chosen · folded: the properties stay in sight", folded({ ...WAYSTONE_PROPS, filters: filterCount([1, 4]) })],
  ["Chosen · French, in-game trade", win(topBar(), item({ ...WAYSTONE_PROPS_FR, stats: waystonePropertyStatsFr }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Chosen · the bar in the color of the tier, tiers 16, 12, 8 and 5", tierWins()],
  ["Chosen · tier 5: white", folded(WAYSTONE_T5)],
  ["Chosen · normal, tier 3: named without its tier", folded(WAYSTONE_T3)],
  ["Chosen · tier 8: the yellow of the warning band", folded(WAYSTONE_T8)],
  ["Chosen · tier 16, corrupted: the corrupted red", folded(WAYSTONE_T16)],
  ["Not chosen · the bar white", tierWins({ white: true })],
  ["Not chosen · the tier and its scale under the bar", propsWinWith(rows => withTierBar(rows, "Base: Waystone", 12, { scale: true }))],
  ["2 · B, as filters only", propsWin([1, 4], false)],
  ["1 · A, nothing but the base, as Exiled Exchange 2", propsWin([])],
  ["1 · C, every property", propsWin([0, 1, 2, 3, 4])],
  ["Today · no property, the modifiers checked", waystoneWin(WAYSTONE, waystoneStats)],
]]);
