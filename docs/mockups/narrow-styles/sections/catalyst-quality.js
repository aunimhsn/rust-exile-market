// Section Catalyst quality (13k4): the kind of quality of the offers. The `quality` filter of the trade site
// doesn't tell the kind of quality of a ring, an amulet or a jewel apart: "Mana quality ≥ 40%" also finds
// the amulets of attack, caster or minion quality, which raise other modifiers. The trade site writes the
// kind of each offer in its answer (`Quality (Attack Modifiers)`, seen on 2026-10-31), which Exiled Exchange 2
// uses nowhere (it has no quality filter on jewellery). Three options, open for the owner:
// - A, the kind in the tooltip of the offer only: in red when it differs; the estimate unchanged; the
//   tooltip of the filter says that the trade site searches any kind;
// - B, A, and the offers of another kind marked in the list (their price faint, the kind after it) and left
//   out of the estimate, as the suspicious offers are; the tooltip of the filter says so;
// - C, the offers of another kind hidden from the list and left out of the estimate, a line under it says
//   how many.
// Chosen by the owner on 2026-10-31: B. Then, the owner's idea: in the tooltip of an offer, a quality at the
// maximum of its item in white, gray below it: 20%, more with its modifiers `+#% to Maximum Quality` (the
// Essence of the Breach, +20%; the implicit of a Breach Ring, +20%, of a Refined one, +25%); a quality pushed
// beyond by a Vaal Infuser (up to 10% more) is at its maximum too. A quality of another kind stays red.
// In B and C, only while the filter of the quality is checked: unchecked, the player doesn't search the
// quality, every offer counts (the kind stays in the tooltip, as in A). The filter is unchecked at first;
// the windows of the offers show it checked by the player.
// Ten offers of the search, from the cheapest: the median of the five cheapest is 2.4 Divine Orbs with every
// kind, 3.2 with the mana quality only.

const AMULET_MANA = { rarity: "rare", name: "Rune Collar", base: "Solar Amulet", cls: "Amulets",
  tags: ["Q 40%", "ilvl 81"], req: "Requires Level 60", statsLabel: "Stats (8)", filters: "4 filters" };
const AMULET_MANA_FR = { rarity: "rare", name: "Collier runique", base: "Amulette solaire", cls: "Amulettes",
  tags: ["Q 40%", "niv. 81"], req: "Niveau requis 60", statsLabel: "Statistiques (8)", filters: "4 filtres",
  tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
const amuletManaStats = tip => [
  { group: "Item", min: true },
  { mod: "Base: Solar Amulet", check: true },
  { mod: "Item level: 81", check: false, min: 81 },
  { mod: "Mana quality: 40%", check: false, min: 40, tip },
  { group: "Implicit" },
  { mod: "+12 to Spirit", check: false, min: 11, range: "10–15", share: .4 },
  { group: "Prefixes", fill: "2 of 3 · 1 open" },
  { mod: "+89 to maximum Mana", check: true, min: 80, tier: 3, range: "80–89", over: { share: .7 } },
  { mod: "+35 to Intelligence", check: true, min: 32, tier: 3, range: "33–36", over: { share: .6 } },
  { group: "Suffixes", fill: "2 of 3 · 1 open" },
  { mod: "+25% to Lightning Resistance", check: true, min: 23, tier: 5, range: "21–25", over: { share: .5 } },
  { mod: "18% increased Mana Regeneration Rate", check: false, min: 16, tier: 6, range: "15–19", over: { share: .4 } },
];

// The offers, from the cheapest, with the kind of their quality.
const MANA_OFFERS = [
  { v: "1.5", k: "div", when: "2h ago", kind: "Caster" },
  { v: "2", k: "div", when: "20min ago", kind: "Attack" },
  { v: "2.4", k: "div", when: "1d ago" },
  { v: "2.5", k: "div", when: "4h ago", away: true },
  { v: "3", k: "div", when: "6h ago", kind: "Minion" },
  { v: "3.2", k: "div", when: "1h ago" },
  { v: "3.5", k: "div", when: "3h ago" },
  { v: "4", k: "div", when: "2d ago" },
];
// `marked`: the kind after the price of an offer of another kind (B); `hidden`: those offers left out (C).
const manaRows = ({ marked, hidden } = {}) => MANA_OFFERS
  .filter(r => !(hidden && r.kind))
  .map(({ kind, ...r }) => marked && kind ? { ...r, other: kind } : r);
const manaMods = [
  { text: "+12 to Spirit", value: 12, yours: 12 },
];
const manaExplicit = [
  { text: "+84 to maximum Mana", value: 84, yours: 89, tier: 3 },
  { text: "+31 to Intelligence", value: 31, yours: 35, tier: 4 },
  { text: "+27% to Lightning Resistance", value: 27, yours: 25, tier: 4 },
];
const tooltipOf = (r, quality, extra = {}) => ({
  rarity: "rare", name: "Gale Locket", price: r.v, currency: r.k, meta: "Solar Amulet · ilvl 82 · Corrupted", status: r.away ? "away" : "online",
  props: [quality], implicit: manaMods, mods: manaExplicit, ...extra,
});
const OTHER_KIND = { text: "Minion quality: 50%", other: true };
// Of the same kind, below the maximum of the offer (40%: 20%, and 20% of its essence): gray, its difference.
const SAME_KIND = { text: "Mana quality: 30%", value: 30, yours: 40 };
// At the maximum of the offer: white.
const SAME_KIND_MAX = { text: "Mana quality: 40%", value: 40, yours: 40, max: true };
const essenceMods = [{ text: "+20% to Maximum Quality" }, ...manaExplicit];

const EVERY_KIND = { estimate: true, values: ["2.4", "24", "1.7K"], offers: "27 similar", note: "Median of the 5 cheapest similar offers." };
const MANA_ONLY = { estimate: true, values: ["3.2", "32", "2.3K"], offers: "27 similar", note: "Median of the 5 cheapest similar offers of mana quality." };
const manaOffers = (extra = {}) => offers({ count: "10 of 27 online", columns: ["Price", "Listed"], rows: manaRows(), ...extra });
const ANY_KIND_TIP = "The trade site searches any kind of quality.";
const LEFT_OUT_TIP = "The trade site searches any kind of quality: the offers of another kind aren't counted in the estimate.";

SECTIONS.unshift(["Catalyst quality (13k4): the kind of quality of the offers, B chosen, the quality at its maximum in white", [
  ["Chosen · B, the filter checked, an offer at its maximum: its quality in white", win(topBar(), item(AMULET_MANA), price(MANA_ONLY),
    manaOffers({ rows: manaRows({ marked: true }), hover: 6, tooltip: tooltipOf(MANA_OFFERS[6], SAME_KIND_MAX, { mods: essenceMods }) }), foot())],
  ["Chosen · B, below its maximum: gray, its difference", win(topBar(), item(AMULET_MANA), price(MANA_ONLY),
    manaOffers({ rows: manaRows({ marked: true }), hover: 3, tooltip: tooltipOf(MANA_OFFERS[3], SAME_KIND, { mods: essenceMods }) }), foot())],
  ["Chosen · B, the filter, unchecked at first: its tooltip says the estimate leaves them out", win(topBar(), item({ ...AMULET_MANA, stats: amuletManaStats(LEFT_OUT_TIP) }, true), foot())],
  ["Chosen · B, the filter checked, another kind: faint, its kind after the price, left out of the estimate", win(topBar(), item(AMULET_MANA), price(MANA_ONLY),
    manaOffers({ rows: manaRows({ marked: true }) }), foot())],
  ["Chosen · B, an offer of another kind hovered", win(topBar(), item(AMULET_MANA), price(MANA_ONLY),
    manaOffers({ rows: manaRows({ marked: true }), hover: 4, tooltip: tooltipOf(MANA_OFFERS[4], OTHER_KIND, { note: "Another kind of quality: not counted in the estimate." }) }), foot())],
  ["Chosen · B, French", win(topBar(), item(AMULET_MANA_FR),
    price({ ...MANA_ONLY, offers: "27 similaires", note: "Médiane des 5 offres similaires les moins chères de qualité de mana." }, "fr"),
    offers({ count: "10 sur 27 en ligne", columns: ["Prix", "Publiée"], age: "recherche il y a 2min", rows: manaRows({ marked: true }).map(r => ({
      ...r, when: r.when.replace(/^(\S+) ago$/, "il y a $1").replace("1d", "1j").replace("2d", "2j"),
      other: r.other && { Caster: "Lanceur de sorts", Attack: "Attaque", Minion: "Créature" }[r.other] })) }, "fr"), foot("poe2scout il y a 12min · GGG trade il y a 2min"))],
  ["A · the filter, unchecked at first: its tooltip says the trade site searches any kind", win(topBar(), item({ ...AMULET_MANA, stats: amuletManaStats(ANY_KIND_TIP) }, true), foot())],
  ["A · every offer counted", win(topBar(), item(AMULET_MANA), price(EVERY_KIND), manaOffers(), foot())],
  ["A · an offer of another kind: its quality in red", win(topBar(), item(AMULET_MANA), price(EVERY_KIND),
    manaOffers({ hover: 4, tooltip: tooltipOf(MANA_OFFERS[4], OTHER_KIND) }), foot())],
  ["A · an offer of the same kind: its difference (the same in B and C)", win(topBar(), item(AMULET_MANA), price(EVERY_KIND),
    manaOffers({ hover: 3, tooltip: tooltipOf(MANA_OFFERS[3], SAME_KIND, { mods: essenceMods }) }), foot())],
  ["C · another kind hidden, how many under the list", win(topBar(), item(AMULET_MANA), price(MANA_ONLY),
    manaOffers({ rows: manaRows({ hidden: true }), footNote: "3 offers of another kind of quality hidden." }), foot())],
]]);
