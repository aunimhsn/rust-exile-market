// Section Stat on several lines (fix): the game writes some stats on two lines or more, a line break in its
// own text (the owner's Kurgal focus, French, 2026-11-01: `Lorsque vous collectez une Infusion élémentaire,
// vous avez 23(15-25)% de chances de gagner une` then `Infusion élémentaire supplémentaire du même type`;
// 66 English and 52 French stats of two lines with one number, 11 of three lines in both). The application
// showed each line as a row: the first with the filter and its bar, the next one under the bar, from the left
// edge, as a line of its own, so that the bar cut the sentence. Fixed: the lines of one stat make one text,
// joined by a space, its bar, tier and origin under it, as this sheet always drew it ("A lord of the Abyss:
// one tier only", section Endgame modifiers). A stat with a number on two of its lines (three stats of one
// unique, `Alternating every 5 seconds:`) keeps a row per line, each with its own bar.
// The text of a stat now wraps on four lines at most, instead of two, cut with "…" past them and whole in
// its tooltip (owner, 2026-11-01: more lines are fine, the elements must never mix; four chosen against
// three, which cut this French sentence at the default width of 420 px).

const FOCUS_KURGAL_FR = { ...FOCUS_KURGAL, name: "Hymne du crâne", base: "Focus sacré", props: [["Bouclier d'énergie", "167"]], req: "Prérequis : Niveau 75 · 107 Int" };
const kurgalFrHead = [
  { group: "Item", min: true },
  { mod: "Base: Focus sacré", check: true },
  { mod: "Item level: 79", check: false, min: 79 },
  { group: "Prefixes", fill: "3 of 3" },
  { mod: "38% d'Augmentation du Bouclier d'énergie", check: true, min: 34, over: { share: .9 } },
  { mod: "+27 de Mana maximal", check: false, min: 24, over: { share: .6 } },
  { rollOnly: true, tier: 2, range: "33–38 · 27–32" },
  { mod: "+40 de Bouclier d'énergie maximal", check: true, min: 36, tier: 6, range: "36–41", over: { share: .4 } },
  { mod: "+32 de Mana maximal", check: false, min: 29, tier: 9, range: "25–34", over: { share: .2 } },
  { group: "Suffixes", fill: "3 of 3" },
  { mod: "+37% à la Résistance au froid", check: false, min: 33, tier: 2, range: "36–40", over: { share: .78 } },
  { mod: "+17 d'Intelligence", check: false, min: 15, tier: 5, range: "17–20", over: { share: .45 } },
];
const kurgalFrBefore = [
  ...kurgalFrHead,
  { mod: "Lorsque vous collectez une Infusion élémentaire, vous avez 23% de chances de gagner une", check: true, min: 21, over: { share: .8 } },
  { mod: "Infusion élémentaire supplémentaire du même type", single: true, range: "15–25", noBar: true, origin: "desecrated" },
];
const kurgalFrAfter = [
  ...kurgalFrHead,
  { mod: "Lorsque vous collectez une Infusion élémentaire, vous avez 23% de chances de gagner une Infusion élémentaire supplémentaire du même type", check: true, min: 21, single: true, range: "15–25", share: .8, origin: "desecrated" },
];
// Past four lines, the text is cut with "…" and shown whole in its tooltip.
const kurgalLongStats = [
  ...kurgalFrHead,
  { mod: "Lorsque vous collectez une Infusion élémentaire, vous avez 23% de chances de gagner une Infusion élémentaire supplémentaire du même type, et une autre phrase assez longue pour passer la quatrième ligne, puisque la phrase du jeu en prend déjà trois ou quatre selon la largeur", check: true, min: 21, single: true, range: "15–25", share: .8, origin: "desecrated" },
];

SECTIONS.unshift(["Stat on several lines (fix): one text on four lines at most, its bar under it", [
  ["Before: French, the bar between the two lines", win(topBar(), item({ ...FOCUS_KURGAL_FR, stats: kurgalFrBefore }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["After: French, one text on four lines", win(topBar(), item({ ...FOCUS_KURGAL_FR, stats: kurgalFrAfter }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["After: English, one text", win(topBar(), item({ ...FOCUS_KURGAL, stats: focusKurgalStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["After: a text past four lines, cut with \"…\"", win(topBar(), item({ ...FOCUS_KURGAL_FR, stats: kurgalLongStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
