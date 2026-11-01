// Section Prices in other currencies (fix a): an offer priced in another currency than the Exalted, Chaos and
// Divine Orbs wrote the raw id of the trade site without icon (`3 waystone-12`, `1 vaal`). Every item of the
// currency exchange gets its icon (771 of them in /api/trade2/data/static, embedded), and its name in the
// language of the item. A catch found in the data on 2026-11-01: only 639 images for 771 items, the Greater
// and Perfect Orbs sharing the icon of their orb (a Perfect Exalted Orb looks like an Exalted Orb), the 19
// Thaumaturgic Fluxes one icon, the 63 Legacies one icon, and the 16 Waystones apart, their tier unreadable
// at 18 px. One open decision, the name after the icon:
// - A, the icon alone, as the three orbs, its name in a tooltip over it;
// - B, the name after the icon of every currency but the three orbs;
// - C, the icon alone for the 10 currencies of the price filter of the trade site (Exalted, Divine, Chaos,
//   Augmentation, Transmutation, Regal, Vaal, Alchemy, Annulment, Mirror), which every player knows by
//   sight, the name after the icon of any other; the name of an icon alone in a tooltip over it;
// - D, asked by the owner on 2026-11-01 (there is room, and it helps the accessibility): the name after the
//   icon of every currency, the three orbs too.
// An item without icon (added to the trade site since the icons were taken; here a made-up name): its name
// alone, in every option. The prices are sample data, sorted as the trade site sorts them, by its rates.

const CURRENCY_NAMES = {
  ex: ["Exalted Orb", "Orbe exalté"],
  div: ["Divine Orb", "Orbe divin"],
  chaos: ["Chaos Orb", "Orbe du chaos"],
  regal: ["Regal Orb", "Orbe royal"],
  vaal: ["Vaal Orb", "Orbe vaal"],
  "waystone-12": ["Waystone (Tier 12)", "Pierre de téléportation (Palier 12)"],
  "perfect-exalted-orb": ["Perfect Exalted Orb", "Orbe exalté parfait"],
  "greater-chaos-orb": ["Greater Chaos Orb", "Orbe du chaos majeur"],
};
// The 10 currencies of the price filter of the trade site, known by sight.
const PRICE_FILTER = ["ex", "div", "chaos", "aug", "transmute", "regal", "vaal", "alch", "annul", "mirror"];
const THREE_ORBS = ["ex", "div", "chaos"];
// The offers of a rare waystone of tier 12; `k` absent for the item without icon.
const otherCurrencyRows = [
  { v: "1", k: "ex", when: "3h ago" },
  { v: "2", k: "regal", when: "1d ago" },
  { v: "3", k: "waystone-12", when: "20min ago" },
  { v: "1", k: "vaal", when: "2h ago", away: true },
  { v: "4", when: "5h ago", name: ["Gilded Effigy", "Effigie dorée"] },
  { v: "1", k: "greater-chaos-orb", when: "1h ago" },
  { v: "1", k: "perfect-exalted-orb", when: "6h ago" },
  { v: "1", k: "div", when: "2d ago" },
];
const frenchWhen = when => when.replace(/^(\d+)(min|h|d) ago$/, (_, n, unit) => `il y a ${n}${unit === "d" ? "j" : unit}`);
// The rows of `option` (A, B, C or D) in `lang`; `hovered`, the index of a row whose icon is under the pointer.
const otherCurrencyOffers = (option, lang = "en", hovered) => otherCurrencyRows.map((r, i) => {
  const index = lang === "fr" ? 1 : 0;
  const name = r.name ? r.name[index] : CURRENCY_NAMES[r.k]?.[index];
  const named = r.k && (option === "D" || !THREE_ORBS.includes(r.k) && option === "B" || !THREE_ORBS.includes(r.k) && (option === "C" && !PRICE_FILTER.includes(r.k)));
  const iconAlone = r.k && !THREE_ORBS.includes(r.k) && !named;
  return {
    ...r,
    when: lang === "fr" ? frenchWhen(r.when) : r.when,
    name: !r.k || named ? name : undefined,
    tip: iconAlone ? name : undefined,
    tipShown: i === hovered,
  };
});

const OTHER_WAYSTONE_PRICE = { estimate: true, values: ["0.01", "0.14", "10"], offers: "84 similar", note: "Median of the 5 cheapest similar offers." };
const OTHER_WAYSTONE_PRICE_FR = { estimate: true, values: ["0.01", "0.14", "10"], offers: "84 similaires", note: "Médiane des 5 offres similaires les moins chères." };
const otherCurrencyBlock = (option, lang = "en", extra = {}) => offers({
  ...(lang === "fr"
    ? { count: "10 sur 84 en ligne", columns: ["Prix", "Publiée"], age: "recherche il y a 2min" }
    : { count: "10 of 84 online", columns: ["Price", "Listed"] }),
  rows: otherCurrencyOffers(option, lang, extra.hoveredIcon),
  ...extra,
}, lang);
const otherWindow = (option, lang, extra) => win(topBar(), item(lang === "fr" ? WAYSTONE_FR : WAYSTONE),
  price(lang === "fr" ? OTHER_WAYSTONE_PRICE_FR : OTHER_WAYSTONE_PRICE, lang), otherCurrencyBlock(option, lang, extra), foot());

// D chosen by the owner on 2026-11-01. Then, asked by the owner: the offers priced in Waystones left out, a
// waystone not being stackable, so that such a price is odd, and those priced in a currency without icon
// (unknown to the application, the Gilded Effigy here); the trade site can't leave them out (its price
// filter takes one option: every currency in Exalted Orbs, the Exalted or Divine Orbs, or one of the 10
// currencies), so the application hides them, while the trade site counts them in its total. One open
// decision:
// - E1, hidden without a word: the count is that of the rows shown, "6 of 84 online";
// - E2, the same, and a faint line under the list, "2 offers in Waystones or an unknown currency left
//   out.", so that a list whose
//   last offers are all shown, short of the total, doesn't look broken.
// Chosen by the owner on 2026-11-01: E1, to keep the market clean: the application doesn't encourage the
// sellers who price in odd currencies.
const withoutWaystones = rows => rows.filter(r => r.k && !r.k.startsWith("waystone-"));
const dWindow = (lang, extra = {}) => win(topBar(), item(lang === "fr" ? WAYSTONE_FR : WAYSTONE),
  price(lang === "fr" ? OTHER_WAYSTONE_PRICE_FR : OTHER_WAYSTONE_PRICE, lang),
  offers({
    ...(lang === "fr"
      ? { count: "6 sur 84 en ligne", columns: ["Prix", "Publiée"], age: "recherche il y a 2min" }
      : { count: "6 of 84 online", columns: ["Price", "Listed"] }),
    rows: withoutWaystones(otherCurrencyOffers("D", lang)),
    ...extra,
  }, lang), foot());

SECTIONS.unshift(["Prices in other currencies (fix a): D chosen; the offers in Waystones or an unknown currency left out, E1 chosen", [
  ["E1 · the offers in Waystones and in an unknown currency hidden without a word", dWindow("en")],
  ["D and E1 · in French", dWindow("fr")],
  // The tooltip of an offer: its price with the icon alone, without the name (owner, 2026-11-01).
  ["D · an offer in Vaal Orbs hovered: the icon alone in its tooltip", dWindow("en", { hover: 2, tooltip: {
    rarity: "rare", name: "Gloom Path", price: "1", currency: "vaal",
    meta: "Waystone (Tier 12) · ilvl 81", status: "away",
    mods: [
      { text: "Monsters have 95% increased Stun Buildup", value: 95, yours: 91 },
      { text: "17% increased Monster Damage", value: 17, yours: 18 },
      { text: "Monster have 112% increased Elemental Ailment Application", value: 112, yours: 106 },
    ],
  } })],
  // The narrowest column: the price of the currency exchange, beside the stock and the listing time; its
  // offers are always paid in Exalted Orbs (in Divine Orbs for the Exalted Orb itself), whose name fits.
  ["D · the currency exchange in French, its narrow column", win(topBar(),
    item({ rarity: "currency", name: "Orbe d'annulation", cls: "Objets monétaires empilables", tags: ["Pile 7/20"], statsLabel: "Statistiques (1)" }),
    price({ values: ["0.38", "3.7", "262"], stackValues: ["2.65", "26", "1.8K"], stackSize: 7, rising: false, trend: "−3 % sur les 7 derniers jours" }, "fr"),
    offers({ count: "8 sur 48 en ligne", columns: ["Par unité", "Stock", "Publiée"], age: "recherche il y a 2min", rows: [
      { v: "255", k: "ex", stock: "40", when: "12min", name: "Orbe exalté" },
      { v: "258", k: "ex", stock: "12", when: "3min", name: "Orbe exalté" },
      { v: "260", k: "ex", stock: "120", when: "45min", name: "Orbe exalté" },
      { v: "262", k: "ex", stock: "33", when: "3h", name: "Orbe exalté" },
      { v: "265", k: "ex", stock: "8", when: "9min", name: "Orbe exalté" },
    ] }, "fr"), foot())],
]]);

// Put after the section above.
SECTIONS.splice(1, 0, ["Prices in other currencies (fix a), the first decision: D chosen, the name after every icon", [
  ["Today: the raw id of the trade site, without icon", win(topBar(), item(WAYSTONE), price(OTHER_WAYSTONE_PRICE),
    offers({ count: "10 of 84 online", columns: ["Price", "Listed"], rows: otherCurrencyRows.map(r => ({
      ...r, k: THREE_ORBS.includes(r.k) ? r.k : undefined,
      name: THREE_ORBS.includes(r.k) ? undefined : r.k || "gilded-effigy",
    })) }), foot())],
  ["A · the icon alone, its name in a tooltip (the Perfect Exalted Orb hovered)", otherWindow("A", "en", { hoveredIcon: 6 })],
  ["B · the name after every icon but the three orbs", otherWindow("B", "en")],
  ["C · the 10 currencies of the price filter by their icon alone, the others named (the Vaal Orb hovered)", otherWindow("C", "en", { hoveredIcon: 3 })],
  ["D · the name after every icon, the three orbs too (the owner's idea)", otherWindow("D", "en")],
]]);
