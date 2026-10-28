// Section Endgame modifiers (13k1): the modifiers tied to a mechanic of the endgame, from the items the
// owner copied on 2026-10-28 (English and French). Open decision, a group of tabs, its windows first:
// - Catalyst quality: the quality of a ring or an amulet given by a catalyst, with its kind.
// Decided with the owner on 2026-10-28:
// - the origin of a desecrated, fractured or crafted modifier written after its range;
// - the green of the evasion for the desecrated modifiers;
// - a desecrated modifier not revealed yet, a line without filter on a green band, the item getting a
//   filter of its number of unrevealed modifiers and its other filters unchecked, like Exiled Exchange 2;
// - a modifier of one tier only (every modifier of a jewel, of a lord of the Abyss, of a waystone, of a
//   tablet, of the Breach), whose T1 means "the only one", without tier, its bar over its own range,
//   which the copy gives;
// - the lines of a Time-Lost jewel without their first words, a tag "Small" or "Notable" after their
//   range.
// A sanctified item has values outside the range of their tier (`+74(80-89) to maximum Mana`), shown as
// they are. The bars over every tier are approximate here.

// The quality of a catalyst, in words or short, `fr` for a French item; `plain` for the tooltip.
const catalyst = (kind, value, fr) => ({
  mod: fr
    ? `<span class="app-cat-full">Qualité (modificateurs de ${kind}) : ${value} %</span><span class="app-cat-short">Qualité de ${kind.toLowerCase()} : ${value} %</span>`
    : `<span class="app-cat-full">Quality (${kind} modifiers): ${value}%</span><span class="app-cat-short">${kind} quality: ${value}%</span>`,
  plain: fr ? `Qualité (modificateurs de ${kind}) : ${value} %` : `Quality (${kind} modifiers): ${value}%`,
});

// Desecrated amulet: every explicit modifier desecrated, the quality of a catalyst, an anointment.
const AMULET_DESECRATED = { rarity: "rare", name: "Doom Idol", base: "Gloam Amulet", cls: "Amulets", tags: ["Corrupted", "Q 40%", "ilvl 79"], req: "Requires Level 56", statsLabel: "Stats (9)", filters: "6 filters" };
const amuletDesecratedStats = [
  { group: "Item", min: true },
  { mod: "Base: Gloam Amulet", check: true },
  { mod: "Item level: 79", check: false, min: 79 },
  { ...catalyst("Mana", 40), check: false, min: 40 },
  { group: "Pseudo" },
  { mod: "+52% total Elemental Resistance", check: true, min: 46 },
  { group: "Enchant" },
  { mod: "Allocates Arcane Intensity", check: true },
  { group: "Implicit" },
  { mod: "-1 Prefix Modifier allowed", check: false },
  { mod: "+1 Suffix Modifier allowed", check: false },
  { group: "Prefixes", fill: "2 of 2", min: true },
  { mod: "36% increased maximum Energy Shield", check: true, min: 32, tier: 3, range: "33–38", over: { share: .62 }, origin: "desecrated" },
  { mod: "+75 to maximum Energy Shield", check: true, min: 67, tier: 2, range: "71–79", over: { share: .8 }, origin: "desecrated" },
  { group: "Suffixes", fill: "4 of 4" },
  { mod: "+22% to Chaos Resistance", check: true, min: 19, tier: 2, range: "20–23", over: { share: .8 }, origin: "desecrated" },
  { mod: "+33% to Lightning Resistance", check: false, min: 29, tier: 3, range: "31–35", over: { share: .68 }, origin: "desecrated" },
  { mod: "+19% to Cold Resistance", check: false, min: 17, tier: 6, range: "16–20", over: { share: .34 }, origin: "desecrated" },
  { mod: "+2 to Level of all Minion Skills", check: true, min: 2, tier: 2, range: "2–2", over: { share: .5 }, origin: "desecrated" },
];

// Fractured mace: one fractured suffix.
const MACE_FRACTURED = { rarity: "rare", name: "Blight Crack", base: "Leaden Greathammer", cls: "Two Hand Maces", tags: ["Fractured", "ilvl 39"], props: [["Physical DPS", "161"], ["Elemental DPS", "4"]], req: "Requires Level 33 · 60 Str", statsLabel: "Stats (6)", filters: "4 filters" };
const maceFracturedStats = [
  { group: "Item", min: true },
  { mod: "Base: Leaden Greathammer", check: true },
  { mod: "Item level: 39", check: false, min: 39 },
  { group: "Weapon" },
  { mod: "Physical DPS: 161", check: true, min: 144 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Adds 12 to 26 Physical Damage", check: false, tier: 6, range: "11–17 · 20–30", over: { share: .3 } },
  { mod: "Adds 2 to 6 Cold Damage", check: false, tier: 10, range: "2–3 · 4–6", over: { share: .02 } },
  { mod: "68% increased Physical Damage", check: false, min: 61, tier: 6, range: "65–84", over: { share: .3 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "Gain 3 Mana per enemy killed", check: false, min: 3, tier: 8, range: "2–3", over: { share: .05 }, origin: "fractured" },
  { mod: "+12% to Critical Damage Bonus", check: true, min: 11, tier: 5, range: "12–13", over: { share: .3 } },
  { mod: "+3 to Level of all Melee Skills", check: true, min: 3, tier: 3, range: "3–3", over: { share: .67 } },
];

// Sanctified sceptre: a crafted prefix, an unrevealed suffix, values outside the range of their tier.
const SCEPTRE_SANCTIFIED = { rarity: "rare", name: "Entropy Knell", base: "Rattling Sceptre", cls: "Sceptres", tags: ["Sanctified", "ilvl 79"], props: [["Spirit", "100"]], req: "Requires Level 60", statsLabel: "Stats (6)", filters: "2 filters" };
const sceptreSanctifiedStats = [
  { group: "Item", min: true },
  { mod: "Base: Rattling Sceptre", check: true },
  { mod: "Item level: 79", check: false, min: 79 },
  { mod: "Unrevealed modifiers: 1", check: true, min: 1 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Allies in your Presence deal 4 to 60 added Attack Lightning Damage", check: false, tier: 1, range: "1–4 · 60–71", over: { share: .9 } },
  { mod: "+74 to maximum Mana", check: false, min: 67, tier: 5, range: "80–89", over: { share: .42 } },
  { mod: "Allies in your Presence deal 82% increased Damage", check: false, min: 74, tier: 3, range: "75–89", over: { share: .6 }, origin: "crafted" },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "29% reduced Attribute Requirements", check: false, min: 26, tier: 2, range: "30", over: { share: .7 } },
  { mod: "50% increased Presence Area of Effect", check: false, min: 45, tier: 3, range: "46–55", over: { share: .6 } },
  { unrevealed: "Unrevealed suffix" },
  { note: "An unrevealed modifier can change the item: its other filters start unchecked." },
];

// Focus with an unrevealed prefix, in French too.
const FOCUS_UNREVEALED = { rarity: "rare", name: "Foe Ward", base: "Plumed Focus", cls: "Foci", tags: ["ilvl 39"], props: [["Energy Shield", "72"]], req: "Requires Level 36 · 52 Int", statsLabel: "Stats (5)", filters: "2 filters" };
const focusUnrevealedStats = [
  { group: "Item", min: true },
  { mod: "Base: Plumed Focus", check: true },
  { mod: "Item level: 39", check: false, min: 39 },
  { mod: "Unrevealed modifiers: 1", check: true, min: 1 },
  { group: "Pseudo" },
  { mod: "+42% total Elemental Resistance", check: false, min: 37 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "52% increased Spell Damage", check: false, min: 46, tier: 4, range: "45–54", over: { share: .55 } },
  { mod: "+36 to maximum Energy Shield", check: false, min: 32, tier: 6, range: "36–41", over: { share: .3 } },
  { unrevealed: "Unrevealed prefix" },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "+24% to Cold Resistance", check: false, min: 21, tier: 5, range: "21–25", over: { share: .45 } },
  { mod: "+18% to Fire Resistance", check: false, min: 16, tier: 6, range: "16–20", over: { share: .3 } },
  { note: "An unrevealed modifier can change the item: its other filters start unchecked." },
];
const FOCUS_UNREVEALED_FR = { rarity: "rare", name: "Protecteur ennemi", base: "Focus à plumes", cls: "Foci", tags: ["niv. 39"], props: [["Bouclier d'énergie", "72"]], req: "Niveau requis 36 · 52 Int", statsLabel: "Statistiques (5)", filters: "2 filtres", tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const focusUnrevealedStatsFr = [
  { group: "Objet", min: true },
  { mod: "Base : Focus à plumes", check: true },
  { mod: "Niveau de l'objet : 39", check: false, min: 39 },
  { mod: "Modificateurs non révélés : 1", check: true, min: 1 },
  { group: "Préfixes", fill: "3 sur 3" },
  { mod: "52% d'Augmentation des Dégâts des Sorts", check: false, min: 46, tier: 4, range: "45–54", over: { share: .55 } },
  { mod: "+36 de Bouclier d'énergie maximal", check: false, min: 32, tier: 6, range: "36–41", over: { share: .3 } },
  { unrevealed: "Préfixe non révélé" },
  { group: "Suffixes", fill: "2 sur 3 · 1 libre" },
  { mod: "+24% à la Résistance au froid", check: false, min: 21, tier: 5, range: "21–25", over: { share: .45 } },
  { mod: "+18% à la Résistance au feu", check: false, min: 16, tier: 6, range: "16–20", over: { share: .3 } },
  { note: "Un modificateur non révélé peut changer l'objet : ses autres filtres partent décochés." },
];

// A lord of the Abyss: Kurgal, one tier only, on two lines.
const FOCUS_KURGAL = { rarity: "rare", name: "Skull Anthem", base: "Sacred Focus", cls: "Foci", tags: ["ilvl 79"], props: [["Energy Shield", "167"]], req: "Requires Level 75 · 107 Int", statsLabel: "Stats (7)", filters: "4 filters" };
const focusKurgalStats = [
  { group: "Item", min: true },
  { mod: "Base: Sacred Focus", check: true },
  { mod: "Item level: 79", check: false, min: 79 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "38% increased Energy Shield", check: true, min: 34, over: { share: .9 } },
  { mod: "+27 to maximum Mana", check: false, min: 24, over: { share: .6 } },
  { rollOnly: true, tier: 2, range: "33–38 · 27–32" },
  { mod: "+40 to maximum Energy Shield", check: true, min: 36, tier: 6, range: "36–41", over: { share: .4 } },
  { mod: "+32 to maximum Mana", check: false, min: 28, tier: 9, range: "25–34", over: { share: .2 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+37% to Cold Resistance", check: true, min: 33, tier: 2, range: "36–40", over: { share: .78 } },
  { mod: "+17 to Intelligence", check: false, min: 15, tier: 5, range: "17–20", over: { share: .45 } },
  { mod: "23% chance when collecting an Elemental Infusion to gain an additional Elemental Infusion of the same type", check: true, min: 21, single: true, range: "15–25", share: .8, origin: "desecrated" },
];

// Time-Lost jewels: every modifier of one tier only, written within the words of the radius.
const SMALL = ["Small Passive Skills in Radius also grant", "Small"];
const NOTABLE = ["Notable Passive Skills in Radius also grant", "Notable"];
const TIME_LOST = { rarity: "rare", name: "Viper Glimmer", base: "Time-Lost Sapphire", cls: "Jewels", tags: ["ilvl 43"], summary: "Radius: Small", statsLabel: "Stats (4)", filters: "4 filters" };
const timeLostStats = [
  { group: "Item", min: true },
  { mod: "Base: Time-Lost Sapphire", check: true },
  { mod: "Item level: 43", check: false, min: 43 },
  { group: "Prefixes", fill: "2 of 2" },
  { radius: SMALL, mod: "2% increased Elemental Damage", check: true, min: 2, single: true, range: "1–2", share: 1 },
  { radius: NOTABLE, mod: "7% faster start of Energy Shield Recharge", check: true, min: 6, single: true, range: "5–7", share: 1 },
  { group: "Suffixes", fill: "2 of 2" },
  { radius: SMALL, mod: "1% increased Mana Regeneration Rate", check: true, min: 1, single: true, range: "1–2", share: 0 },
  { radius: NOTABLE, mod: "1% of Damage taken Recouped as Life", check: true, min: 1 },
];
const SMALL_FR = ["Les Talents Mineurs dans le Rayon octroient également", "Mineurs"];
const NOTABLE_FR = ["Les Talents éminents dans le Rayon octroient également", "Éminents"];
const TIME_LOST_FR = { rarity: "rare", name: "Lueur vipérine", base: "Saphir perdu dans le temps", cls: "Joyaux", tags: ["niv. 43"], summary: "Rayon : Petit", statsLabel: "Statistiques (4)", filters: "4 filtres", tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const timeLostStatsFr = [
  { group: "Objet", min: true },
  { mod: "Base : Saphir perdu dans le temps", check: true },
  { mod: "Niveau de l'objet : 43", check: false, min: 43 },
  { group: "Préfixes", fill: "2 sur 2" },
  { radius: SMALL_FR, mod: "2% d'Augmentation des Dégâts élémentaires", check: true, min: 2, single: true, range: "1–2", share: 1 },
  { radius: NOTABLE_FR, mod: "Le Démarrage de la Recharge du Bouclier d'énergie est 7% plus rapide", check: true, min: 6, single: true, range: "5–7", share: 1 },
  { group: "Suffixes", fill: "2 sur 2" },
  { radius: SMALL_FR, mod: "1% d'Augmentation du Taux de Régénération de Mana", check: true, min: 1, single: true, range: "1–2", share: 0 },
  { radius: NOTABLE_FR, mod: "1% des Dégâts subis sont Recouvrés sous forme de Vie", check: true, min: 1 },
];

// Breach ring: a modifier of the Genesis tree written without tier (one tier only), a catalyst quality of
// cold on a twice corrupted ring next to it.
const BREACH_RING = { rarity: "rare", name: "Glyph Circle", base: "Breach Ring", cls: "Rings", tags: ["ilvl 77"], req: "Requires Level 44", statsLabel: "Stats (7)", filters: "4 filters" };
const breachRingStats = [
  { group: "Item", min: true },
  { mod: "Base: Breach Ring", check: true },
  { mod: "Item level: 77", check: false, min: 77 },
  { group: "Implicit" },
  { mod: "+20% to Maximum Quality", check: false },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Minions have 21% increased maximum Life", check: true, min: 19, single: true, range: "19–22", share: .67 },
  { mod: "+76 to maximum Mana", check: false, min: 68, tier: 7, range: "65–79", over: { share: .4 } },
  { mod: "+129 to Evasion Rating", check: true, min: 116, tier: 4, range: "108–141", over: { share: .6 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+20 to Dexterity", check: false, min: 18, tier: 5, range: "17–20", over: { share: .5 } },
  { mod: "Gain 35 Life per enemy killed", check: true, min: 31, tier: 2, range: "29–40", over: { share: .75 } },
  { mod: "+18% to Chaos Resistance", check: true, min: 16, tier: 3, range: "16–19", over: { share: .6 } },
];
const RING_CATALYST = { rarity: "rare", name: "Wrath Circle", base: "Prismatic Ring", cls: "Rings", tags: ["Twice Corrupted", "Q 20%", "ilvl 82"], req: "Requires Level 63", statsLabel: "Stats (8)", filters: "5 filters" };
const ringCatalystStats = [
  { group: "Item", min: true },
  { mod: "Base: Prismatic Ring", check: true },
  { mod: "Item level: 82", check: false, min: 82 },
  { ...catalyst("Cold", 20), check: false, min: 20 },
  { group: "Enchant" },
  { mod: "+14 to Dexterity", check: false, min: 13, range: "10–15", share: .8 },
  { group: "Implicit" },
  { mod: "+10% to all Elemental Resistances", check: false, min: 9, range: "7–10", share: 1 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "Adds 1 to 65 Lightning damage to Attacks", check: true, tier: 1, range: "1–4 · 60–71", over: { share: .85 } },
  { mod: "+125 to Evasion Rating", check: false, min: 113, tier: 4, range: "108–141", over: { share: .55 }, origin: "desecrated" },
  { mod: "Adds 8 to 20 Physical Damage to Attacks", check: true, tier: 3, range: "7–11 · 14–20", over: { share: .6 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "16% increased Rarity of Items found", check: true, min: 14, tier: 1, range: "15–18", over: { share: .8 } },
  { mod: "+28 to Dexterity", check: false, min: 25, tier: 2, range: "28–30", over: { share: .85 } },
  { mod: "67% increased Mana Regeneration Rate", check: true, min: 60, tier: 1, range: "60–69", over: { share: .9 } },
];

SECTIONS.push(["Endgame modifiers (13k1): desecrated, fractured, unrevealed, one tier, catalysts", [
  ["Time-Lost jewel: one tier only", win(topBar(), item({ ...TIME_LOST, stats: timeLostStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Time-Lost jewel, French", win(topBar(), item({ ...TIME_LOST_FR, stats: timeLostStatsFr }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Desecrated amulet, catalyst quality", win(topBar(), item({ ...AMULET_DESECRATED, stats: amuletDesecratedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Catalyst quality, twice corrupted", win(topBar(), item({ ...RING_CATALYST, stats: ringCatalystStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Fractured suffix", win(topBar(), item({ ...MACE_FRACTURED, stats: maceFracturedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Unrevealed prefix", win(topBar(), item({ ...FOCUS_UNREVEALED, stats: focusUnrevealedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Unrevealed prefix, French", win(topBar(), item({ ...FOCUS_UNREVEALED_FR, stats: focusUnrevealedStatsFr }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Sanctified: values outside their tier, crafted, unrevealed", win(topBar(), item({ ...SCEPTRE_SANCTIFIED, stats: sceptreSanctifiedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["A lord of the Abyss: one tier only", win(topBar(), item({ ...FOCUS_KURGAL, stats: focusKurgalStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["Breach ring: a modifier without tier", win(topBar(), item({ ...BREACH_RING, stats: breachRingStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
