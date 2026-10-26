// Section Base properties.
SECTIONS.push(["Base properties", [
  ["Rare body armour: armour and evasion", win(topBar(),
    item({ rarity: "rare", name: "Doom Shell", base: "Wanderer Coat", cls: "Body Armours", tags: ["Q 20%", "2 sockets", "ilvl 82"], props: [["Armour", "412"], ["Evasion", "380"]], req: "Requires Level 65 · 59 Str · 59 Dex", statsLabel: "Stats (6)" }),
    price(RARE_ESTIMATE_BEFORE), foot())],
  ["Unique focus: energy shield", win(topBar(),
    item({ rarity: "unique", name: "The Eternal Spark", base: "Crystal Focus", cls: "Foci", tags: ["ilvl 81"], props: [["Energy Shield", "44"]], req: "Requires Level 26 · 43 Int", statsLabel: "Stats (5)" }),
    price({ values: ["<0.01", "0.01", "1"], offers: "1655 offers", trend: "Stable over the last 5 days", stable: true }), foot())],
  ["Shield: armour and block", win(topBar(),
    item({ rarity: "rare", name: "Rune Wall", base: "Tawhoan Tower Shield", cls: "Shields", tags: ["ilvl 79"], props: [["Armour", "290"], ["Block", "26%"]], req: "Requires Level 70 · 121 Str", statsLabel: "Stats (5)" }),
    price(RARE_ESTIMATE_BEFORE), foot())],
]]);
