// Section Offer states.
SECTIONS.push(["Offer states", [
  ["Searching the offers", win(topBar(), item(HH), price(HH_PRICE), offers({ state: "loading" }), foot())],
  ["Offer hovered", win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, hover: 3, tooltip: {
    name: "Headhunter", price: "10", currency: "div", meta: "Heavy Belt · ilvl 82 · Corrupted", status: "online",
    implicit: [{ text: "25% increased Stun Threshold", value: 25, yours: 26 }, { text: "Has 2 Charm Slots", value: 2, yours: 2 }],
    mods: [
      { text: "+48 to maximum Life", value: 48, yours: 55 },
      { text: "+40 to Strength", value: 40, yours: 34 },
      { text: "+23 to Dexterity", value: 23, yours: 22 },
      { text: "When you kill a Rare monster, you gain its Modifiers for 60 seconds" },
    ],
  } }), foot())],
  ["Suspicious offer hovered (opens below)", win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, hover: 0, tooltip: {
    name: "Headhunter", price: "6", currency: "ex", meta: "Heavy Belt · ilvl 78 · Corrupted", status: "online", below: true, suspicious: true,
    implicit: [{ text: "22% increased Stun Threshold", value: 22, yours: 26 }, { text: "Has 1 Charm Slot", value: 1, yours: 2 }],
    mods: [
      { text: "+52 to maximum Life", value: 52, yours: 55 },
      { text: "+39 to Strength", value: 39, yours: 34 },
      { text: "+35 to Dexterity", value: 35, yours: 22 },
      { text: "When you kill a Rare monster, you gain its Modifiers for 60 seconds" },
    ],
  } }), `<div class="b" style="height:190px"></div>`, foot())],
  ["Offer pinned, whisper copied", win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, hover: 4, tooltip: {
    name: "Headhunter", price: "11", currency: "div", meta: "Heavy Belt · ilvl 80 · Corrupted", status: "away", pinned: true, copied: true,
    implicit: [{ text: "28% increased Stun Threshold", value: 28, yours: 26 }, { text: "Has 2 Charm Slots", value: 2, yours: 2 }],
    mods: [
      { text: "+53 to maximum Life", value: 53, yours: 55 },
      { text: "+36 to Strength", value: 36, yours: 34 },
      { text: "+30 to Dexterity", value: 30, yours: 22 },
      { text: "When you kill a Rare monster, you gain its Modifiers for 60 seconds" },
    ],
  } }), foot())],
  ["Loading more offers", win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, more: "loading" }), foot())],
  ["More offers that couldn't come", win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, more: "failed" }), foot())],
  ["Ready to search again", win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, ready: true, age: "searched 5min ago" }), foot("poe2scout 12min ago · GGG trade 5min ago"))],
  ["Trade site unreachable", win(topBar(), item(HH), price(HH_PRICE), offers({ state: "unreachable" }), foot())],
  ["Long wait before the next search", win(topBar(), item(HH), price(HH_PRICE), offers({ state: "wait" }), foot("poe2scout 12min ago · GGG trade: next search in 4min"))],
  ["No offer online", win(topBar(), item(HH), price(HH_PRICE), offers({ lines: ["No offer online with these filters.", "Uncheck a stat or lower a minimum, then search again."] }), foot())],
  ["No price found", win(topBar(), item({ rarity: "unique", name: "Strugglescream", base: "Stellar Amulet", cls: "Amulets", tags: ["ilvl 76"], req: "Requires Level 60", statsLabel: "Stats (5)", filters: "2 filters" }),
    price({ lines: ["No price found for this item in Forbidden Rites."] }), offers({ state: "idle" }), foot())],
]]);
