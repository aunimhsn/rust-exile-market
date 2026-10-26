// Section Special cases.
SECTIONS.push(["Special cases", [
  ["Currency stack", win(topBar(), item({ rarity: "currency", name: "Orb of Annulment", cls: "Stackable Currency", tags: ["Stack 7/20"], statsLabel: "Stats (1)" }),
    price({ values: ["0.25", "2.49", "182"], stackValues: ["1.76", "17.4", "1.3K"], stackSize: 7, rising: false, trend: "−11% over the last 7 days" }),
    offers({ count: "8 of 48 online", columns: ["Per unit", "Stock", "Listed"], rows: [
      { v: "175", k: "ex", stock: "40", when: "12min" }, { v: "178", k: "ex", stock: "12", when: "3min" }, { v: "180", k: "ex", stock: "120", when: "45min" },
      { v: "182", k: "ex", stock: "33", when: "3h" }, { v: "185", k: "ex", stock: "60", when: "9min" }] }), foot())],
  ["Unidentified unique", win(topBar(), item({ rarity: "unique", name: "Stellar Amulet", cls: "Amulets", tags: ["Unidentified", "ilvl 74"], req: "Requires Level 60", statsLabel: "Stats (1)" }),
    price({ candidates: [["Astramentis", "4.0", "div"], ["Hinekora's Sight", "640", "ex"], ["Fixation of Yix", "95", "ex"], ["Strugglescream", null]] }),
    offers({ lines: ["Identify the item to search its offers."] }), foot())],
  ["One orb, stable price", win(topBar(), item({ rarity: "currency", name: "Divine Orb", cls: "Stackable Currency", tags: ["Stack 1/20"], statsLabel: "Stats (1)" }),
    price({ values: ["1", "10", "750"], trend: "Stable over the last 7 days", stable: true }), offers({ state: "idle" }), foot())],
  ["Prices downloading", win(topBar(), item({ rarity: "currency", name: "Orb of Annulment", cls: "Stackable Currency", tags: ["Stack 7/20"], statsLabel: "Stats (1)" }),
    price({ lines: ["The prices of Forbidden Rites are downloading…"] }), offers({ state: "idle" }), foot("poe2scout: downloading prices… · GGG trade: no search yet"))],
  ["French item (interface in French)", win(topBar(), item({ rarity: "unique", name: "Chasseur de têtes", base: "Ceinture lourde", cls: "Ceintures", tags: ["Corrompu", "niv. 80"], req: "Requiert niveau 50", statsLabel: "Statistiques (6)", filters: "3 filtres", tradeLabel: "Voir sur l'échange", tradeTitle: "Ouvrir la recherche sur le site d'échange" }),
    price({ values: ["12.4", "123", "9K"], offers: "23 offres", trend: "+4% sur les 3 derniers jours" }, "fr"), offers({ state: "idle" }, "fr"), foot("poe2scout il y a 12min · GGG trade il y a 2min"))],
]]);
