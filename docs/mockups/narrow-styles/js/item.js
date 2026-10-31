// The item block, its stats and their filters.
// The attribute of a requirement or of a base property, for its color.
const colorAttributes = text => text.replace(/(\d+ )(Str|For)\b/g, '<span class="attr-str">$1$2</span>')
  .replace(/(\d+ )(Dex)\b/g, '<span class="attr-dex">$1$2</span>')
  .replace(/(\d+ )(Int)\b/g, '<span class="attr-int">$1$2</span>');
const propClass = name => ({ Armour: "attr-str", Evasion: "attr-dex", "Energy Shield": "attr-int", "Runic Ward": "app-ward", "Sauvegarde runique": "app-ward" })[name] || "";

// The bar of a rare or magic modifier over every tier of its base (13j2): filled to where the roll
// falls between the lowest roll of its worst tier and the highest of tier 1 (`overTiers` of the section
// Roll bar), so that a full bar means close to the best possible.
// The origin of a modifier in words (13k1), in English; `row.originLabel` for another language.
// The Vaal modifier of a corrupted unique (13k2d), mutated for the trade site, says "Vaal" in every
// language, the word of the game (owner's choice, 2026-10-28).
const ORIGIN_LABEL = { desecrated: "Desecrated", fractured: "Fractured", crafted: "Crafted", mutated: "Vaal" };

// A state tag that is a filter (13k2d): `{ label, state, on, hover, tips }`, lit for the offers of that state
// only, gray for the offers with or without it; its tooltip says which.
// A click toggles it, with its tooltip (`tips`: lit, gray), so that the owner can try it.
const stateTag = t => `<span class="tag app-state-tag app-state-${t.state}${t.on ? " is-on" : ""}${t.hover ? " is-hovered" : ""}"`
  + ` data-tip-on="${t.tips[0]}" data-tip-off="${t.tips[1]}">`
  + t.label
  + `<span class="app-tip">${t.tips[t.on ? 0 : 1]}</span></span>`;
// The data of a state tag, shared by the sections: `on` lit, `hover` its tooltip open; `fr` for a French item.
const STATE_TIPS = {
  corrupted: ["Corrupted offers only", "Corrupted or not", "Offres corrompues seulement", "Corrompues ou non"],
  twice: ["Twice corrupted offers only", "Corrupted or not", "Offres doublement corrompues seulement", "Corrompues ou non"],
  sanctified: ["Sanctified offers only", "Sanctified or not", "Offres sanctifiées seulement", "Sanctifiées ou non"],
  fractured: ["Fractured offers only", "Fractured or not", "Offres fissurées seulement", "Fissurées ou non"],
  mirrored: ["Mirrored offers only", "Mirrored or not", "Offres reflétées seulement", "Reflétées ou non"],
};
const state = (label, kind, on, { hover = false, fr = false } = {}) => ({
  label, state: kind === "twice" ? "corrupted" : kind, on, hover, tips: STATE_TIPS[kind].slice(fr ? 2 : 0, fr ? 4 : 2),
});

document.addEventListener("click", event => {
  const tag = event.target.closest(".app-state-tag");
  if (!tag) return;
  const on = tag.classList.toggle("is-on");
  tag.querySelector(".app-tip").textContent = tag.dataset[on ? "tipOn" : "tipOff"];
});
const rareBar = o => `<span class="bar"><i style="width:${(o.share * 100).toFixed(1)}%"></i></span>`;

function item(it, unfolded = false) {
  // Corrupted once or twice, as in the application.
  const tagClass = t => /Corrupt|Corrompu|Double corruption/.test(t) ? "corrupted" : /^\d+ T1$/.test(t) ? "t1" : "";
  // A tag object: a state tag, or a number in a style of its own (`cls`, 13k5d1: a gem at its top levels).
  const tags = (it.tags || []).map(t => typeof t !== "object" ? `<span class="tag ${tagClass(t)}">${t}</span>`
    : t.state ? stateTag(t) : `<span class="tag ${t.cls || ""}">${t.label}</span>`).join("");
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
      // `row.origin`: a desecrated, fractured or crafted modifier (13k1).
      const cls = (row.cls ? ` ${row.cls}` : "") + (row.origin ? ` app-origin-${row.origin}` : "");
      // A modifier not revealed yet (13k1): a line without filter, on a green band.
      if (row.unrevealed) return `<div class="mod2 wide app-unrevealed${cls}"><span class="text">${row.unrevealed}</span></div>`;
      if (row.note) return `<div class="note${cls}">${row.note}</div>`;
      // A tier found from the value (13k2e, a copy from the in-game trade) looks like any other, a note
      // under the stats saying so (decided with the owner on 2026-10-29, against a dashed outline and "≈");
      // without tooltip, also when the room of the item rules out the other affix: it is then certain
      // (owner, 2026-10-29, step 13k2e3c2).
      // Two neighbouring tiers in doubt (13k2e4b, `[best, worst]`): "T1/T2", one or the other, in the color of
      // the worst (owner, 2026-10-29: a dash reads as a span of tiers, as "T13–T1" of the field's tooltip).
      // No number for a modifier of one tier only (13k3b1): "Amanamu Prefix" under a line in doubt (13k3b2).
      const tierBadge = number => {
        if (number == null) return "";
        const [best, worst] = Array.isArray(number) ? number : [number, number];
        const label = best === worst ? `T${best}` : `T${best}/T${worst}`;
        return `<span class="tier ${worst <= 3 ? "tier-top" : ""} t${worst}">${label}</span>`;
      };
      // A word after the range (13k2e), like the origin: "Hybrid" on the part of a hybrid modifier (13k2e4,
      // its tooltip naming its other line), the affix of a line whose affix stays unknown; so that the tiers
      // stay aligned on the left (owner, 2026-10-29, against the word before the tier, and "with …" after the
      // range, too long).
      const after = word => {
        if (!word) return "";
        const tip = word.tip ? `<span class="app-tip">${word.tip}</span>` : "";
        return `<span class="app-after${word.tip ? " app-has-tip" : ""}${word.end ? " app-tip-end" : ""}${word.hover ? " is-hovered" : ""}">${word.label}${tip}</span>`;
      };
      // The tier and the ranges of a hybrid modifier, under all its lines: it reads as one modifier; a word
      // after them for the other lines of a hybrid split from a summed line (13k2e4b3b).
      if (row.rollOnly) {
        const rollOrigin = row.origin ? `<span class="app-origin">${row.originLabel || ORIGIN_LABEL[row.origin]}</span>` : "";
        return `<div class="mod2 min-top${cls}"><span></span><div class="roll2" style="grid-column: 2"><span class="range">${tierBadge(row.tier)}${row.range}${after(row.after)}${rollOrigin}</span></div></div>`;
      }
      // A line whose affix stays unknown (13k2e), or a line that adds up two modifiers (13k2e4): the tier and
      // range of each, one under the other (`row.affixes`: [tier, range, word after the range]); its origin
      // on a line of its own under them, since the line has it whichever affix it is (owner, 2026-10-29,
      // step 13k2e3c2, against after the last tier, which read as the origin of the suffix only, and after
      // the text of the line, which wraps it more and is unlike every other line).
      if (row.affixes) {
        const check = `<span class="check ${row.check ? "on" : ""}">${row.check ? ICON.check(11) : ""}</span>`;
        const field = `<span class="input app-range"><input value="${row.min ?? ""}" inputmode="decimal" aria-label="Minimum"><i>–</i><input value="" inputmode="decimal" aria-label="Maximum"><span class="app-tip"></span></span>`;
        const affixOrigin = row.origin ? `<span class="app-origin">${row.originLabel || ORIGIN_LABEL[row.origin]}</span>` : "";
        // The currency of a part (13k3d2: "Alloy"), after its word, in the cyan of the crafted modifiers.
        const currency = label => label ? `<span class="app-origin app-origin-crafted">${label}</span>` : "";
        const lines = row.affixes.map(([number, range, word, part]) =>
          `<span class="range">${tierBadge(number)}${range}${after(word)}${currency(part)}</span>`).join("")
          + (affixOrigin ? `<span class="range app-origin-alone">${affixOrigin}</span>` : "");
        return `<div class="mod2 min-top${cls}">${check}<span class="text">${row.mod}</span>${field}<div class="roll2 app-affixes">${lines}</div></div>`;
      }
      // A filter: its checkbox, its text on two lines at most (whole under the pointer), its minimum
      // on the line of the checkbox in a column kept for it, and its bar and range under the text.
      // A stat without a filter has no checkbox and starts at the left edge.
      // A line the trade site can't search: a circle struck through in place of its checkbox, in the
      // warning color, with its tooltip (`row.warning`); `row.hover` shows it open.
      const warning = row.warning ? `<span class="app-warning${row.hover ? " is-hovered" : ""}">${ICON.forbid(15)}<span class="app-tip">${row.warning}</span></span>` : "";
      const box = warning || (row.check === undefined ? "" : `<span class="check ${row.check ? "on" : ""}">${row.check ? ICON.check(11) : ""}</span>`);
      // A modifier of one tier only (`row.single`, 13k1: jewels, lords of the Abyss, waystones) has no tier:
      // its T1 would say "the best" while it is "the only one" (decided with the owner on 2026-10-28).
      const tier = row.single ? "" : row.tier ? tierBadge(row.tier) : "";
      // The origin in words after the range.
      const origin = row.origin ? `<span class="app-origin">${row.originLabel || ORIGIN_LABEL[row.origin]}</span>` : "";
      // A line of a Time-Lost jewel (`row.radius`: the words the game writes before it, and its short tag):
      // the words left out, the tag on the line under it, after its range (decided with the owner on
      // 2026-10-28, against the whole line, cut by the narrow width in French).
      const radius = row.radius ? `<span class="app-radius">${row.radius[1]}</span>` : "";
      const mod = row.mod;
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
        // A tier without range (`row.tierOnly`, 13k3b2): a fixed value of a modifier of several tiers, which
        // the advanced copy writes without range, "T2" in both copies (owner's choice on 2026-10-30).
        const rollTop = row.range || row.tierOnly ? `<div class="roll2">${bar}<span class="range">${tier}${row.range || ""}${after(row.after)}${origin}</span>${radius}</div>`
          : bar || radius ? `<div class="roll2">${bar}${radius}</div>`
          // A word without tier nor range (13k2e4b): a summed line split two ways; or the origin alone
          // (13k2e5: the Vaal modifier of a unique copied from the in-game trade, without range).
          : row.after || origin ? `<div class="roll2"><span class="range${row.after ? "" : " app-origin-alone"}">${after(row.after)}${origin}</span></div>` : "";
        return `<div class="mod2 min-top${row.wide ? " app-wide" : ""}${cls}">${box}${text}${fields || "<span></span>"}${rollTop}</div>`;
      }
      const input = row.min === undefined ? "" : `<span class="input">${row.min}</span>`;
      // The origin after the range, or alone under a fixed line (a fixed Vaal modifier, as the application shows it).
      const roll = row.range || input || origin ? `<div class="roll2">${bar}<span class="range${row.range ? "" : " app-origin-alone"}">${tier}${row.range || ""}${origin}</span>${input}</div>` : "";
      return `<div class="mod2 ${box ? "" : "wide"}${cls}">${box}${text}${roll}</div>`;
    }).join("")}</div>`;
  } else if (it.statsLabel) {
    stats = `<div class="stats-toggle"><span class="pill">${pill(false)}</span>${filters}</div>`;
  }
  // A property of the item block may be written short (`[name, value, full name]`, 13k5c): its tooltip gives
  // the name of the game.
  // `noClass`: a waystone, whose item class only repeats its base (13k5c, the owner's choice).
  return `
    <div class="band b item-band">
      <div class="name-row">
        <div class="name rarity-${it.rarity}">${it.name}</div>
        <a class="trade" title="${it.tradeTitle || "Open the search on the trade site"}">${it.tradeLabel || "Check on trade"}</a>
      </div>
      <div class="base">${it.base ? `${it.base}${it.noClass ? "" : ` <span class="cls">· ${it.cls}</span>`}` : `<span class="cls">${it.cls}</span>`}</div>
      <div class="facts">
        ${it.summary ? `<div class="summary">${it.summary}</div>` : ""}
        ${it.props ? `<div class="summary props">${it.props.map(([k, v, full]) => `<div${full ? ` title="${full}"` : ""}><span class="${propClass(k)}">${k}</span><b>${v}</b></div>`).join("")}</div>` : ""}
        ${it.req ? `<div class="reqline">${colorAttributes(it.req)}</div>` : ""}
        ${tags ? `<div class="meta">${tags}</div>` : ""}
      </div>
      ${stats}
    </div>`;
}
