// Section Main.
SECTIONS.push(["Main", [
  ["Main", win(topBar(), item(HH), price(HH_PRICE), offers(HH_OFFERS), foot())],
  ["Stats unfolded (filters)", win(topBar(), item({ ...HH, stats: HH_STATS }, true), foot())],
  ["Before the first search", win(topBar(), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot())],
  ["League list open", win(topBar(false, "Forbidden Rites", true), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot())],
  ["Settings open", win(topBar(true), settings, foot(`<span class="app-author">Exile Market 0.1.0 · Made by Sylent · <a href="https://sylent.art" target="_blank" rel="noopener">sylent.art</a></span>`))],
]]);
