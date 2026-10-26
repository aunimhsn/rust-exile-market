// The offers and their tooltip.
function offerTooltip(t) {
  const compare = (list, kind = "") => list.map(m => {
    const diff = m.yours === undefined ? "" : m.value - m.yours;
    const delta = diff === "" || diff === 0 ? "" : `<span class="delta ${diff > 0 ? "up" : "down"}">${diff > 0 ? "+" : "−"}${Math.abs(diff)}</span>`;
    const tier = m.tier ? `<span class="tt-tier"><span class="tier ${m.tier <= 3 ? "tier-top" : ""} t${m.tier}">T${m.tier}</span></span>` : "";
    return `<div class="tt-mod ${kind}"><span>${tier}${m.text}</span>${delta}</div>`;
  }).join("");
  return `<div class="offer-tooltip ${t.pinned ? "pinned" : ""} ${t.below ? "below" : ""}">
    <div class="tt-head"><span class="rarity-${t.rarity || "unique"} tt-name">${t.name}</span><span class="tt-price">${t.price} ${orb(t.currency)}</span></div>
    <div class="tt-meta">${t.meta}</div>
    ${t.suspicious ? `<div class="tt-warn">Be careful: far below the market price.</div>` : ""}
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
    // Like the header: a flat block the height of the band, glued to its right edge.
    const action = fr ? "Chercher les offres" : "Search the offers";
    return `<div class="b search-band">
      <div class="search-text"><span class="label">${label}</span>
        <div class="muted">${fr ? "Une recherche par clic, dans les limites du site d'échange." : "One search per click, within the limits of the trade site."}</div></div>
      <div class="search-block" title="${action}" aria-label="${action}">${ICON.search(22)}</div>
    </div>`;
  }
  if (o.state === "loading") {
    return `<div class="band b"><div class="head"><span class="label">${label}</span></div>
      <div class="actions" style="justify-content:flex-start;gap:10px"><span class="spinner"></span>Searching the trade site…</div></div>`;
  }
  if (o.state === "wait") {
    return `<div class="band b"><div class="head flat-head"><span class="title-stack"><span class="label">${label}</span><span class="age">searched 1min ago</span></span><span class="right"></span><div class="again waiting" title="Next search in 4min">4min</div></div>
      <div class="warn">Trade site limit: next search in 4min.</div></div>`;
  }
  if (o.lines) {
    return `<div class="band b"><div class="head"><span class="label">${label}</span></div>${o.lines.map(l => `<div class="msg muted">${l}</div>`).join("")}</div>`;
  }
  const three = o.columns.length === 3;
  const again = o.ready
    ? `<div class="again ready" title="Search again">${ICON.refresh(18)}</div>`
    : `<div class="again waiting" title="Next search in 8s">8s</div>`;
  const rows = o.rows.map((r, i) => `<div class="row-o ${r.sus ? "sus" : ""} ${o.hover === i ? "hovered" : ""}">
      <span class="p">${r.v} ${orb(r.k)}</span>${three ? `<span>${r.stock}</span>` : ""}<span><i class="online ${r.away ? "away" : ""}"></i>${r.when}</span>${o.hover === i ? offerTooltip(o.tooltip) : ""}</div>`).join("");
  return `<div class="band b">
    <div class="head flat-head"><span class="title-stack"><span class="label">${label}</span><span class="age">${o.age || "searched 2min ago"}</span></span><span class="right">${o.count}</span>${again}</div>
    <div class="offers ${three ? "three" : ""}"><div class="cols">${o.columns.map(c => `<span>${c}</span>`).join("")}</div>${rows}</div>
    ${more(o.more, fr)}</div>`;
}

// Under the list: the double chevron; the spinner while the next offers come; when they couldn't come, the
// chevron again, to ask them again, with why above it.
function more(state, fr) {
  if (state === "loading") return `<div class="more loading"><span class="spinner"></span></div>`;
  const chevron = `<div class="more" title="${fr ? "Afficher plus d'offres" : "Show more offers"}">${ICON.downDouble(16)}</div>`;
  return state === "failed" ? `<div class="more-note">The trade site can't be reached.</div>${chevron}` : chevron;
}
