// Section Other items.
SECTIONS.push(["Other items", [
  ["Unique bow: corrupted, quality, sockets", win(topBar(), item({ rarity: "unique", name: "Death's Harp", base: "Dualstring Bow", cls: "Bows", tags: ["Corrupted", "Q 20%", "2 sockets", "ilvl 79"], props: [["Physical DPS", "198"]], req: "Requires Level 52 · 96 Dex", statsLabel: "Stats (5)", filters: "2 filters" }),
    price({ values: ["1.8", "18", "1.3K"], offers: "4 offers", warning: "Only 4 offers: the price is uncertain" }),
    offers({ count: "4 of 4 online", columns: ["Price", "Listed"], rows: [{ v: "1.5", k: "div", when: "5h ago" }, { v: "1.8", k: "div", when: "1d ago" }, { v: "2", k: "div", when: "50min ago" }, { v: "2.5", k: "div", when: "3d ago" }] }), foot())],
  ["Many tags (they wrap)", win(topBar(), item({ rarity: "unique", name: "Ironbound", base: "Runeforged Warden Bow", cls: "Bows", tags: ["Twice Corrupted", "Sanctified", "Fractured", "Mirrored", "Q 23%", "3 sockets", "ilvl 82"], props: [["Physical DPS", "312"]], req: "Requires Level 70 · 172 Dex", statsLabel: "Stats (6)", filters: "3 filters" }),
    price({ values: ["38", "376", "27.6K"], offers: "2 offers", warning: "Only 2 offers: the price is uncertain" }), offers({ state: "idle" }), foot())],
]]);
