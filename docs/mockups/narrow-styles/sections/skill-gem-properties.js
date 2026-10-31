// Section Skill gem properties (13k5d1): the level, the quality and the sockets of a skill gem, which make
// its price while the application searches it by its name only today (`Flame Wall`, the owner's gem of level
// 2), and doesn't show its level. The trade site searches them (`/data/filters`, checked on 2026-11-01:
// `misc_filters` `gem_level` and `gem_sockets`, `type_filters` `quality`). Four searches on 2026-11-01 (Runes
// of Aldur): levels up to 21 (20 from the gem, "+1 Level from Corruption"), none above; up to 5 sockets, none
// above, and 2 at least even at level 20; quality up to 23 % (corrupted). The fields stay within them: the
// level from 1 to 21, the quality from 0 to 23 %, the sockets from 0 to 5.
// Exiled Exchange 2 (commit cca3066) searches the name, the corrupted state, and checks the level from 19, the
// quality from 16 % and the sockets from 3, each with its value as minimum.
// The filters go in a group of their own, "Gem" (French "Gemme"), like the "Waystone" group: "Level: 15",
// "Quality: 12%", "Sockets: 3" (French "Niveau : 15", "Qualité : 12%", "Châsses : 3"); no filter of the
// base: the name of the gem is always searched.
// Decision 1, where the level is in the item block: A, a tag, first of the numbers ("Level 2", French "Niveau
// 2"); B, after the item class, under the name ("Skill Gems · Level 2"), as the tier of a waystone; C, as B,
// its color telling the top levels: 20 (the highest from the gem) in the orange of tier 1, 21 (corrupted) in
// the corrupted red, as the tiers of a waystone.
// Decision 2, what is checked: A, as Exiled Exchange 2; B, the level always, the quality from 16 %, the
// sockets from 3 (2 is what every gem has): a gem of level 15 searched by its name alone is compared with gems
// of level 1 sold for nothing; C, every property.
// Decision 3, the price block of a skill gem, which no price database lists: A, today, "No price found for
// this item in Forbidden Rites.", as if the data missed it; B, "No market price for skill gems." and "Search
// similar offers to estimate it.", as a rare item.
// Decision 4, the owner's idea: the tags of the quality and the sockets at their top, in the colors of the
// level: A, plain, as today; B, the quality, 20 % (the highest without corruption) in orange, 21 to 23 %
// (corrupted) in red; C, as B, and 5 sockets (the most, also corrupted) in orange. Not for the other items:
// 20 % of quality is usual on most endgame items, its color would tell nothing.
// Chosen by the owner on 2026-11-01: 1 C, 2 B, 3 B, 4 C; the other options kept below, for the record.

// The base line of a gem: its item class, then its level in `style` ("tag": not there, a tag in its place).
const gemBase = (cls, word, level, style) => {
  if (style === "tag") return undefined;
  const color = style === "color" ? (level >= 21 ? " is-top" : level >= 20 ? " is-high" : "") : "";
  return `<span class="cls">${cls} ·</span> <span class="app-waystone-tier${color}">${word} ${level}</span>`;
};
// The tags of a gem: its states, then its level in decision 1 A, its quality and its sockets in the colors of
// decision 4 (`colors`: "plain", "quality" or "both").
const gemTags = ({ level, quality, sockets, states = [], fr = false }, style, colors) => {
  const qualityColor = colors === "plain" ? "" : quality > 20 ? "app-tag-top" : quality === 20 ? "app-tag-high" : "";
  const socketsColor = colors === "both" && sockets >= 5 ? "app-tag-high" : "";
  return [
    ...states,
    style === "tag" && `${fr ? "Niveau" : "Level"} ${level}`,
    quality && { label: `Q ${quality}%`, cls: qualityColor },
    { label: `${sockets} ${fr ? "châsses" : "sockets"}`, cls: socketsColor },
  ].filter(Boolean);
};
// A skill gem in the level `style` of decision 1 and the `colors` of decision 4.
const gem = (it, style = "color", colors = "both") => ({
  rarity: "gem", name: it.name, cls: it.fr ? "Gemmes d'aptitude" : "Skill Gems", noClass: true,
  base: gemBase(it.fr ? "Gemmes d'aptitude" : "Skill Gems", it.fr ? "Niveau" : "Level", it.level, style),
  tags: gemTags(it, style, colors), req: it.req, statsLabel: it.fr ? "Statistiques (0)" : "Stats (0)", filters: it.filters,
});
const FLAME_WALL = { name: "Flame Wall", level: 2, quality: 0, sockets: 2, req: "Requires Level 3 · 9 Int" };
const FLAME_WALL_FR = { ...FLAME_WALL, name: "Mur de flammes", req: "Requiert niveau 3 · 9 Int", fr: true };
const COMET = { name: "Comet", level: 15, quality: 12, sockets: 3, req: "Requires Level 52 · 101 Int" };
const FLICKER = { name: "Flicker Strike", level: 21, quality: 23, sockets: 5, states: [state("Twice Corrupted", "twice", false)],
  req: "Requires Level 90 · 86 Dex · 86 Int" };
const FLAME_WALL_LEVEL_19 = { ...FLAME_WALL, level: 19, quality: 17, sockets: 3, req: "Requires Level 62 · 115 Int" };
const FLAME_WALL_LEVEL_20 = { ...FLAME_WALL, level: 20, quality: 20, sockets: 4, req: "Requires Level 65 · 121 Int" };
const COMET_LEVEL_20 = { ...COMET, level: 20, quality: 20, sockets: 5, req: "Requires Level 65 · 121 Int" };

// The Gem group: `[text, value, checked]` rows.
const gemStats = (rows, fr = false) => [
  { group: fr ? "Gemme" : "Gem", min: true },
  ...rows.map(([mod, value, check]) => ({ mod, check, min: value })),
];
const filterText = (en, french, value, fr) => fr ? `${french} : ${value}` : `${en}: ${value}`;
// The rows of a gem in the rule `rule` of decision 2.
const checkedBy = (rule, { level, quality, sockets }) => ({
  level: rule === "ee2" ? level >= 19 : true,
  quality: rule === "all" ? true : quality >= 16,
  sockets: rule === "all" ? true : sockets >= 3,
});
const gemRows = (rule, it) => {
  const checked = checkedBy(rule, it);
  return [
    [filterText("Level", "Niveau", it.level, it.fr), it.level, checked.level],
    it.quality ? [filterText("Quality", "Qualité", `${it.quality}%`, it.fr), it.quality, checked.quality] : null,
    [filterText("Sockets", "Châsses", it.sockets, it.fr), it.sockets, checked.sockets],
  ].filter(Boolean);
};
const filterWord = (count, fr) => `${count} ${fr ? "filtre" : "filter"}${count > 1 ? "s" : ""}`;
// The price block of decision 3.
const GEM_PRICE_TODAY = { lines: ["No price found for this item in Forbidden Rites."] };
const GEM_PRICE = { lines: ["No market price for skill gems.", "Search similar offers to estimate it."] };
const GEM_PRICE_FR = { lines: ["Pas de prix de marché pour les gemmes d'aptitude.", "Recherchez des offres similaires pour l'estimer."] };

// A gem window, folded, its level in `style`, its tags in `colors`.
const gemFolded = (it, style = "color", colors = "both") => win(topBar(), item(gem(it, style, colors)), price(GEM_PRICE), foot());
// A gem window, unfolded, in the rule `rule`.
const gemUnfolded = (it, rule, { priced } = {}) => {
  const rows = gemRows(rule, it);
  const count = rows.filter(([, , check]) => check).length;
  return win(topBar(), item({ ...gem({ ...it, filters: filterWord(count, it.fr) }), stats: gemStats(rows, it.fr) }, true),
    price(priced ?? (it.fr ? GEM_PRICE_FR : GEM_PRICE), it.fr ? "fr" : "en"), offers({ state: "idle" }, it.fr ? "fr" : "en"),
    foot(it.fr ? "poe2scout il y a 12min · GGG trade il y a 2min" : undefined));
};
const today = it => win(topBar(), item({ ...gem(it, "tag", "plain"), tags: [`${it.sockets} sockets`], base: undefined }),
  price(GEM_PRICE_TODAY), offers({ state: "idle" }), foot());

SECTIONS.unshift(["Skill gem properties (13k5d1): 1 C, 2 B, 3 B and 4 C chosen, the level, the quality and the sockets in the colors of their top", [
  ["Chosen · 1 C · 2 B · 3 B · 4 C, the owner's gem", gemUnfolded(FLAME_WALL, "level")],
  ["Chosen · a gem of level 15: its level checked", gemUnfolded(COMET, "level")],
  ["Chosen · corrupted, its state tag gray at start, as every item", gemUnfolded(FLICKER, "level")],
  ["Chosen · French", gemUnfolded(FLAME_WALL_FR, "level")],
  ["Chosen · folded, level 21, 23 % and 5 sockets", gemFolded(FLICKER)],
  ["Chosen · level 20, 20 %, 5 sockets", gemFolded(COMET_LEVEL_20)],
  ["Chosen · level 20, 20 %, 4 sockets", gemFolded(FLAME_WALL_LEVEL_20)],
  ["Chosen · level 19, 17 %, 3 sockets: white", gemFolded(FLAME_WALL_LEVEL_19)],
  ["Not chosen · 1 A, the level as a tag, first of the numbers", gemFolded(FLAME_WALL, "tag")],
  ["Not chosen · 1 B, the level after the item class, plain", gemFolded(FLICKER, "plain")],
  ["Not chosen · 2 A, as Exiled Exchange 2: the level from 19, the quality from 16 %, the sockets from 3", gemUnfolded(COMET, "ee2")],
  ["Not chosen · 2 C, every property", gemUnfolded(COMET, "all")],
  ["Not chosen · 3 A, today: no price found, as if the data missed it", gemUnfolded(FLAME_WALL, "level", { priced: GEM_PRICE_TODAY })],
  ["Not chosen · 4 A, plain", gemFolded(FLICKER, "color", "plain")],
  ["Not chosen · 4 B, the quality alone", gemFolded(FLICKER, "color", "quality")],
  ["Today · no level, searched by its name only", today(FLAME_WALL)],
]]);
