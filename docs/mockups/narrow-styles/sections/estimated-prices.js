// Section Estimated prices: rare, magic, normal, jewels.
SECTIONS.push(["Estimated prices: rare, magic, normal, jewels", [
  ["Rare bow: before the search", win(topBar(), item(BOW_RARE), price(RARE_ESTIMATE_BEFORE), offers({ state: "idle" }), foot())],
  ["Rare bow: filters", win(topBar(), item({ ...BOW_RARE, stats: BOW_FILTERS }, true), foot())],
  ["Rare bow: estimated price", win(topBar(), item(BOW_RARE),
    price({ estimate: true, values: ["2.4", "24", "1.7K"], offers: "18 similar", note: "Median of the 5 cheapest similar offers." }),
    offers({ count: "10 of 18 online", columns: ["Price", "Listed"], rows: [
      { v: "300", k: "ex", when: "1h ago", sus: true }, { v: "2", k: "div", when: "20min ago" }, { v: "2.2", k: "div", when: "4h ago" },
      { v: "2.4", k: "div", when: "1d ago", away: true }, { v: "2.5", k: "div", when: "2h ago" }, { v: "3", k: "div", when: "6h ago" }] }), foot())],
  ["Rare bow: offer hovered", win(topBar(), item(BOW_RARE),
    price({ estimate: true, values: ["2.4", "24", "1.7K"], offers: "18 similar", note: "Median of the 5 cheapest similar offers." }),
    offers({ count: "10 of 18 online", columns: ["Price", "Listed"], hover: 2, tooltip: {
      rarity: "rare", name: "Doom Song", price: "2.2", currency: "div", meta: "Gemini Bow · ilvl 80 · Q 20% · 2 sockets", status: "online",
      props: [
        { text: "Physical DPS 238", value: 238, yours: 245 },
        { text: "Elemental DPS 51", value: 51, yours: 44 },
      ],
      mods: [
        { text: "148% increased Physical Damage", value: 148, yours: 152, tier: 2 },
        { text: "Adds 5 to 82 Lightning Damage", tier: 3 },
        { text: "+3 to Level of all Projectile Skills", value: 3, yours: 3, tier: 1 },
        { text: "16% increased Attack Speed", value: 16, yours: 14, tier: 1 },
        { text: "+290 to Accuracy Rating", value: 290, yours: 312, tier: 5 },
        { text: "+18% to Critical Damage Bonus", tier: 3 },
      ],
    }, rows: [
      { v: "300", k: "ex", when: "1h ago", sus: true }, { v: "2", k: "div", when: "20min ago" }, { v: "2.2", k: "div", when: "4h ago" },
      { v: "2.4", k: "div", when: "1d ago", away: true }, { v: "2.5", k: "div", when: "2h ago" }, { v: "3", k: "div", when: "6h ago" }] }), foot())],
  ["Rare ring: pseudo stats", win(topBar(), item({ ...RING, stats: RING_FILTERS }, true), foot())],
  ["Magic item: few similar offers", win(topBar(), item(MAGIC),
    price({ estimate: true, values: ["0.1", "1", "70"], offers: "3 similar", warning: "Only 3 similar offers: the estimate is uncertain", note: "Median of the 3 similar offers." }),
    offers({ count: "3 of 3 online", columns: ["Price", "Listed"], rows: [
      { v: "50", k: "ex", when: "2d ago" }, { v: "70", k: "ex", when: "5h ago" }, { v: "1", k: "chaos", when: "1h ago" }] }), foot())],
  ["Rare jewel: estimated price", win(topBar(), item(JEWEL),
    price({ estimate: true, values: ["1.2", "12", "870"], offers: "9 similar", note: "Median of the 5 cheapest similar offers." }),
    offers({ count: "9 of 9 online", columns: ["Price", "Listed"], rows: [
      { v: "1", k: "div", when: "3h ago" }, { v: "1.1", k: "div", when: "40min ago" }, { v: "1.2", k: "div", when: "1d ago" },
      { v: "1.5", k: "div", when: "2h ago" }, { v: "2", k: "div", when: "5h ago" }] }), foot())],
  ["Rare jewel: filters", win(topBar(), item({ ...JEWEL, stats: JEWEL_FILTERS }, true), foot())],
  ["Exceptional base: filters", win(topBar(), item({ ...STAFF, stats: STAFF_FILTERS }, true), foot())],
  ["Exceptional base: estimated price", win(topBar(), item(STAFF),
    price({ estimate: true, values: ["0.6", "6", "435"], offers: "11 similar", note: "Median of the 5 cheapest similar offers." }),
    offers({ count: "10 of 11 online", columns: ["Price", "Listed"], rows: [
      { v: "350", k: "ex", when: "2h ago" }, { v: "0.5", k: "div", when: "1d ago" }, { v: "0.6", k: "div", when: "30min ago" },
      { v: "0.7", k: "div", when: "4h ago", away: true }, { v: "1", k: "div", when: "3d ago" }] }), foot())],
]]);
