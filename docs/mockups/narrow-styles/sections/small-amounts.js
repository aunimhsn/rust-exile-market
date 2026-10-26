// Section Small amounts: the Divine and Chaos Orb amounts under 0.1 are hidden, the currencies left move to the left.
// Sample rates: 1 div = 10 c = 750 ex.
const ANDVARIUS = { rarity: "unique", name: "Andvarius", base: "Gold Ring", cls: "Rings", tags: ["ilvl 72"], req: "Requires Level 35", statsLabel: "Stats (4)" };
const GOLDRIM = { rarity: "unique", name: "Goldrim", base: "Feathered Tiara", cls: "Helmets", tags: ["ilvl 14"], props: [["Energy Shield", "29"]], req: "Requires Level 11", statsLabel: "Stats (4)" };
SECTIONS.push(["Small amounts", [
  ["0.4 div: all shown", win(topBar(), item(ANDVARIUS),
    price({ values: ["0.4", "4", "300"], offers: "12 offers", trend: "+8% over the last 7 days" }), foot())],
  ["0.08 div: Divine Orbs hidden", win(topBar(), item(GOLDRIM),
    price({ values: ["0.08", "0.8", "60"], offers: "31 offers", trend: "Stable over the last 7 days", stable: true }), foot())],
  ["1 ex: Divine and Chaos Orbs hidden", win(topBar(),
    item({ rarity: "unique", name: "The Eternal Spark", base: "Crystal Focus", cls: "Foci", tags: ["ilvl 81"], props: [["Energy Shield", "44"]], req: "Requires Level 26 · 43 Int", statsLabel: "Stats (5)" }),
    price({ values: ["<0.01", "0.01", "1"], offers: "1655 offers", trend: "Stable over the last 5 days", stable: true }), foot())],
  ["Stack: the unit price too", win(topBar(), item({ rarity: "currency", name: "Orb of Augmentation", cls: "Stackable Currency", tags: ["Stack 18/20"], statsLabel: "Stats (1)" }),
    price({ values: ["0.03", "0.27", "20"], stackValues: ["0.48", "4.8", "360"], stackSize: 18, trend: "+3% over the last 7 days" }), foot())],
]]);
