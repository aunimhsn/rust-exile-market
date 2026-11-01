// The offers and their tooltip.
// The currency of a price (fix of the prices in other currencies): its icon (`k`), its name after it
// (`name`) in weak text, or its name alone for an item without icon (`k` absent); the name in a tooltip
// over the icon (`tip`), shown when `tipShown`.
function currencyMark(r) {
  const tip = r.tip ? `<span class="app-tip">${r.tip}</span>` : "";
  const icon = r.k ? `<span class="app-currency-icon ${r.tipShown ? "is-hovered" : ""}">${orb(r.k)}${tip}</span>` : "";
  const name = r.name ? `<span class="app-currency-name ${r.k ? "" : "is-alone"}">${r.name}</span>` : "";
  return icon + name;
}
function offerTooltip(t) {
  const compare = (list, kind = "") => list.map(m => {
    const diff = m.yours === undefined ? "" : m.value - m.yours;
    const delta = diff === "" || diff === 0 ? "" : `<span class="delta ${diff > 0 ? "up" : "down"}">${diff > 0 ? "+" : "−"}${Math.abs(diff)}</span>`;
    const tier = m.tier ? `<span class="tt-tier"><span class="tier ${m.tier <= 3 ? "tier-top" : ""} t${m.tier}">T${m.tier}</span></span>` : "";
    // A quality of another kind than the copied item's (13k4, `m.other`): its text in red, without difference;
    // a quality at the maximum of its item (`m.max`): its text in white.
    // A property of the copied waystone that the offer lacks (13k5c2, `m.lacking`): faint, at 0.
    return `<div class="tt-mod ${kind} ${m.other ? "is-other" : m.max ? "is-max" : m.lacking ? "is-lacking" : ""}"><span>${tier}${m.text}</span>${m.other ? "" : delta}</div>`;
  }).join("");
  return `<div class="offer-tooltip ${t.pinned ? "pinned" : ""} ${t.below ? "below" : ""}">
    <div class="tt-head"><span class="rarity-${t.rarity || "unique"} tt-name">${t.name}</span><span class="tt-price">${t.price} ${currencyMark({ k: t.currency, name: t.currencyName })}</span></div>
    <div class="tt-meta">${t.meta}</div>
    ${t.req ? `<div class="tt-req">${colorAttributes(t.req)}</div>` : ""}
    ${t.suspicious ? `<div class="tt-warn">Be careful: far below the market price.</div>` : ""}
    ${t.note ? `<div class="tt-note">${t.note}</div>` : ""}
    ${[[t.props, "prop"], [t.implicit, ""], [t.mods, ""]].filter(([list]) => list && list.length).map(([list, kind]) => `<div class="tt-mods">${compare(list, kind)}</div>`).join("")}
    <div class="tt-foot">
      <span class="status s-${t.status}"><i></i>${{ online: "Online", away: "Away", offline: "Offline" }[t.status]}</span>
      ${t.status === "offline" ? "" : `<span class="pill ${t.copied ? "done" : ""}">${t.copied ? `Copied${ICON.check(11)}` : "Copy whisper"}</span>`}
    </div>
  </div>`;
}

function offers(o, lang = "en") {
  const fr = lang === "fr";
  const label = fr ? "Offres" : "Offers";
  if (o.state === "idle") {
    // Like the title row after a search: the hint small under the label, and the search block the size of
    // the waiting block, in the top right corner; the band is only the height of that row.
    const action = fr ? "Chercher les offres" : "Search the offers";
    const hint = fr ? "Une recherche par clic, dans les limites du site d'échange." : "One search per click, within the limits of the trade site.";
    return row(hint, `<div class="search-block" title="${action}" aria-label="${action}">${ICON.search(22)}</div>`);
  }
  // Until the offers show, the band keeps the height of this row: the line under the label tells what
  // happens, and the block in the corner keeps its place.
  function row(line, block, kind = "") {
    return `<div class="band b"><div class="head flat-head idle-head"><span class="title-stack"><span class="label">${label}</span><span class="age ${kind}">${line}</span></span>${block}</div></div>`;
  }
  if (o.state === "loading") {
    return row("Searching the trade site…", `<div class="again waiting" title="Waiting for the answer of the trade site"><span class="spinner"></span></div>`);
  }
  if (o.state === "unreachable") {
    return row("The trade site can't be reached.", `<div class="search-block" title="Search the offers">${ICON.search(22)}</div>`, "plain");
  }
  if (o.state === "wait") {
    return row("Trade site limit: next search in 4min.", `<div class="again waiting" title="Next search in 4min">4min</div>`, "warn");
  }
  if (o.lines) {
    return `<div class="band b"><div class="head"><span class="label">${label}</span></div>${o.lines.map(l => `<div class="msg muted">${l}</div>`).join("")}</div>`;
  }
  const three = o.columns.length === 3;
  const again = o.ready
    ? `<div class="again ready" title="Search again">${ICON.refresh(18)}</div>`
    : `<div class="again waiting" title="Next search in 8s">8s</div>`;
  // An offer of another kind of quality (13k4, `r.other`: the kind): its price faint, the kind after it.
  const rows = o.rows.map((r, i) => `<div class="row-o ${r.sus ? "sus" : ""} ${r.other || r.kindTag ? "is-other" : ""} ${o.hover === i ? "hovered" : ""}">
      <span class="p"><span class="app-amount">${r.v}</span> ${currencyMark(r)}${r.other ? `<span class="app-other-kind">${r.other}</span>` : ""}${r.kindTag ? `<span class="app-kind-dot">·</span><span class="app-kind-tag">${r.kindTag}</span>` : ""}</span>${three ? `<span>${r.stock}</span>` : ""}<span><i class="online ${r.away ? "away" : ""}"></i>${r.when}</span>${o.hover === i ? offerTooltip(o.tooltip) : ""}</div>`).join("");
  return `<div class="band b">
    <div class="head flat-head"><span class="title-stack"><span class="label">${label}</span><span class="age">${o.age || "searched 2min ago"}</span></span><span class="right">${o.count}</span>${again}</div>
    <div class="offers ${three ? "three" : ""}"><div class="cols">${o.columns.map(c => `<span>${c}</span>`).join("")}</div>${rows}</div>
    ${o.footNote ? `<div class="more-note">${o.footNote}</div>` : ""}${more(o.more, fr)}</div>`;
}

// Under the list: the double chevron; the spinner while the next offers come; when they couldn't come, the
// chevron again, to ask them again, with why above it.
function more(state, fr) {
  if (state === "loading") return `<div class="more loading"><span class="spinner"></span></div>`;
  const chevron = `<div class="more" title="${fr ? "Afficher plus d'offres" : "Show more offers"}">${ICON.downDouble(16)}</div>`;
  return state === "failed" ? `<div class="more-note">The trade site can't be reached.</div>${chevron}` : chevron;
}
