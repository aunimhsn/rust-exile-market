// The price block.
function price(p, lang = "en") {
  const label = lang === "fr" ? "Prix" : "Price";
  let body = "";
  // A Divine or Chaos Orb amount under 0.1 is hidden, and the currencies left move to the left; the
  // Exalted Orbs always stay.
  const tiny = (v, i) => i < 2 && (v.startsWith("<") || (!/[KM]$/.test(v) && Number(v) < 0.1));
  const row = values => `<div class="prices">${[["div", values[0]], ["chaos", values[1]], ["ex", values[2]]]
    .filter(([, v], i) => !tiny(v, i)).map(([k, v]) => `<div class="price">${orb(k)}${v}</div>`).join("")}</div>`;
  const inline = values => [[values[0], "div"], [values[1], "c"], [values[2], "ex"]]
    .filter(([v], i) => !tiny(v, i)).map(([v, k]) => `${v} ${k}`).join(" · ");
  let caption = "";
  if (p.stackValues) {
    // Stack: the value of the copied stack in large, the unit price on a small line, to compare with the offers.
    body = `${row(p.stackValues)}<div class="line muted">Unit price: ${inline(p.values)}</div>`;
    // Currency exchange items always have plenty of offers: the stack size replaces their count.
    caption = `Stack of ${p.stackSize}`;
    if (p.trend) body += `<div class="line ${p.rising === false ? "fall" : "rise"}">${p.trend}</div>`;
  } else if (p.values) {
    body = row(p.values);
    if (p.trend) body += `<div class="line ${p.stable ? "muted" : p.rising === false ? "fall" : "rise"}">${p.trend}</div>`;
    if (p.warning) body += `<div class="line warn">${p.warning}</div>`;
  }
  if (p.note) body += `<div class="line muted">${p.note}</div>`;
  if (p.lines) body = p.lines.map((l, i) => `<div class="msg ${i ? "muted" : ""}">${l}</div>`).join("");
  if (p.candidates) {
    body = `<div class="msg">Identify the item to get its price.</div><div class="line muted">Possible uniques</div>
      <div class="candidates">${p.candidates.map(([n, v, k]) => `<div><span class="rarity-unique">${n}</span>${v === null ? `<span class="muted">No offers</span>` : `<span>${v} ${orb(k)}</span>`}</div>`).join("")}</div>`;
  }
  const right = caption || p.offers;
  const title = p.estimate ? (lang === "fr" ? "Prix estimé" : "Estimated price") : label;
  return `<div class="band a"><div class="head"><span class="label">${title}</span>${right ? `<span class="right">${right}</span>` : ""}</div>${body}</div>`;
}
