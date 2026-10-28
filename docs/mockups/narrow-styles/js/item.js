// The item block, its stats and their filters.
// The attribute of a requirement or of a base property, for its color.
const colorAttributes = text => text.replace(/(\d+ )(Str|For)\b/g, '<span class="attr-str">$1$2</span>')
  .replace(/(\d+ )(Dex)\b/g, '<span class="attr-dex">$1$2</span>')
  .replace(/(\d+ )(Int)\b/g, '<span class="attr-int">$1$2</span>');
const propClass = name => ({ Armour: "attr-str", Evasion: "attr-dex", "Energy Shield": "attr-int" })[name] || "";

// The bar of a rare or magic modifier over every tier of its base (13j2): filled to where the roll
// falls between the lowest roll of its worst tier and the highest of tier 1 (`overTiers` of the section
// Roll bar), so that a full bar means close to the best possible.
// The origin of a modifier in words (13k1), in English; `row.originLabel` for another language.
const ORIGIN_LABEL = { desecrated: "Desecrated", fractured: "Fractured", crafted: "Crafted" };
const rareBar = o => `<span class="bar"><i style="width:${(o.share * 100).toFixed(1)}%"></i></span>`;

function item(it, unfolded = false) {
  // Corrupted once or twice, as in the application.
  const tagClass = t => /Corrupt|Corrompu|Double corruption/.test(t) ? "corrupted" : /^\d+ T1$/.test(t) ? "t1" : "";
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
      if (row.group) return `<div class="stats-group"><span>${row.group}${row.fill ? ` <em>${row.fill}</em>` : ""}</span>${row.min ? "<span>min – max</span>" : ""}</div>`;
      if (row.prop) return `<div class="prop"><span>${row.prop[0]}:</span> ${row.prop[1]}</div>`;
      // A class of its own, e.g. the variant of an open decision it belongs to.
      // `row.origin`: a desecrated, fractured or crafted modifier (13k1), drawn as the variant `origin` says.
      const cls = (row.cls ? ` ${row.cls}` : "") + (row.origin ? ` app-origin-${row.origin}` : "");
      // A modifier not revealed yet (13k1): a line without filter, drawn as the variant `unrevealed` says.
      if (row.unrevealed) return `<div class="mod2 wide app-unrevealed${cls}"><span class="text">${row.unrevealed}</span></div>`;
      if (row.note) return `<div class="note${cls}">${row.note}</div>`;
      // The tier and the ranges of a hybrid modifier, under all its lines: it reads as one modifier.
      if (row.rollOnly) return `<div class="mod2 min-top"><span></span><div class="roll2" style="grid-column: 2"><span class="range"><span class="tier t${row.tier}">T${row.tier}</span>${row.range}</span></div></div>`;
      // A filter: its checkbox, its text on two lines at most (whole under the pointer), its minimum
      // on the line of the checkbox in a column kept for it, and its bar and range under the text.
      // A stat without a filter has no checkbox and starts at the left edge.
      // A line the trade site can't search: a circle struck through in place of its checkbox, in the
      // warning color, with its tooltip (`row.warning`); `row.hover` shows it open.
      const warning = row.warning ? `<span class="app-warning${row.hover ? " is-hovered" : ""}">${ICON.forbid(15)}<span class="app-tip">${row.warning}</span></span>` : "";
      const box = warning || (row.check === undefined ? "" : `<span class="check ${row.check ? "on" : ""}">${row.check ? ICON.check(11) : ""}</span>`);
      // A modifier of one tier only (`row.single`, 13k1: jewels, lords of the Abyss, waystones): its T1 says
      // "the only one", not "the best"; shown or not as the variant `single` says.
      const tier = row.single ? `<span class="tier app-single">T1</span>`
        : row.tier ? `<span class="tier ${row.tier <= 3 ? "tier-top" : ""} t${row.tier}">T${row.tier}</span>` : "";
      // The origin in words after the range (variant `origin` "word"); hidden in the other variants.
      const origin = row.origin ? `<span class="app-origin">${row.originLabel || ORIGIN_LABEL[row.origin]}</span>` : "";
      // A line of a Time-Lost jewel (`row.radius`: the words before it, and its short tag), as the
      // variant `radius` says.
      const mod = row.radius ? `<span class="app-radius-full">${row.radius[0]} </span><span class="app-radius-short">${row.radius[1]}</span>${row.mod}` : row.mod;
      // The bar: filled to where the roll falls in its range, also on a line better lower, so that it
      // reads with the range written next to it.
      // A rare or magic modifier with the modifier tables (`row.over`, 13j2): its bar over every tier of
      // its base, also on each line of a hybrid (no range of its own: its tier and ranges go under the last).
      const bar = row.over ? rareBar(row.over)
        : row.range && !row.noBar ? `<span class="bar"><i style="width:${row.share * 100}%"></i></span>` : "";
      // The range field of a filter: the minimum, a dash, the maximum, an empty half without a word
      // (the title of the column names them). Each half holds three characters, four on the field of a
      // stat whose range reaches them ("wide"); it never grows with what is typed. A line better lower
      // is searched with a maximum: its minimum stays empty. Real fields, to try them: the caret in the
      // middle of an empty half, the number written from the middle.
      const low = row.lower ? undefined : row.min;
      // The tooltip of the field, under the pointer (`row.hover` shows it open): the rolls the field
      // accepts, those of a unique ("Rolls 40 to 60"), or for a rare, from the lowest roll of its worst
      // tier to the highest of tier 1 (`row.bounds`, from the modifier tables, step 13j; nothing for a
      // rare before them); then "Lower is better" for a line better lower. Not the name of a half nor
      // that it is empty: the title of the column and the field show them.
      const [boundLow, boundHigh] = (row.range || "").split("–");
      const bounds = row.bounds ?? (row.tier || !boundHigh || row.range.includes("·") ? null : `${boundLow} ${fr ? "à" : "to"} ${boundHigh}`);
      const rolls = bounds ? `${fr ? "Jets de" : "Rolls"} ${bounds}` : null;
      // For a rare, the tiers of these rolls right under them, small and gray: "T13–T1" (13j2; in
      // the line of the rolls, "Rolls 10 T13 to 214 T1" read badly, the owner found).
      const tiers = row.boundTiers ? `<span class="app-tip-tiers">T${row.boundTiers[0]}–T${row.boundTiers[1]}</span>` : "";
      const lowerNote = row.lower ? (fr ? "Plus c'est bas, mieux c'est" : "Lower is better") : null;
      const fieldTip = [rolls && `<span>${rolls}</span>${tiers}`, lowerNote && `<span>${lowerNote}</span>`].filter(Boolean).join("");
      const half = (value, name) => `<input value="${value ?? ""}" inputmode="decimal" aria-label="${name}">`;
      const hovered = row.hover ? " is-hovered" : "";
      const fields = row.min === undefined ? "" : `<span class="input app-range${hovered}">${half(low, "Minimum")}<i>–</i>${half(row.max, "Maximum")}<span class="app-tip">${fieldTip}</span></span>`;
      const plain = row.plain ?? mod.replace(/<br>/g, " ").replace(/<[^>]+>/g, "");
      const tip = row.tip ? `${plain}\n${row.tip}` : plain;
      const text = `<span class="text" title="${tip.replace(/"/g, "&quot;")}">${mod}</span>`;
      if (box) {
        const rollTop = row.range ? `<div class="roll2">${bar}<span class="range">${tier}${row.range}${origin}</span></div>`
          : bar ? `<div class="roll2">${bar}</div>` : "";
        return `<div class="mod2 min-top${row.wide ? " app-wide" : ""}${cls}">${box}${text}${fields || "<span></span>"}${rollTop}</div>`;
      }
      const input = row.min === undefined ? "" : `<span class="input">${row.min}</span>`;
      const roll = row.range || input ? `<div class="roll2">${bar}<span class="range">${tier}${row.range || ""}</span>${input}</div>` : "";
      return `<div class="mod2 ${box ? "" : "wide"}${cls}">${box}${text}${roll}</div>`;
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
