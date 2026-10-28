// Section Errors.
SECTIONS.push(["Errors", [
  ["Offline, cached prices", win(topBar(), banner("The price database can't be reached. Last updated 3h ago."), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot("poe2scout 3h ago (offline) · GGG trade 2min ago"))],
  ["League ended: back to Standard, without a message", win(topBar(false, "Standard"), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot())],
  ["New league started", win(topBar(false, "Standard"), newLeague("Rise of the Abyssal"), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot())],
  ["Clipboard unreadable", win(topBar(), banner("The clipboard can't be read."), empty, foot("poe2scout 12min ago"))],
  ["Several warnings", win(topBar(), banner("The clipboard can't be read.", "The price database can't be reached. Last updated 3h ago."), empty, foot("poe2scout 3h ago (offline)"))],
  ["First launch, downloading the prices", win(topBar(), downloading("Forbidden Rites", 40, "Counting the Exalted Orbs one by one…"), foot("poe2scout: downloading prices… · GGG trade: no search yet"))],
  ["Another league chosen, downloading its prices", win(topBar(false, "Runes of Aldur"), downloading("Runes of Aldur", 75, "Spotting the bait offers…"), foot("poe2scout: downloading prices… · GGG trade: no search yet"))],
  ["First launch offline, no prices", win(topBar(), banner("Prices unavailable, retrying…"), empty, foot("poe2scout: no prices yet"))],
  ["Before the first item", win(topBar(), empty, foot("poe2scout 12min ago"))],
  ["Item couldn't be read", win(topBar(), `<div class="band b"><div class="msg">This item couldn't be read.</div><div class="line muted">Copy it again, or copy another item.</div></div>`, foot())],
  // An item with modifiers copied without their details (no "{ … }" header): nothing priced or searched.
  ["Item copied without its details", win(topBar(), `<div class="band b"><div class="msg">This item was copied without its details.</div><div class="line muted">Copy it again with Ctrl + Alt + C.</div></div>`, foot())],
]]);
