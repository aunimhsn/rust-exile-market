// Section Search filters (13e): the range field of the filters and its tooltip, a line the trade site
// can't search (a modifier added by a patch): a circle struck through in place of its checkbox. A line
// better lower ("reduced Movement Speed"): searched with a maximum, so that the offers found are
// at least as good. The Evasion Rating and the Lightning Resistance have a maximum set by the player (a budget); the
// Energy Shield and the Evasion Rating have numbers of three digits, the Armour a range of four
// digits, which widens its field.
// The tooltip of a line the trade site can't search: two sentences, one per line.
const UNSEARCHABLE = {
  en: "The trade site can't search this line.<br>The offers may not have it.",
  fr: "Le site d'échange ne peut pas chercher cette ligne.<br>Il se peut que les offres ne l'aient pas.",
};
const unsearchableRow = (language, text, range, share, hovered = false) =>
  ({ mod: text, warning: UNSEARCHABLE[language], hover: hovered, range, share });
// The stats of the sample unique, in English or French; `hover` opens the tooltip of the range field
// of a line: { [line index]: true }, or the warning of the line without trade id: { warning: true }.
const stormtreadStats = (language, hover = {}) => {
  const en = language === "en";
  const rows = [
    { mod: en ? "+52 to maximum Life" : "+52 à la Vie maximale", check: true, min: 52, range: "40–60", share: .6 },
    { mod: en ? "+122 to maximum Energy Shield" : "+122 au Bouclier d'énergie maximal", check: true, min: 122, range: "100–150", share: .44 },
    { mod: en ? "+245 to Evasion Rating" : "+245 au Score d'évasion", check: true, min: 245, max: 310, range: "200–320", share: .38 },
    { mod: en ? "+2250 Armour if you've Blocked Recently" : "+2250 à l'Armure si vous avez Bloqué récemment", check: true, min: 2250, range: "1500–3000", share: .5, wide: true },
    { mod: en ? "18% reduced Movement Speed" : "18% de Réduction de la Vitesse de déplacement", check: true, min: 18, max: 18, lower: true, range: "15–25", share: .3 },
  ].map((row, index) => hover[index] ? { ...row, hover: hover[index] } : row);
  return [
    { group: "Unique", min: true },
    ...rows,
    unsearchableRow(language, en ? "Gain 3 Thunder Charges when you Dodge Roll" : "Gagne 3 Charges de foudre quand vous effectuez une Roulade", "2–4", .5, Boolean(hover.warning)),
    { mod: en ? "+25% to Lightning Resistance" : "+25% à la Résistance à la foudre", check: true, min: 25, max: 28, range: "20–30", share: .5 },
    { mod: en ? "Your Dodge Roll leaves a trail of Shocked Ground" : "Votre Roulade laisse une traînée de Sol électrifié" },
  ];
};
const stormtread = (language, hover) => language === "en"
  ? win(topBar(), item({ rarity: "unique", name: "Stormtread", base: "Lattice Sandals", cls: "Boots", tags: ["ilvl 72"], props: [["Energy Shield", "64"]], req: "Requires Level 52 · 41 Int", filters: "6 filters", statsLabel: "Stats (8)",
      stats: stormtreadStats(language, hover) }, true), price({ values: ["0.3", "3", "220"], offers: "9 offers", trend: "Stable over the last 7 days", stable: true }), foot())
  : win(topBar(), item({ rarity: "unique", name: "Foulée d'orage", base: "Sandales en treillis", cls: "Bottes", tags: ["ilvl 72"], props: [["Bouclier d'énergie", "64"]], req: "Niveau requis 52 · 41 Int", filters: "6 filtres", statsLabel: "Statistiques (8)", tradeLabel: "Voir sur l'échange",
      stats: stormtreadStats(language, hover) }, true), price({ values: ["0.3", "3", "220"], offers: "9 offres", trend: "Stable sur les 7 derniers jours", stable: true }), foot());
SECTIONS.push(["Search filters (13e): a line without trade id, a line better lower", [
  ["Unique: a line of a new patch, a penalty", stormtread("en")],
  ["The same, in French", stormtread("fr")],
  ["The line without trade id hovered, in French", stormtread("fr", { warning: true })],
]]);
SECTIONS.push(["Range field tooltip (13e): the rolls the field accepts, the better side", [
  ["A field hovered", stormtread("en", { 0: "max" })],
  ["A line better lower hovered, in French", stormtread("fr", { 4: "max" })],
  ["A line better lower hovered", stormtread("en", { 4: "max" })],
]]);
SECTIONS.push(["Rare: its range field tooltip (13j), a line without trade id under a tier 1", [
  ["After the modifier tables: its rolls over every tier", win(topBar(),
    item({ rarity: "rare", name: "Doom Shell", base: "Wanderer Coat", cls: "Body Armours", tags: ["ilvl 82"], props: [["Armour", "412"], ["Evasion", "380"]], req: "Requires Level 65 · 59 Str · 59 Dex", filters: "3 filters", statsLabel: "Stats (3)",
      stats: [
        { group: "Prefixes", fill: "1 of 3 · 2 open", min: true },
        { mod: "+86 to maximum Life", check: true, min: 77, tier: 3, range: "85–99", share: .1, noBar: true, bounds: "10 to 214", boundTiers: [13, 1], hover: "min" },
        { group: "Suffixes", fill: "2 of 3 · 1 open" },
        { mod: "+41% to Cold Resistance", check: true, min: 37, tier: 1, range: "41–45", share: .1, noBar: true, bounds: "6 to 45", boundTiers: [8, 1] },
        { mod: "+23% to Lightning Resistance", check: true, min: 21, tier: 4, range: "21–25", share: .5, noBar: true, bounds: "6 to 45", boundTiers: [8, 1] },
      ] }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["A line without trade id under a tier 1", win(topBar(),
    item({ rarity: "rare", name: "Storm Loop", base: "Sapphire Ring", cls: "Rings", tags: ["ilvl 81"], req: "Requires Level 60", filters: "3 filters", statsLabel: "Stats (4)",
      stats: [
        { group: "Prefixes", fill: "1 of 3 · 2 open", min: true },
        { mod: "+86 to maximum Life", check: true, min: 77, tier: 3, range: "85–99", share: .1, noBar: true },
        { group: "Suffixes", fill: "3 of 3" },
        { mod: "+44% to Cold Resistance", check: true, min: 40, tier: 1, range: "41–45", share: .1, noBar: true },
        { ...unsearchableRow("en", "Gain 3 Thunder Charges when you Dodge Roll", "3–4", .5), tier: 1, noBar: true },
        { mod: "+23% to Lightning Resistance", check: true, min: 21, tier: 4, range: "21–25", share: .5, noBar: true },
      ] }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
