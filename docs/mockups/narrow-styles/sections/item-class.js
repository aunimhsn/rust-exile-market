// Section Item class (13j3): the base unchecked searches the item class instead, a bow among all the bows
// (`type_filters.category` of the trade site), no longer any base. The class is the one of the copied text,
// in the language of the item, as in the head of the item block, in faint under the base (decided with the
// owner on 2026-10-28, against the class in place of the base, like Exiled Exchange 2, and a row of its own).
const ARMOUR_FR = { rarity: "rare", name: "Manteau du destin", base: "Vêtement gracieux", cls: "Armures", tags: ["Q 20%", "2 châsses", "niv. 82"], props: [["Bouclier d'énergie", "567"]], req: "Niveau requis 70 · 121 Int", statsLabel: "Statistiques (4)", filters: "4 filtres", tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" };
// The row of the base, `checked`; unchecked, the class searched in faint under it; `fr` for a French item.
function baseRow(base, cls, checked, fr) {
  const [baseLabel, searched] = fr ? ["Base :", "Classe cherchée :"] : ["Base:", "Class searched:"];
  return [{ mod: checked ? `${baseLabel} ${base}` : `${baseLabel} ${base}<br><span class="app-sub">${searched} ${cls}</span>`, check: checked }];
}
const bowClassFilters = checked => [
  { group: "Item", min: true },
  ...baseRow("Gemini Bow", "Bows", checked, false),
  { mod: "Item level: 81", check: false, min: 81 },
  { mod: "Sockets: 2", check: false, min: 2 },
  ...BOW_FILTERS.slice(4),
];
const armourClassFilters = checked => [
  { group: "Objet", min: true },
  ...baseRow("Vêtement gracieux", "Armures", checked, true),
  { mod: "Niveau de l'objet : 82", check: false, min: 82 },
  { mod: "Châsses : 2", check: false, min: 2 },
  { group: "Pseudo" },
  { mod: "+92 de Vie maximale au total", check: true, min: 82 },
  { group: "Préfixes", fill: "2 sur 3 · 1 libre" },
  { mod: "+92 de Vie maximale", check: false, min: 82, tier: 3, range: "85–99", share: .45 },
  { mod: "84% d'augmentation du Bouclier d'énergie", check: true, min: 75, tier: 2, range: "80–91", share: .6 },
  { group: "Suffixes", fill: "1 sur 3 · 2 libres" },
  { mod: "+31% de Résistance au froid", check: true, min: 27, tier: 3, range: "31–35", share: .1 },
];
SECTIONS.push(["Item class (13j3): the base unchecked searches its class", [
  ["Rare bow: base checked", win(topBar(), item({ ...BOW_RARE, filters: "4 filters", stats: bowClassFilters(true) }, true), foot())],
  ["The same, base unchecked: every bow", win(topBar(), item({ ...BOW_RARE, filters: "3 filters", stats: bowClassFilters(false) }, true), foot())],
  ["French item, base unchecked: every body armour", win(topBar(), item({ ...ARMOUR_FR, filters: "3 filtres", stats: armourClassFilters(false) }, true), foot())],
]]);
