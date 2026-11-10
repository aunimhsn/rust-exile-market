// Section Thin prices and refused searches (17e3c), two decisions.
// 1. An item of the currency exchange whose price stands on fewer than 10 trades (355 of the 638 items of the
//    owner's cache on 2026-11-09): the application doesn't trust it to tell a suspicious offer, yet shows it in
//    large without a word, while a unique with fewer than 5 offers has its warning. A, the warning of a unique;
//    B, the warning and the amounts faint; C, no large price, the last trade on a small line.
// 2. A search the trade site turned down with an error (a stat id renamed by a patch is the likely one after
//    Path of Exile 2 1.0), or answered with something the application can't read, shows today "The trade site
//    can't be reached.", which tells the player to try again. A, its own words in the title row; B, the words in
//    yellow and what to do on a line under the row, "report it" opening the bug report; C, B with the reason the
//    trade site gives.
// The owner chose 1 A and 2 B on 2026-11-10: the warning a player knows already, where B or C would cast doubt on
// more than half of the prices of the currency exchange; and what to do, since unchecking a filter is the one
// thing a player can do against a stat renamed by a patch, the reason of the trade site going to the log only.
// A page of the next offers that can't be read says so above the chevron, without the hint.
// With them, two French words of the search: "Cherchez" as the four other messages, and "prochaine recherche
// dans" as the English "next search in".

const RUNE = { rarity: "currency", name: "Lesser Charging Rune", cls: "Socketables", tags: ["Stack 3/10"], statsLabel: "Stats (1)" };
const RUNE_FR = { rarity: "currency", name: "Rune de charge inférieure", cls: "Objets de châsse", tags: ["Pile 3/10"], statsLabel: "Statistiques (1)" };
const CHANCE = { rarity: "currency", name: "Orb of Chance", cls: "Stackable Currency", tags: ["Stack 1/20"], statsLabel: "Stats (1)" };
// One trade, absurd: 40,156 ex, while the sellers ask 4 to 6 ex.
const RUNE_PRICE = { values: ["53.5", "535", "40.2K"], stackValues: ["161", "1.6K", "120K"], stackSize: 3, trend: "+2,140% over the last 2 days" };
const RUNE_OFFERS = (fr = false) => ({
  count: fr ? "5 sur 9 en ligne" : "5 of 9 online", columns: fr ? ["Par unité", "Stock", "Publiée"] : ["Per unit", "Stock", "Listed"],
  age: fr ? "recherche il y a 2min" : undefined, rows: [
    { v: "4", k: "ex", stock: "12", when: fr ? "20min" : "20min ago", name: fr ? "Orbe exalté" : "Exalted Orb" },
    { v: "4", k: "ex", stock: "3", when: fr ? "2h" : "2h ago", name: fr ? "Orbe exalté" : "Exalted Orb" },
    { v: "5", k: "ex", stock: "40", when: fr ? "1j" : "1d ago", name: fr ? "Orbe exalté" : "Exalted Orb", away: true },
    { v: "6", k: "ex", stock: "7", when: fr ? "3h" : "3h ago", name: fr ? "Orbe exalté" : "Exalted Orb" },
    { v: "6", k: "ex", stock: "25", when: fr ? "5j" : "5d ago", name: fr ? "Orbe exalté" : "Exalted Orb" },
  ] });
// Six trades, a likely price.
const CHANCE_PRICE = { values: ["0.07", "0.7", "52"], trend: "−6% over the last 7 days", rising: false };

const THIN = { en: n => n === 1 ? "Only 1 trade: the price is uncertain" : `Only ${n} trades: the price is uncertain`,
  fr: n => n === 1 ? "Un seul échange : le prix est incertain" : `Seulement ${n} échanges : le prix est incertain` };
// B: the amounts faint, the orbs too.
const faint = html => html.replace('class="prices"', 'class="prices app-thin"');
// C: no large price; the last trade small, under the warning.
const lastTrade = (n, values) => ({ lines: [n === 1 ? "Only 1 trade: too few for a price." : `Only ${n} trades: too few for a price.`,
  `Last trades: ${values[0]} div · ${values[1]} c · ${values[2]} ex`] });

const thinWin = (it, p, offersBlock) => win(topBar(), item(it), p, offersBlock ?? offers({ state: "idle" }), foot());

// The title row of the offers, as before the first search, with the line `line` of the kind `kind` and,
// under the row, the hint `hint`.
const refusedRow = (line, kind, hint, fr = false) => {
  const action = fr ? "Chercher les offres" : "Search the offers";
  return `<div class="band b"><div class="head flat-head idle-head"><span class="title-stack"><span class="label">${fr ? "Offres" : "Offers"}</span><span class="age ${kind}">${line}</span></span><div class="search-block" title="${action}" aria-label="${action}">${ICON.search(22)}</div></div>${hint ? `<div class="app-offers-hint">${hint}</div>` : ""}</div>`;
};
const REPORT = (words, fr = false) => `<span class="link" title="${fr ? "Signaler un bug" : "Report a bug"}">${words}</span>`;
const REFUSED = {
  en: { line: "The trade site refused this search.", hint: `Uncheck a filter and search again. If it happens again, ${REPORT("report it")}.` },
  fr: { line: "Le site d'échange a refusé cette recherche.", hint: `Décochez un filtre et cherchez à nouveau. Si cela se répète, ${REPORT("signalez-le", true)}.` },
};
const UNREADABLE = {
  en: { line: "The answer of the trade site can't be read.", hint: `The game may have changed. ${REPORT("Report it")}, so that Exile Market can be fixed.` },
  fr: { line: "La réponse du site d'échange est illisible.", hint: `Le jeu a peut-être changé. ${REPORT("Signalez-le", true)}, pour qu'Exile Market soit corrigé.` },
};
const refusedWin = (offersBlock, fr = false) => win(topBar(), item(fr ? { ...BOW_RARE, cls: "Arcs" } : BOW_RARE),
  price(fr ? { lines: ["Pas de prix de marché pour les objets rares.", "Cherchez des offres similaires pour l'estimer."] } : RARE_ESTIMATE_BEFORE, fr ? "fr" : "en"),
  offersBlock, foot(fr ? "poe2scout il y a 12min · GGG trade il y a 2min" : undefined));
// The next offers of a search whose page can't be read: the note above the chevron says it.
const unreadableMore = (note) => offers({ ...HH_OFFERS, more: "failed" }).replace("The trade site can't be reached.", note);

SECTIONS.unshift(["Thin prices and refused searches (17e3c): 1 A and 2 B chosen, and two French words", [
  ["1 · Today · 1 trade, shown in large without a word", thinWin(RUNE, price(RUNE_PRICE), offers(RUNE_OFFERS())) ],
  ["1 · Chosen · A · the warning of a unique", thinWin(RUNE, price({ ...RUNE_PRICE, warning: THIN.en(1) }), offers(RUNE_OFFERS()))],
  ["1 · Chosen · A · 6 trades, a likely price", thinWin(CHANCE, price({ ...CHANCE_PRICE, warning: THIN.en(6) }))],
  ["1 · Chosen · A · en français", win(topBar(), item(RUNE_FR), price({ ...RUNE_PRICE, trend: "+2140 % sur les 2 derniers jours", warning: THIN.fr(1) }, "fr")
    .replace("Unit price", "Prix unitaire").replace("Stack of 3", "Pile de 3"), offers(RUNE_OFFERS(true), "fr"), foot("poe2scout il y a 12min · GGG trade il y a 2min"))],
  ["1 · Not chosen · B · the warning, the amounts faint", thinWin(RUNE, faint(price({ ...RUNE_PRICE, warning: THIN.en(1) })), offers(RUNE_OFFERS()))],
  ["1 · Not chosen · B · 6 trades", thinWin(CHANCE, faint(price({ ...CHANCE_PRICE, warning: THIN.en(6) })))],
  ["1 · Not chosen · C · no large price, the last trades small", thinWin(RUNE, price({ ...lastTrade(1, RUNE_PRICE.values), offers: "Stack of 3" }), offers(RUNE_OFFERS()))],
  ["1 · Not chosen · C · 6 trades", thinWin(CHANCE, price(lastTrade(6, CHANCE_PRICE.values)))],
  ["2 · Today · a refused search reads as a network failure", refusedWin(offers({ state: "unreachable" }))],
  ["2 · Not chosen · A · refused: its own words, in gray", refusedWin(refusedRow(REFUSED.en.line, "plain"))],
  ["2 · Not chosen · A · unreadable", refusedWin(refusedRow(UNREADABLE.en.line, "plain"))],
  ["2 · Chosen · the next offers, their page unreadable: its own words above the chevron, without the hint", win(topBar(), item(HH), price(HH_PRICE), unreadableMore(UNREADABLE.en.line), foot())],
  ["2 · Chosen · B · refused: in yellow, what to do under it", refusedWin(refusedRow(REFUSED.en.line, "warn", REFUSED.en.hint))],
  ["2 · Chosen · B · unreadable", refusedWin(refusedRow(UNREADABLE.en.line, "warn", UNREADABLE.en.hint))],
  ["2 · Chosen · B · refusé, en français", refusedWin(refusedRow(REFUSED.fr.line, "warn", REFUSED.fr.hint, true), true)],
  ["2 · Chosen · B · illisible, en français", refusedWin(refusedRow(UNREADABLE.fr.line, "warn", UNREADABLE.fr.hint, true), true)],
  ["2 · Not chosen · C · refused, with the reason of the trade site", refusedWin(refusedRow(REFUSED.en.line, "warn",
    `<span class="app-offers-reason">“Unknown stat id: explicit.stat_3299347043”</span>${REFUSED.en.hint}`))],
  ["Chosen · French words · « Cherchez », as the four other messages", refusedWin(offers({ state: "idle" }, "fr"), true)],
  ["Chosen · French words · « prochaine recherche dans », as in English", win(topBar(), item(HH), price(HH_PRICE),
    refusedRow("Limite du site d'échange : prochaine recherche dans 4min.", "warn", "", true)
      .replace(/<div class="search-block"[^]*?<\/svg><\/div>/, `<div class="again waiting" title="Prochaine recherche dans 4min">4min</div>`),
    foot("poe2scout il y a 12min · GGG trade\u00a0: prochaine recherche dans 4min"))],
]]);
