// Section Filters checked on a rare (17e3f): too few offers on a rare, the rare jewellery above all, since every
// modifier is checked at 90 % of its value, four or five at once past four modifiers.
// The rule chosen by the owner on 2026-11-10 (option 1), the default of Exiled Exchange 2: on a rare item or jewel,
// every modifier starts unchecked (implicit, prefixes, suffixes); the pseudo totals and the DPS stay checked,
// with the base type and the filters of the Item group as before. A minimum checked keeps its 90 %. Magic items
// keep every modifier checked. When no stat is checked (no pseudo total, no DPS, no modifier: a jewel, or a
// player who unchecked them), the search still runs, but its offers give no estimate: the price block asks for a
// modifier, before the search and after it, so that the price of the worst items of the base never reads as the
// price of the item; when none can be checked (an unidentified item), it says nothing more.
// Turned down on 2026-11-10: the modifiers of tier 1 or 2 checked, two at most by their roll over their tiers,
// then in a group "at least 2 of them" (a `count` group of the trade site): a tier is the game's measure of a
// roll, not what the market pays for, nor what the player needs (owner).

// `rows` with the checks of today, or of the rule: the modifiers unchecked.
const MODIFIER_GROUPS = /^(Implicit|Prefixes|Suffixes|Explicit|Implicite|Préfixes|Explicite)$/;
const ruled = (rows, withRule) => {
  let group = "";
  return rows.map(row => {
    if (row.group) group = row.group;
    return withRule && MODIFIER_GROUPS.test(group) && row.check !== undefined ? { ...row, check: false } : row;
  });
};
// A stat checked: a filter outside the Item group.
const statChecked = rows => {
  let group = "";
  return rows.some(row => {
    if (row.group) group = row.group;
    return row.check && !/^(Item|Objet)$/.test(group);
  });
};
const NO_STAT = {
  en: { lines: ["No market price for rare items.", "Check a modifier to estimate this item."] },
  fr: { lines: ["Pas de prix de marché pour les objets rares.", "Cochez un modificateur pour estimer cet objet."] },
};
const BEFORE_FR = { lines: ["Pas de prix de marché pour les objets rares.", "Cherchez des offres similaires pour l'estimer."] };
// The window of `it` with its stats `rows` unfolded (`withRule`), its price block, and `offersBlock` when given.
const ruleWin = (it, rows, withRule, offersBlock) => {
  const fr = it.fr;
  const stats = ruled(rows, withRule);
  const count = stats.filter(row => row.check).length;
  const block = !statChecked(stats) ? NO_STAT[fr ? "fr" : "en"] : fr ? BEFORE_FR : RARE_ESTIMATE_BEFORE;
  return win(topBar(), item({ ...it, filters: fr ? `${count} filtre${count > 1 ? "s" : ""}` : `${count} filter${count > 1 ? "s" : ""}`, stats }, true),
    price(block, fr ? "fr" : "en"), ...(offersBlock ? [offersBlock] : []),
    foot(fr ? "poe2scout il y a 12min · GGG trade il y a 2min" : undefined));
};

// The owner's ring, twice corrupted (src/item/test-items/en-owner-rare-ring-catalyst.txt): its value is in its
// modifiers, its pseudo totals weak; searched by them only.
const RULE_RING = { rarity: "rare", name: "Wrath Circle", base: "Prismatic Ring", cls: "Rings",
  tags: [state("Twice Corrupted", "twice", false), "Q 20%", "ilvl 82"], req: "Requires Level 63", statsLabel: "Stats (8)" };
const ruleRingRows = [
  { group: "Item", min: true },
  { mod: "Base: Prismatic Ring", check: true },
  { mod: "Item level: 82", check: false, min: 82 },
  { ...catalyst("Cold", 20), check: false, min: 20 },
  { group: "Pseudo" },
  { mod: "+30% total Elemental Resistance", check: true, min: 27 },
  { mod: "+42 total to Dexterity", check: true, min: 37 },
  { group: "Enchant" },
  { mod: "+14 to Dexterity", check: false },
  { group: "Implicit" },
  { mod: "+10% to all Elemental Resistances", check: false, min: 9, range: "7–10", share: 1 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Adds 1 to 65 Lightning damage to Attacks", check: true, min: 30, tier: 1, range: "1–4 · 60–71", over: { share: .87 } },
  { mod: "+125 to Evasion Rating", check: true, min: 112, tier: 4, range: "108–141", over: { share: .52 }, origin: "desecrated" },
  { mod: "Adds 8 to 20 Physical Damage to Attacks", check: true, min: 13, tier: 3, range: "7–11 · 14–20", over: { share: .51 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "16% increased Rarity of Items found", check: true, min: 14, tier: 1, range: "15–18", over: { share: .83 } },
  { mod: "+28 to Dexterity", check: false, min: 25, tier: 2, range: "28–30", over: { share: .82 } },
  { mod: "67% increased Mana Regeneration Rate", check: true, min: 60, tier: 1, range: "60–69", over: { share: .97 } },
];

// The owner's Breach ring (en-owner-rare-ring-breach.txt): three pseudo totals.
const RULE_BREACH = { rarity: "rare", name: "Glyph Circle", base: "Breach Ring", cls: "Rings", tags: ["ilvl 77"],
  req: "Requires Level 44", statsLabel: "Stats (7)" };
const ruleBreachRows = [
  { group: "Item", min: true },
  { mod: "Base: Breach Ring", check: true },
  { mod: "Item level: 77", check: false, min: 77 },
  { group: "Pseudo" },
  { mod: "+18% total Chaos Resistance", check: true, min: 16 },
  { mod: "+76 total maximum Mana", check: true, min: 68 },
  { mod: "+20 total to Dexterity", check: true, min: 18 },
  { group: "Implicit" },
  { mod: "+20% to Maximum Quality" },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Minions have 21% increased maximum Life", check: true, min: 19, tier: 3, range: "19–22", over: { share: .61 } },
  { mod: "+76 to maximum Mana", check: false, min: 68, tier: 7, range: "65–79", over: { share: .4 } },
  { mod: "+129 to Evasion Rating", check: true, min: 116, tier: 4, range: "108–141", over: { share: .54 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+20 to Dexterity", check: false, min: 18, tier: 5, range: "17–20", over: { share: .55 } },
  { mod: "Gain 35 Life per enemy killed", check: true, min: 31, tier: 2, range: "29–40", over: { share: .63 } },
  { mod: "+18% to Chaos Resistance", check: false, min: 16, tier: 3, range: "16–19", over: { share: .61 } },
];

// The owner's amulet (en-owner-rare-amulet-catalyst.txt): two pseudo totals.
const ruleAmuletRows = [
  { group: "Item", min: true },
  { mod: "Base: Gloam Amulet", check: true },
  { mod: "Item level: 79", check: false, min: 79 },
  { ...catalyst("Mana", 40), check: false, min: 40 },
  { group: "Pseudo" },
  { mod: "+52% total Elemental Resistance", check: true, min: 46 },
  { mod: "+22% total Chaos Resistance", check: true, min: 19 },
  { group: "Enchant" },
  { mod: "Allocates Arcane Intensity", check: false },
  { group: "Implicit" },
  { mod: "-1 Prefix Modifier allowed" },
  { mod: "+1 Suffix Modifier allowed" },
  { group: "Prefixes", fill: "2 of 2", min: true },
  { mod: "36% increased maximum Energy Shield", check: true, min: 32, tier: 3, range: "33–38", over: { share: .65 }, origin: "desecrated" },
  { mod: "+75 to maximum Energy Shield", check: true, min: 67, tier: 2, range: "71–79", over: { share: .83 }, origin: "desecrated" },
  { group: "Suffixes", fill: "4 of 4" },
  { mod: "+22% to Chaos Resistance", check: false, min: 19, tier: 2, range: "20–23", over: { share: .8 }, origin: "desecrated" },
  { mod: "+33% to Lightning Resistance", check: false, min: 29, tier: 3, range: "31–35", over: { share: .68 }, origin: "desecrated" },
  { mod: "+19% to Cold Resistance", check: false, min: 17, tier: 6, range: "16–20", over: { share: .34 }, origin: "desecrated" },
  { mod: "+2 to Level of all Minion Skills", check: true, min: 2, tier: 2, tierOnly: true, over: { share: .5 }, origin: "desecrated" },
];

// The owner's mace for leveling (en-owner-rare-two-hand-mace-fractured.txt): its DPS.
const ruleMaceRows = [
  { group: "Item", min: true },
  { mod: "Base: Leaden Greathammer", check: true },
  { mod: "Item level: 39", check: false, min: 39 },
  { group: "Weapon" },
  { mod: "Physical DPS: 161", check: true, min: 144 },
  { mod: "Elemental DPS: 4", check: false, min: 3 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Adds 12 to 26 Physical Damage", check: false, tier: 6, range: "11–17 · 20–30", over: { share: .3 } },
  { mod: "Adds 2 to 6 Cold Damage", check: false, tier: 10, range: "2–3 · 4–6", over: { share: .02 } },
  { mod: "68% increased Physical Damage", check: false, min: 61, tier: 6, range: "65–84", over: { share: .3 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "Gain 3 Mana per enemy killed", check: true, min: 3, tier: 8, range: "2–3", over: { share: .05 }, origin: "fractured" },
  { mod: "+12% to Critical Damage Bonus", check: true, min: 11, tier: 5, range: "12–13", over: { share: .3 } },
  { mod: "+3 to Level of all Melee Skills", check: true, min: 3, tier: 3, tierOnly: true, over: { share: .67 } },
];

// A jewel: no pseudo total, no DPS; every modifier unchecked, nothing narrows the search but its base.
const ruleJewelRows = [
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

// An unidentified magic jewel: no line, nothing to check; the price block says nothing more (code review of
// 17e3f, owner's choice on 2026-11-10), rather than ask for a modifier the player can't check.
const UNIDENTIFIED_JEWEL = { rarity: "magic", name: "Emerald", cls: "Jewels", tags: ["Unidentified", "ilvl 75"], statsLabel: "Stats (0)" };
// The jewel in French.
const JEWEL_FR = { ...JEWEL, fr: true, name: "Braise de la couvée", base: "Rubis", cls: "Joyaux", tags: ["niv. 81"], statsLabel: "Statistiques (4)",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const ruleJewelRowsFr = [
  { group: "Objet", min: true },
  { mod: "Base : Rubis", check: true },
  { mod: "Niveau de l'objet : 81", check: false, min: 81 },
  { group: "Préfixes", fill: "2 sur 2" },
  { mod: "12% d'Augmentation des Dégâts des Projectiles", check: true, min: 10, single: true, range: "11–13", share: .5 },
  { mod: "10% d'Augmentation des Dégâts avec les Arcs", check: true, min: 9, single: true, range: "8–11", share: .67 },
  { group: "Suffixes", fill: "2 sur 2" },
  { mod: "14% d'Augmentation du Bonus de Dégâts critiques", check: true, min: 12, single: true, range: "13–15", share: .5 },
  { mod: "8% d'Augmentation de la Vitesse d'Attaque", check: true, min: 7, single: true, range: "7–8", share: 1 },
];
// The jewel searched with nothing but its base: offers of any Ruby, no estimate.
const JEWEL_OFFERS = { count: "10 of 2,481 online", columns: ["Price", "Listed"], rows: [
  { v: "1", k: "ex", when: "10min ago" }, { v: "1", k: "ex", when: "2h ago" }, { v: "1", k: "ex", when: "1d ago", away: true },
  { v: "2", k: "ex", when: "3h ago" }, { v: "2", k: "ex", when: "5h ago" }, { v: "3", k: "ex", when: "20min ago" }] };
// The player checked the attack speed: the second line asks for a search again.
const jewelOneChecked = rows => ruled(rows, true).map(row => row.mod === "8% increased Attack Speed" ? { ...row, check: true } : row);

SECTIONS.unshift(["Filters checked on a rare (17e3f): option 1, every modifier unchecked; no estimate without a checked stat", [
  ["Your ring · Today · every modifier checked", ruleWin(RULE_RING, ruleRingRows, false)],
  ["Your ring · Rule · its pseudo totals only", ruleWin(RULE_RING, ruleRingRows, true)],
  ["A jewel · Today", ruleWin(JEWEL, ruleJewelRows, false)],
  ["A jewel · Rule · no stat checked: the price block asks for a modifier", ruleWin(JEWEL, ruleJewelRows, true, offers({ state: "idle" }))],
  ["A jewel · Rule · searched anyway: its offers, no estimate", ruleWin(JEWEL, ruleJewelRows, true, offers(JEWEL_OFFERS))],
  ["A jewel · Rule · the player checks a modifier: back to the usual text", ruleWin(JEWEL, jewelOneChecked(ruleJewelRows), false, offers({ state: "idle" }))],
  ["A jewel · Rule · en français", ruleWin(JEWEL_FR, ruleJewelRowsFr, true, offers({ state: "idle" }, "fr"))],
  ["An unidentified jewel · Rule · nothing to check: no second line", win(topBar(),
    item({ ...UNIDENTIFIED_JEWEL, filters: "1 filter", stats: [{ group: "Item", min: true }, { mod: "Base: Emerald", check: true }, { mod: "Item level: 75", check: false, min: 75 }] }, true),
    price({ lines: ["No market price for magic items."] }), offers({ state: "idle" }), foot())],
  ["Your Breach ring · Today", ruleWin(RULE_BREACH, ruleBreachRows, false)],
  ["Your Breach ring · Rule · its three pseudo totals", ruleWin(RULE_BREACH, ruleBreachRows, true)],
  ["Your amulet · Today", ruleWin(AMULET_DESECRATED, ruleAmuletRows, false)],
  ["Your amulet · Rule · its two pseudo totals", ruleWin(AMULET_DESECRATED, ruleAmuletRows, true)],
  ["Your mace for leveling · Today", ruleWin(MACE_FRACTURED, ruleMaceRows, false)],
  ["Your mace for leveling · Rule · its DPS", ruleWin(MACE_FRACTURED, ruleMaceRows, true)],
]]);
