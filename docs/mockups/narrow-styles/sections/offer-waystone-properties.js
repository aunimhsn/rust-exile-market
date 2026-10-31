// Section Offer waystone properties (13k5c2): in the tooltip of an offer of waystone, its properties with
// their difference from the copied waystone, which Exiled Exchange 2 doesn't show. The trade site gives
// them in `/fetch` (`properties`, seen on 2026-10-31: `[ItemRarity|Item Rarity]` "+11%", `[LimitedRespawn|
// Revives Available]` "0"); they come first, where the quality of another item does, written as their
// filters. Every property is better higher, the revives too: a buyer pays for the bonuses (the rewards of
// the monsters better higher, the owner's rule of 13k3e).
// The copied waystone is the owner's of tier 12 (Revives 3, Item Rarity +11%, Monster Rarity +25%, Monster
// Effectiveness +13%, Waystone Drop Chance +50%, no Pack Size), its item rarity and drop chance checked
// (13k5c1, B). The offer hovered has no Monster Effectiveness, and a Pack Size the copied one lacks.
// Two open decisions:
// - a property of the copied waystone that the offer lacks: A, not written, as any property the offer
//   doesn't have; B, written faint at 0, its loss in red, so that the buyer sees what this offer gives up;
// - the tier in the line under the name: 1, as the trade site writes it, "Waystone (Tier 12)" (today);
//   2, short and in its color, as in the item block, "Waystone · T12".
// Chosen by the owner on 2026-10-31: B and 2; the other options kept below, for the record.

// The offers, from the cheapest: all with at least 9 % item rarity and 45 % drop chance.
const WAYSTONE_OFFERS = [
  { v: "18", k: "ex", when: "3h ago" },
  { v: "20", k: "ex", when: "40min ago" },
  { v: "22", k: "ex", when: "1d ago", away: true },
  { v: "25", k: "ex", when: "2h ago" },
  { v: "28", k: "ex", when: "5h ago" },
  { v: "30", k: "ex", when: "1h ago" },
  { v: "35", k: "ex", when: "6h ago" },
  { v: "40", k: "ex", when: "2d ago" },
];
// `[text, value, the copied waystone's]`; the offer's revives, rarities and drop chance, a pack size the copied
// one lacks, no monster effectiveness.
const offerProps = (lacking, fr) => [
  [fr ? "Résurrections disponibles : 0" : "Revives available: 0", 0, 3],
  [fr ? "Rareté des objets : +14%" : "Item rarity: +14%", 14, 11],
  [fr ? "Taille des groupes : +8%" : "Pack size: +8%", 8, 0],
  [fr ? "Rareté des monstres : +31%" : "Monster rarity: +31%", 31, 25],
  ...lacking ? [[fr ? "Efficacité des monstres : 0%" : "Monster effectiveness: 0%", 0, 13, true]] : [],
  [fr ? "Chances de trouver des pierres de téléportation : +62%" : "Waystone drop chance: +62%", 62, 50],
].map(([text, value, yours, lack]) => ({ text, value, yours, lacking: lack }));
const offerWaystoneMods = fr => fr ? [
  { text: "Les Monstres ont 34% d'Augmentation de la Vitesse d'attaque" },
  { text: "Les Monstres infligent 20% des Dégâts sous forme de Dégâts de froid supplémentaires" },
  { text: "Les Monstres ont 52% d'Augmentation de la Chance de coup critique" },
] : [
  { text: "Monsters have 34% increased Attack Speed" },
  { text: "Monsters deal 20% of Damage as Extra Cold" },
  { text: "Monsters have 52% increased Critical Hit Chance" },
];
// `lacking`: B; `short`: 2.
const waystoneTooltip = ({ lacking = false, short = false, fr = false } = {}) => ({
  rarity: "rare", name: fr ? "Seuil sinistre" : "Grim Threshold", price: "25", currency: "ex", status: "online",
  meta: `${short ? waystoneBase(fr ? "Pierre de téléportation" : "Waystone", 12) : fr ? "Pierre de téléportation (Palier 12)" : "Waystone (Tier 12)"} · ${fr ? "niv. 81" : "ilvl 81"}`,
  props: offerProps(lacking, fr), mods: offerWaystoneMods(fr),
});
const WAYSTONE_ESTIMATE = { estimate: true, values: ["0.04", "0.4", "25"], offers: "31 similar", note: "Median of the 5 cheapest similar offers." };
const waystoneOffersWin = tooltip => win(topBar(), item({ ...WAYSTONE_PROPS, filters: "3 filters" }), price(WAYSTONE_ESTIMATE),
  offers({ count: "10 of 31 online", columns: ["Price", "Listed"], rows: WAYSTONE_OFFERS, hover: 3, tooltip }), foot());
const FR_WHEN = r => ({ ...r, when: r.when.replace(/^(\S+) ago$/, "il y a $1").replace("1d", "1j").replace("2d", "2j") });

SECTIONS.unshift(["Offer waystone properties (13k5c2): B and 2 chosen, the properties of an offer of waystone in its tooltip, a lacking one faint at 0, the tier short", [
  ["Chosen · B · 2, a lacking property faint at 0, the tier short and in its color", waystoneOffersWin(waystoneTooltip({ lacking: true, short: true }))],
  ["Chosen · B · 2, French", win(topBar(), item({ ...WAYSTONE_PROPS_FR, filters: "3 filtres" }),
    price({ ...WAYSTONE_ESTIMATE, offers: "31 similaires", note: "Médiane des 5 offres similaires les moins chères." }, "fr"),
    offers({ count: "10 sur 31 en ligne", columns: ["Prix", "Publiée"], age: "recherche il y a 2min", rows: WAYSTONE_OFFERS.map(FR_WHEN), hover: 3,
      tooltip: waystoneTooltip({ lacking: true, short: true, fr: true }) }, "fr"), foot("poe2scout il y a 12min · GGG trade il y a 2min"))],
  ["Not chosen · A, a lacking property not written", waystoneOffersWin(waystoneTooltip({ short: true }))],
  ["Not chosen · 1, the tier as the trade site writes it", waystoneOffersWin(waystoneTooltip({ lacking: true }))],
  ["Today · no property", waystoneOffersWin({ ...waystoneTooltip(), props: [] })],
]]);
