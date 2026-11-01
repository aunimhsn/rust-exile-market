// Section Offer gem properties (13k5d2): in the tooltip of an offer of skill gem, its level, its quality and
// its sockets with their difference from the copied gem, which Exiled Exchange 2 doesn't show. The trade site
// gives them in `/fetch` (seen on 2026-11-01: `properties` `Level` "16", "21", `[Quality]` "+20%",
// and `gemSockets` ["W", "W", "W"]). Every property is better higher.
// The copied gem is a Comet of level 15, 12 % of quality (not checked: below 16 %) and 3 sockets (checked),
// so the offers have level 15 or more and 3 sockets or more, any quality (13k5d1, 2 B). The offer hovered is
// level 16, without quality (a property of the copied gem it lacks: faint at 0, its loss in red, as the
// properties of a waystone, 13k5c2 B), 4 sockets.
// In every option, the line under the name gives the item class of the copied gem, "Skill Gems", in place of
// the name of the gem written again (a gem has no base type of its own: the trade site writes its name
// there) and of its item level (0), then the corruption, as the line of any offer.
// Three decisions:
// - 1, where the level is: A, in the group, first, with its difference in the column of the others; B, under
//   the name, short and in the colors of the item block (20 orange, 21 red), as the tier of a waystone, without
//   difference; C, as B, its difference right after it;
// - 2, the color of a line at its top (level 20 or more, quality 20 % or more, 5 sockets): A, white, as the
//   quality of an offer at its maximum (13k4); B, gray as the others. Not the orange and red of the item block:
//   a quality line in red already tells a quality of another kind (13k4).
// - 3, the owner's question: the requirements of the gem: A, not written, as today; B, a line under the class,
//   as in the item block, its attributes in their colors. Not compared with the copied gem: the level of the
//   character is unknown, a higher requirement is neither better nor worse.
// Chosen by the owner on 2026-11-01: 1 B, 2 A, 3 B, and the requirements of the offers of weapons and armours
// too; the other options kept below, for the record.
// Checked on the trade site on 2026-11-01 (Runes of Aldur, three searches, English and French): `/fetch` gives
// `requirements` (`Level` type 62, French `Niveau`; `[Dexterity|Dex]` type 64, `[Intelligence|Int]` type 65)
// for a gem and a body armour; a gem has `ilvl` 0, not written; its `Level` is "21", and a corruption can lower
// it ("-1 Level from Corruption").

const COMET_COPIED = { ...COMET, filters: "2 filters" };
// The offers, from the cheapest: level 15 or more, 3 sockets or more.
const GEM_OFFERS = [
  { v: "2", k: "ex", when: "1h ago" },
  { v: "3", k: "ex", when: "4h ago" },
  { v: "4", k: "ex", when: "20min ago", away: true },
  { v: "6", k: "ex", when: "3h ago" },
  { v: "8", k: "ex", when: "1d ago" },
  { v: "12", k: "ex", when: "2h ago" },
  { v: "25", k: "ex", when: "5h ago" },
  { v: "40", k: "ex", when: "2d ago" },
];
// The level in the line under the name, in the colors of the item block, and its difference after it (`delta`).
const levelMeta = (word, level, yours, delta) => {
  const color = level >= 21 ? " is-top" : level >= 20 ? " is-high" : "";
  const diff = level - yours;
  const after = delta && diff ? ` <span class="delta ${diff > 0 ? "up" : "down"}">${diff > 0 ? "+" : "−"}${Math.abs(diff)}</span>` : "";
  return `<span class="app-waystone-tier${color}">${word} ${level}</span>${after}`;
};
// The tooltip of the offer `o` ({level, quality, sockets, corrupted}) in decisions `level` ("group", "meta",
// "delta") and `top` ("white", "gray").
const gemTooltip = (o, { level = "meta", top = "white", req = true, fr = false } = {}) => {
  const yours = COMET;
  const white = top === "white";
  const line = (en, french, value) => fr ? `${french} : ${value}` : `${en}: ${value}`;
  const props = [
    level === "group" && { text: line("Level", "Niveau", o.level), value: o.level, yours: yours.level, max: white && o.level >= 20 },
    (o.quality || yours.quality) && { text: line("Quality", "Qualité", `${o.quality}%`), value: o.quality, yours: yours.quality,
      max: white && o.quality >= 20, lacking: !o.quality },
    { text: line("Sockets", "Châsses", o.sockets), value: o.sockets, yours: yours.sockets, max: white && o.sockets >= 5 },
  ].filter(Boolean);
  const meta = [
    fr ? "Gemmes d'aptitude" : "Skill Gems",
    level !== "group" && levelMeta(fr ? "Niveau" : "Level", o.level, yours.level, level === "delta"),
    o.corrupted && (fr ? "Corrompu" : "Corrupted"),
  ].filter(Boolean).join(' <span class="cls">·</span> ');
  const requirements = fr ? `Requiert niveau ${o.req[0]} · ${o.req[1]} Int` : `Requires Level ${o.req[0]} · ${o.req[1]} Int`;
  return { rarity: "gem", name: fr ? "Comète" : "Comet", price: o.price, currency: "ex", status: "online", meta, props,
    req: req ? requirements : undefined };
};
// `req`: the level and the intelligence it requires (sample data).
const HOVERED = { level: 16, quality: 0, sockets: 4, price: "6", req: [66, 116] };
const TOP_OFFER = { level: 21, quality: 23, sockets: 5, corrupted: true, price: "40", req: [90, 157] };
const AT_TWENTY = { level: 20, quality: 20, sockets: 3, price: "25", req: [87, 152] };
const GEM_ESTIMATE = { estimate: true, values: ["0.01", "0.06", "6"], offers: "27 similar", note: "Median of the 5 cheapest similar offers." };
const gemOffersWin = (tooltip, hover = 3) => win(topBar(), item(gem(COMET_COPIED)), price(GEM_ESTIMATE),
  offers({ count: "10 of 27 online", columns: ["Price", "Listed"], rows: GEM_OFFERS, hover, tooltip }), foot());
const gemOffersWinFr = tooltip => win(topBar(), item(gem({ ...COMET, name: "Comète", req: "Requiert niveau 52 · 101 Int", fr: true, filters: "2 filtres" })),
  price({ ...GEM_ESTIMATE, offers: "27 similaires", note: "Médiane des 5 offres similaires les moins chères." }, "fr"),
  offers({ count: "10 sur 27 en ligne", columns: ["Prix", "Publiée"], age: "recherche il y a 2min", rows: GEM_OFFERS.map(FR_WHEN), hover: 3, tooltip }, "fr"),
  foot("poe2scout il y a 12min · GGG trade il y a 2min"));

// The requirements of an offer of weapon or of armour, under its base (3 B): they rise with its modifiers.
const OFFER_JACKET = { rarity: "rare", name: "Gale Shell", base: "Rambler Jacket", cls: "Body Armours", tags: ["ilvl 81"],
  props: [["Evasion", "389"], ["Energy Shield", "112"]], req: "Requires Level 70 · 67 Dex · 67 Int", statsLabel: "Stats (6)", filters: "5 filters" };
const JACKET_OFFERS = [
  { v: "1", k: "ex", when: "2h ago" },
  { v: "1", k: "ex", when: "3h ago" },
  { v: "2", k: "ex", when: "1h ago" },
  { v: "3", k: "ex", when: "6h ago" },
  { v: "5", k: "ex", when: "1d ago" },
  { v: "8", k: "ex", when: "20min ago" },
];
const jacketTooltip = req => ({
  rarity: "rare", name: "Sorrow Shell", price: "3", currency: "ex", status: "online", meta: "Rambler Jacket · ilvl 80",
  req: req ? "Requires Level 70 · 67 Dex · 67 Int" : undefined,
  mods: [
    { text: "+92 to maximum Life", value: 92, yours: 85, tier: 3 },
    { text: "+38% to Fire Resistance", value: 38, yours: 41, tier: 2 },
    { text: "+31% to Cold Resistance", value: 31, yours: 33, tier: 3 },
    { text: "+24% to Lightning Resistance", value: 24, yours: 24, tier: 4 },
  ],
});
const BOW_OFFERS = [
  { v: "300", k: "ex", when: "1h ago", sus: true }, { v: "2", k: "div", when: "20min ago" }, { v: "2.2", k: "div", when: "4h ago" },
  { v: "2.4", k: "div", when: "1d ago", away: true }, { v: "2.5", k: "div", when: "2h ago" }, { v: "3", k: "div", when: "6h ago" }];
const bowTooltip = req => ({
  rarity: "rare", name: "Doom Song", price: "2.2", currency: "div", status: "online", meta: "Gemini Bow · ilvl 80",
  req: req ? "Requires Level 72 · 163 Dex" : undefined,
  mods: [
    { text: "148% increased Physical Damage", value: 148, yours: 152, tier: 2 },
    { text: "Adds 5 to 82 Lightning Damage", value: 43.5, yours: 39.5, tier: 3 },
    { text: "+3 to Level of all Projectile Skills", value: 3, yours: 3, tier: 1 },
    { text: "16% increased Attack Speed", value: 16, yours: 14, tier: 1 },
  ],
});
const RARE_PRICE = (values, count) => ({ estimate: true, values, offers: `${count} similar`, note: "Median of the 5 cheapest similar offers." });
const jacketWin = req => win(topBar(), item(OFFER_JACKET), price(RARE_PRICE(["0.01", "0.05", "3"], 24)),
  offers({ count: "6 of 24 online", columns: ["Price", "Listed"], rows: JACKET_OFFERS, hover: 3, tooltip: jacketTooltip(req) }), foot());
const bowWin = req => win(topBar(), item(BOW_RARE), price(RARE_PRICE(["2.4", "24", "1.7K"], 18)),
  offers({ count: "10 of 18 online", columns: ["Price", "Listed"], rows: BOW_OFFERS, hover: 2, tooltip: bowTooltip(req) }), foot());

SECTIONS.unshift(["Offer gem properties (13k5d2): 1 B, 2 A and 3 B chosen, the level under the name, the quality and the sockets with their difference, the requirements of gems, weapons and armours", [
  ["Chosen · 1 B · 2 A · 3 B, the level under the name in its color, the requirements under it", gemOffersWin(gemTooltip(HOVERED))],
  ["Chosen · corrupted, level 21, 23 %, 5 sockets: white at their top", gemOffersWin(gemTooltip(TOP_OFFER), 7)],
  ["Chosen · level 20, 20 %", gemOffersWin(gemTooltip(AT_TWENTY), 6)],
  ["Chosen · French", gemOffersWinFr(gemTooltip(HOVERED, { fr: true }))],
  ["Chosen · 3 B, an offer of body armour", jacketWin(true)],
  ["Chosen · 3 B, an offer of weapon", bowWin(true)],
  ["Not chosen · 1 A, the level in the group, with its difference", gemOffersWin(gemTooltip(HOVERED, { level: "group" }))],
  ["Not chosen · 1 C, the level under the name, its difference after it", gemOffersWin(gemTooltip(HOVERED, { level: "delta" }))],
  ["Not chosen · 2 B, gray at their top", gemOffersWin(gemTooltip(TOP_OFFER, { top: "gray" }), 7)],
  ["Not chosen · 3 A, without the requirements", gemOffersWin(gemTooltip(HOVERED, { req: false }))],
  ["Today · an offer of gem: the name written again, no property", gemOffersWin({ ...gemTooltip(HOVERED, { req: false }), meta: "Comet · ilvl 0", props: [] })],
  ["Today · an offer of body armour, no requirements", jacketWin(false)],
]]);
