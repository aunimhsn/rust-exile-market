// The item block, its stats and their filters.
// The attribute of a requirement or of a base property, for its color.
const colorAttributes = text => text.replace(/(\d+ )(Str|For)\b/g, '<span class="attr-str">$1$2</span>')
  .replace(/(\d+ )(Dex)\b/g, '<span class="attr-dex">$1$2</span>')
  .replace(/(\d+ )(Int)\b/g, '<span class="attr-int">$1$2</span>');
const propClass = name => ({ Armour: "attr-str", Evasion: "attr-dex", "Energy Shield": "attr-int" })[name] || "";

function item(it, unfolded = false) {
  const tagClass = t => /^Corrupt|^Corrompu/.test(t) ? "corrupted" : /^\d+ T1$/.test(t) ? "t1" : "";
  const tags = (it.tags || []).map(t => `<span class="tag ${tagClass(t)}">${t}</span>`).join("");
  const filters = it.filters ? `<span class="filters">${it.filters}</span>` : "";
  // "Show 6 stats" when folded, "Hide stats" when unfolded; French when the label is French.
  const count = Number((it.statsLabel || "").match(/\d+/)?.[0] || 0);
  const fr = /Statistiques/.test(it.statsLabel || "");
  const plural = count > 1 ? "s" : "";
  // An item without modifiers (a normal item) only has filters to show.
  const pill = unfolded => count === 0
    ? (unfolded ? (fr ? "Masquer les filtres" : "Hide filters") : (fr ? "Afficher les filtres" : "Show filters"))
    : unfolded
      ? (fr ? "Masquer les stats" : "Hide stats")
      : (fr ? `Afficher ${count} stat${plural}` : `Show ${count} stat${plural}`);
  let stats = "";
  if (it.stats && unfolded) {
    stats = `<div class="stats-toggle open"><span class="pill">${pill(true)}</span>${filters}</div><div class="stats">${it.stats.map(row => {
      if (row === "sep") return `<div class="sep"></div>`;
      if (row.minHeader) return `<div class="req"><span></span><span>min</span></div>`;
      if (row.group) return `<div class="stats-group"><span>${row.group}${row.fill ? ` <em>${row.fill}</em>` : ""}</span>${row.min ? "<span>min</span>" : ""}</div>`;
      if (row.prop) return `<div class="prop"><span>${row.prop[0]}:</span> ${row.prop[1]}</div>`;
      if (row.note) return `<div class="note">${row.note}</div>`;
      // The tier and the ranges of a hybrid modifier, under all its lines: it reads as one modifier.
      if (row.rollOnly) return `<div class="mod2 min-top"><span></span><div class="roll2" style="grid-column: 2"><span class="range"><span class="tier t${row.tier}">T${row.tier}</span>${row.range}</span></div></div>`;
      // A filter: its checkbox, its text on two lines at most (whole under the pointer), its minimum
      // on the line of the checkbox in a column kept for it, and its bar and range under the text.
      // A stat without a filter has no checkbox and starts at the left edge.
      const box = row.check === undefined ? "" : `<span class="check ${row.check ? "on" : ""}">${row.check ? ICON.check(11) : ""}</span>`;
      const tier = row.tier ? `<span class="tier ${row.tier <= 3 ? "tier-top" : ""} t${row.tier}">T${row.tier}</span>` : "";
      // No bar for the tier of a rare until the modifier tables (step 13).
      const bar = row.range && !row.noBar ? `<span class="bar"><i style="width:${row.share * 100}%"></i></span>` : "";
      const input = row.min !== undefined ? `<span class="input">${row.min}</span>` : "";
      const plain = row.mod.replace(/<br>/g, " ");
      const text = `<span class="text" title="${plain.replace(/"/g, "&quot;")}">${row.mod}</span>`;
      if (box) {
        const rollTop = row.range ? `<div class="roll2">${bar}<span class="range">${tier}${row.range}</span></div>` : "";
        return `<div class="mod2 min-top">${box}${text}${input || "<span></span>"}${rollTop}</div>`;
      }
      const roll = row.range || input ? `<div class="roll2">${bar}<span class="range">${tier}${row.range || ""}</span>${input}</div>` : "";
      return `<div class="mod2 ${box ? "" : "wide"}">${box}${text}${roll}</div>`;
    }).join("")}</div>`;
  } else if (it.statsLabel) {
    stats = `<div class="stats-toggle"><span class="pill">${pill(false)}</span>${filters}</div>`;
  }
  return `
    <div class="band b item-band">
      <div class="name-row">
        <div class="name rarity-${it.rarity}">${it.name}</div>
        <a class="trade" title="${it.tradeTitle || "Open the search on the trade site"}">${it.tradeLabel || "Check on trade"}</a>
      </div>
      <div class="base">${it.base ? `${it.base} <span class="cls">· ${it.cls}</span>` : `<span class="cls">${it.cls}</span>`}</div>
      <div class="facts">
        ${it.summary ? `<div class="summary">${it.summary}</div>` : ""}
        ${it.props ? `<div class="summary props">${it.props.map(([k, v]) => `<div><span class="${propClass(k)}">${k}</span><b>${v}</b></div>`).join("")}</div>` : ""}
        ${it.req ? `<div class="reqline">${colorAttributes(it.req)}</div>` : ""}
        ${tags ? `<div class="meta">${tags}</div>` : ""}
      </div>
      ${stats}
    </div>`;
}
