// Section Window over the game (14b1): Ctrl+D in the game opens the narrow window of the application over the
// game, as a column of 420 px beside the panel the item is in (option B of docs/wireframes/overlay-window.excalidraw,
// the owner's choice on 2026-11-01): left of the inventory when the item is on the right half of the screen,
// right of the stash when it is on the left half, like Exiled Exchange 2 (PriceCheckWindow.vue, commit cca3066).
// Checked in the owner's game on 2026-11-01 (Bazzite KDE, windowed fullscreen): such a window shows over the game,
// doesn't take its focus, and a click on it stays in it. The screen is the owner's, 2560 × 1440, at half scale;
// a click on an item stands for Ctrl+D, and the real Ctrl key of the keyboard is read. The height, chosen by the
// owner on 2026-11-01: the height of the content, level with the item (its top at the top of the item, moved up as
// much as the screen needs), growing when the stats are shown, at most the height of the screen, then its content
// scrolls. Two open decisions, one group of tabs each: its title bar, and what closes it.

VARIANTS.overlayBar = { label: "Title bar", options: { full: "Whole", short: "League and Close only" }, fallback: "full" };
VARIANTS.overlayClose = { label: "Closes", options: { away: "Mouse 40 px away (EE2, Ctrl held to reach it)", delay: "1 s outside the item and the window", click: "A click in the game" }, fallback: "away" };
for (const key of ["overlayBar", "overlayClose"]) {
  const v = VARIANTS[key];
  chosen[key] = params.get(key) in v.options ? params.get(key) : v.fallback;
}

const OV_BOW_PRICE = { estimate: true, values: ["2.4", "24", "1.7K"], offers: "18 similar", note: "Median of the 5 cheapest similar offers." };
const OV_BOW_OFFERS = { count: "10 of 18 online", columns: ["Price", "Listed"], rows: [
  { v: "300", k: "ex", when: "1h ago", sus: true }, { v: "2", k: "div", when: "20min ago" }, { v: "2.2", k: "div", when: "4h ago" },
  { v: "2.4", k: "div", when: "1d ago", away: true }, { v: "2.5", k: "div", when: "2h ago" }, { v: "3", k: "div", when: "6h ago" }] };
// The items of the screen: the panel, the cell (column, row) and size in cells, and the window each opens.
const OV_ITEMS = {
  headhunter: { panel: "stash", at: [3, 5], size: [2, 1], label: "Headhunter", rarity: "unique",
    window: (unfolded = false) => win(topBar(), item({ ...HH, stats: HH_STATS }, unfolded), price(HH_PRICE), offers(HH_OFFERS), foot()) },
  bow: { panel: "inventory", at: [5, 1], size: [2, 4], label: "Storm Bite", rarity: "rare",
    window: (unfolded = false) => win(topBar(), item({ ...BOW_RARE, stats: BOW_FILTERS }, unfolded), price(OV_BOW_PRICE), offers(OV_BOW_OFFERS), foot()) },
  ring: { panel: "inventory", at: [9, 3], size: [1, 1], label: "Ring", rarity: "rare",
    window: (unfolded = false) => win(topBar(), item({ ...RING, stats: RING_FILTERS }, unfolded), price(RARE_ESTIMATE_BEFORE), offers({ state: "idle" }), foot()) },
};
const OV = { cell: 70, panel: 888, width: 420, stash: [24, 200], inventory: [2560 - 888 + 24, 880] };

const ovItemsHtml = panel => Object.entries(OV_ITEMS).filter(([, it]) => it.panel === panel).map(([key, it]) => {
  const [x0, y0] = OV[panel];
  return `<div class="ov-item is-${it.rarity}" data-ov-item="${key}" style="left:${x0 + it.at[0] * OV.cell}px; top:${y0 + it.at[1] * OV.cell}px; width:${it.size[0] * OV.cell}px; height:${it.size[1] * OV.cell}px">${it.label}</div>`;
}).join("");
const ovGrid = (panel, rows) => {
  const [x, y] = OV[panel];
  return `<div class="ov-grid" style="left:${x}px; top:${y}px">${"<span></span>".repeat(12 * rows)}</div>`;
};
const ovScreen = `<div class="ov-screen"><div class="ov-stage">
    <div class="ov-scene"><i></i></div>
    <div class="ov-panel ov-stash"><h4>Stash</h4></div>${ovGrid("stash", 12)}${ovItemsHtml("stash")}
    <div class="ov-panel ov-inventory"><h4>Inventory</h4><div class="ov-equipment"></div></div>${ovGrid("inventory", 5)}${ovItemsHtml("inventory")}
    <div class="ov-reach"></div>
    <div class="ov-window"></div>
  </div></div>
  <div class="ov-pointer-note">Click an item: it stands for <span class="ov-key">Ctrl</span>+<span class="ov-key">D</span>. Your real <span class="ov-key ov-ctrl">Ctrl</span> key is read: hold it to keep the window open. <b class="ov-status">Closed.</b></div>`;

// The windowed fullscreen: where the player learns that the exclusive fullscreen isn't supported.
const ovEmpty = empty.replace(`<span class="key">C</span>`, `<span class="key">D</span>`);
const ovEmptyNote = ovEmpty.replace(`<span>The price shows up here.</span>`, `<span>The price shows up here.</span>
  <span class="ov-note-line">Play in windowed fullscreen: in fullscreen, the game lets no application copy the item or show its price over it.</span>`);
// Chosen by the owner on 2026-11-01: the line under the invitation only, the most discreet; no bubble, and no
// Help in the settings for now (no dead elements there; its place to see later). The interface never names
// another application.
SECTIONS.unshift(["Windowed fullscreen note (14b1): A chosen, a line under the invitation", [
  ["Chosen · A line under the invitation, before the first item", win(topBar(), ovEmptyNote, foot())],
]]);
SECTIONS.unshift(["Window over the game (14b1): click an item of the screen, it stands for Ctrl+D", [
  ["Your screen, 2560 × 1440 at half scale: stash on the left, inventory on the right", ovScreen],
  ["The window over the game, at real size", `<div class="ov-real">${OV_ITEMS.bow.window()}</div>`],
]]);

// The behaviour: open at a click on an item, placed by the half of the screen, closed by the chosen rule.
const ovState = { unfolded: false, open: false, press: null, item: null, entered: false, timer: null, ctrl: false, last: null };
const ovStage = () => document.querySelector(".ov-stage");
function ovStatus(text) {
  const status = document.querySelector(".ov-status");
  if (status) status.textContent = text;
}
function ovClose(why) {
  clearTimeout(ovState.timer);
  Object.assign(ovState, { open: false, press: null, item: null, entered: false, timer: null });
  const stage = ovStage();
  if (!stage) return;
  stage.querySelector(".ov-window").className = "ov-window";
  stage.querySelector(".ov-reach").classList.remove("is-on");
  ovStatus(`Closed: ${why}.`);
}
// Level with the item: the top of the window at the top of the item, moved up as much as the screen needs, at
// most the height of the screen (its content then scrolls).
function ovPlace(windowElement) {
  windowElement.style.top = `${Math.max(0, Math.min(ovState.item.y, 1440 - windowElement.offsetHeight))}px`;
}
function ovOpen(key, element, press, unfolded = false) {
  const stage = ovStage();
  const it = OV_ITEMS[key];
  const box = { x: element.offsetLeft, y: element.offsetTop, w: element.offsetWidth, h: element.offsetHeight };
  const right = box.x + box.w / 2 > 1280;
  const left = right ? 2560 - OV.panel - OV.width : OV.panel;
  const windowElement = stage.querySelector(".ov-window");
  windowElement.innerHTML = it.window(unfolded);
  windowElement.className = "ov-window is-open";
  windowElement.style.left = `${left}px`;
  const reach = stage.querySelector(".ov-reach");
  reach.style.left = `${press.x}px`;
  reach.style.top = `${press.y}px`;
  reach.classList.add("is-on");
  clearTimeout(ovState.timer);
  Object.assign(ovState, { open: true, press, item: box, entered: false, timer: null, last: { key, press }, unfolded });
  ovPlace(windowElement);
  ovStatus(`Open ${right ? "left of the inventory" : "right of the stash"}: ${it.label}.`);
}
const ovPoint = event => {
  const rect = document.querySelector(".ov-screen").getBoundingClientRect();
  return { x: (event.clientX - rect.left) * 2, y: (event.clientY - rect.top) * 2 };
};
const ovInside = (p, b) => p.x >= b.x && p.x <= b.x + b.w && p.y >= b.y && p.y <= b.y + b.h;
const ovWindowBox = () => {
  const w = ovStage().querySelector(".ov-window");
  return { x: w.offsetLeft, y: w.offsetTop, w: w.offsetWidth, h: w.offsetHeight };
};
document.addEventListener("click", event => {
  // A tab draws the sheet again (its own listener runs first): the window of the last item opens again, so
  // that the variants compare on the same item.
  if (event.target.closest("#styles") && event.target.dataset.variant) {
    clearTimeout(ovState.timer);
    if (ovState.open && ovState.last) {
      const element = document.querySelector(`[data-ov-item="${ovState.last.key}"]`);
      if (element) ovOpen(ovState.last.key, element, ovState.last.press, ovState.unfolded);
    }
    return;
  }
  if (!event.target.closest(".ov-screen")) return;
  const itemElement = event.target.closest(".ov-item");
  if (itemElement) {
    ovOpen(itemElement.dataset.ovItem, itemElement, ovPoint(event));
  } else if (event.target.closest(".ov-window .stats-toggle .pill")) {
    // Show or hide the stats: the window grows or shrinks, and moves up if the screen needs it.
    const windowElement = ovStage().querySelector(".ov-window");
    ovState.unfolded = !ovState.unfolded;
    windowElement.innerHTML = OV_ITEMS[ovState.last.key].window(ovState.unfolded);
    ovPlace(windowElement);
  } else if (event.target.closest(".app-icon-button[title='Close']") && event.target.closest(".ov-window")) {
    ovClose("its Close button");
  } else if (ovState.open && !event.target.closest(".ov-window")) {
    ovClose("a click in the game");
  }
});
document.addEventListener("mousemove", event => {
  if (!ovState.open || !event.target.closest(".ov-screen")) return;
  // A tab drew the sheet again: the window drawn before is gone.
  if (!ovStage().querySelector(".ov-window.is-open")) {
    ovState.open = false;
    return;
  }
  const p = ovPoint(event);
  const inWindow = ovInside(p, ovWindowBox());
  const windowElement = ovStage().querySelector(".ov-window");
  if (inWindow) ovState.entered = true;
  switch (chosen.overlayClose) {
    case "away":
      if (inWindow || ovState.ctrl) return;
      if (ovState.entered) ovClose("the mouse left the window");
      else if (Math.hypot(p.x - ovState.press.x, p.y - ovState.press.y) > 40) ovClose("the mouse moved 40 px away without Ctrl held");
      break;
    case "delay":
      if (inWindow || ovInside(p, ovState.item)) {
        clearTimeout(ovState.timer);
        ovState.timer = null;
        windowElement.classList.remove("is-fading");
        ovStatus("Open: the mouse is on the item or in the window.");
      } else if (!ovState.timer) {
        windowElement.classList.add("is-fading");
        ovStatus("Closing in 1 s, unless the mouse comes back to the item or the window…");
        ovState.timer = setTimeout(() => ovClose("1 s outside the item and the window"), 1000);
      }
      break;
    default:
      break;
  }
});
const ovCtrl = held => {
  ovState.ctrl = held;
  document.querySelectorAll(".ov-ctrl").forEach(key => key.classList.toggle("is-held", held));
};
document.addEventListener("keydown", event => { if (event.key === "Control") ovCtrl(true); });
document.addEventListener("keyup", event => { if (event.key === "Control") ovCtrl(false); });
window.addEventListener("blur", () => ovCtrl(false));
// `?ovopen=bow` opens the window of that item at load, as if Ctrl+D was pressed on its middle, `&ovstats` with its
// stats shown (screenshots).
if (params.get("ovopen") in OV_ITEMS) {
  setTimeout(() => {
    const element = document.querySelector(`[data-ov-item="${params.get("ovopen")}"]`);
    ovOpen(params.get("ovopen"), element, { x: element.offsetLeft + element.offsetWidth / 2, y: element.offsetTop + element.offsetHeight / 2 }, params.has("ovstats"));
  });
}
