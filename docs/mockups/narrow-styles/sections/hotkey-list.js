// Section Hotkey list (14c3b2): option C, chosen by the owner on 2026-11-03: the key chosen from a list of safe
// shortcuts, "Other…" last for a free capture (14c3b3, drawn in the sections below), and, when the key is taken,
// the warning offering a free key of the list at once, tried before it is offered.
//
// The shortcuts of the list, checked on 2026-11-03 against:
// - the keys of Path of Exile 2 (the owner's `poe2_production_Config.ini`, both schemes): Ctrl+Q, W, E, R, T (the
//   second skill bar) and Ctrl+U (the atlas); with WASD movement, W A S D (Z Q S D on AZERTY) move, Ctrl+F is a
//   skill, Ctrl+D detonates the mines, Ctrl+Z picks up; Alt held highlights the items, so that Alt with a key of
//   the movement would stop the character; F1 shows the performance overlay and F8 takes a screenshot;
// - the keys of the chat and the stash the game keeps (Ctrl+C, V, A, F, Ctrl+Enter), and Ctrl+Alt+C, the
//   advanced copy;
// - the default keys of Exiled Exchange 2 (`renderer/src/web/Config.ts`, `price-check/PriceCheckWindow.vue`):
//   Ctrl+D, Ctrl+Alt+D, Shift+Space, F5 (/hideout) and F9 (/exit);
// - the systems: Ctrl+F1 to F12 and Alt+F1 to F4 on KDE and GNOME, F12 kept by Windows for its debugger
//   (`RegisterHotKey`), Shift+Tab and F12 of Steam.
// Hence G and the function keys F3 and F4: a letter that no scheme uses, at the same place on QWERTY, AZERTY and
// QWERTZ, and function keys the game leaves free (tried in the owner's game). Ctrl+D stays the default, the key
// players know. Two keys at most (the owner's choice on 2026-11-03): Ctrl+Shift+D, awkward to press in the game,
// left to "Other…"; Ctrl+G, free in Exiled Exchange 2 too, is the key offered when Ctrl+D is taken.
const HL_LIST = [["Ctrl", "D"], ["Ctrl", "G"], ["Alt", "G"], ["F3"], ["F4"]];
const hlSame = (a, b) => a.join("+") === b.join("+");
// The name of a key on its cap: Shift is Maj on the French keyboards and in the French texts of Windows.
const hlKey = fr => key => (fr && key === "Shift" ? "Maj" : key);

// The keys of a shortcut as the list and its button write them: key caps (`look` "caps", the first drawing), or
// a text, "Ctrl+D", in the white button (the looks "a", "b" and "c" asked by the owner on 2026-11-03: the key caps
// kept for the empty window only). The class `app-hotkey-label` marks what a choice copies into the button.
const hlLabel = (combo, fr, look) => (look === "caps"
  ? `<span class="app-hotkey-chip app-hotkey-label">${combo.map(key => `<span class="key">${hlKey(fr)(key)}</span>`).join("+")}</span>`
  : `<span class="app-hotkey-name app-hotkey-label">${combo.map(hlKey(fr)).join("+")}</span>`);

// The list under the keys: each shortcut, the default told, the chosen one checked; "Other…" last, under a line,
// with the key captured when it isn't one of the list. `fr` the French texts.
const hlList = (keys, fr, look) => {
  const caps = combo => hlLabel(combo, fr, look);
  const check = chosen => `<span class="app-hotkey-check">${chosen ? ICON.check(11) : ""}</span>`;
  const row = (combo, index) => `<div class="app-hotkey-option ${hlSame(combo, keys) ? "is-chosen" : ""}">
    ${check(hlSame(combo, keys))}${caps(combo)}${index === 0 ? `<span class="app-hotkey-default">${fr ? "Par défaut" : "Default"}</span>` : ""}</div>`;
  const other = !HL_LIST.some(combo => hlSame(combo, keys));
  return `<div class="app-hotkey-list">${HL_LIST.map(row).join("")}
    <div class="app-hotkey-separator"></div>
    <div class="app-hotkey-option is-other ${other ? "is-chosen" : ""}">${check(other)}<span>${fr ? "Autre…" : "Other…"}</span>${other ? caps(keys) : ""}</div></div>`;
};

// The settings of 14c3 with the keys as a list: a chevron after them, pointing right like the one of the league
// and turned down while the list is open under them (`open`). `look` the drawing of the button (see hlLabel);
// `offer` the free key offered under a warning; `fr` the French window.
const hlWin = ({ keys = ["Ctrl", "D"], open = false, note = HK_HELP, warn = false, offer = null, fr = false, look = "caps" } = {}) => {
  const shift = hlKey(fr);
  const label = look === "caps"
    ? `<span class="app-hotkey-label">${hkKeys(keys.map(shift))}</span>`
    : `<span class="app-hotkey-label">${keys.map(shift).join("+")}</span>`;
  const select = `<span class="app-hotkey-picker ${open ? "is-open" : ""}">
    <span class="app-hotkey-select ${look === "caps" ? "app-hotkey-keys" : `is-button is-${look}`}" title="${fr ? "Changer la touche" : "Change the key"}">${label}${ICON.chevron(11)}</span>
    ${hlList(keys, fr, look)}</span>`;
  const button = offer ? `<span class="app-hotkey-offer"><span class="pill">${fr ? "Utiliser" : "Use"} ${offer.map(shift).join("+")}</span></span>` : "";
  let html = hkWin({ keys, note, warn }).replace(/<span class="app-hotkey-keys [^"]*" title="Change the key">[\s\S]*?<\/span><\/div>/,
    `${select}</div>`).replace("</div>\n    <div class=\"disclaimer\">", `${button}</div>\n    <div class="disclaimer">`);
  if (fr) {
    [["<h3>Settings</h3>", "<h3>Réglages</h3>"], [">Language<", ">Langue<"], ["</i>System<", "</i>Système<"],
      ["Always on top", "Toujours au premier plan"], [">Hotkey<", ">Raccourci<"], ["Price check in the game", "Estimer le prix en jeu"],
      ["This product isn't affiliated with or endorsed by Grinding Gear Games in any way.",
        "Ce produit n'est ni affilié à Grinding Gear Games ni approuvé par ce studio, de quelque manière que ce soit."],
      ["Exile Market 0.1.0 · Made by Sylent", "Exile Market 0.1.0 · Créé par Sylent"]]
      .forEach(([en, french]) => { html = html.replace(en, french); });
  }
  return html;
};

const HL_TAKEN_FR = "<b>Ctrl+D</b> est déjà pris par une autre application.<br>Choisissez une autre touche, ou copiez avec <b>Ctrl+C</b>.";
// The line of the empty window when the key is taken, the free key offered under it rather than the settings.
const hlEmptyTaken = hkEmptyC.replace(`<span>The price shows up here.</span>`, `<span>The price shows up here.</span>
  <span class="ov-note-line warn"><b>Ctrl+D</b> is taken by another application.</span>
  <span class="app-hotkey-offer is-centered"><span class="pill">Use Ctrl+G</span></span>`);

SECTIONS.unshift(["Hotkey list (14c3b2): option C, five safe keys, Other… last, a free key offered", [
  ["Closed: the keys and a chevron pointing right, a click opens the list", hlWin()],
  ["Open: five keys of two keys at most, the default told, Other… last", hlWin({ open: true })],
  ["Open, a key captured by Other… (14c3b3) shown checked on its row", hlWin({ keys: ["Alt", "D"], open: true })],
  ["En français : Par défaut, Autre…, et Maj pour Shift (une touche capturée)", hlWin({ keys: ["Ctrl", "Shift", "D"], open: true, fr: true, note: "Copie l'objet sous le curseur et affiche son prix par-dessus le jeu." })],
  ["Key taken: the next free key of the list offered under the warning", hlWin({ note: HK_TAKEN, warn: true, offer: ["Ctrl", "G"] })],
  ["Key taken, before the first item: the same offer, without opening the settings", win(topBar(), hlEmptyTaken, foot())],
  ["The free key used: grabbed at once, the warning gone", hlWin({ keys: ["Ctrl", "G"] })],
  ["Clé prise, en français", hlWin({ note: HL_TAKEN_FR, warn: true, offer: ["Ctrl", "G"], fr: true })],
]]);

// Section Hotkey list look (14c3b2): the owner's request on 2026-11-03: the key caps, pretty but heavy, and
// their chevron hard to see, are kept for the empty window only; the hotkey of the settings becomes a white button
// in the style of "Use Ctrl+G", its chevron inside. Three tries: A, the text black, the chevron right after it;
// B, the same in gray; C, a classic select of a fixed width, the chevron at its right edge. The list keeps the
// background of the league list, its shortcuts written as text.
SECTIONS.unshift(["Hotkey list look (14c3b2): a white button like Use Ctrl+G, its chevron inside, three tries", [
  ["A · black text, the chevron right after it", hlWin({ look: "a" })],
  ["A · open: the shortcuts as text", hlWin({ look: "a", open: true })],
  ["A · key taken: the button and Use Ctrl+G together", hlWin({ look: "a", note: HK_TAKEN, warn: true, offer: ["Ctrl", "G"] })],
  ["B · gray text, the chevron right after it", hlWin({ look: "b" })],
  ["B · open", hlWin({ look: "b", open: true })],
  ["B · key taken", hlWin({ look: "b", note: HK_TAKEN, warn: true, offer: ["Ctrl", "G"] })],
  ["C · a classic select, the chevron at its right edge", hlWin({ look: "c" })],
  ["C · open", hlWin({ look: "c", open: true })],
  ["C · in French, a key captured by Autre…", hlWin({ look: "c", keys: ["Ctrl", "Shift", "D"], fr: true, note: "Copie l'objet sous le curseur et affiche son prix par-dessus le jeu." })],
  ["Before · the key caps and their chevron", hlWin()],
]]);

// The list works: the keys open and close it, a key chooses it; a click elsewhere or Esc closes it.
document.getElementById("sheet").addEventListener("click", event => {
  const option = event.target.closest(".app-hotkey-option");
  const picker = event.target.closest(".app-hotkey-picker");
  document.querySelectorAll(".app-hotkey-picker.is-open").forEach(open => { if (open !== picker) open.classList.remove("is-open"); });
  if (!picker) return;
  if (option && !option.classList.contains("is-other")) {
    picker.querySelectorAll(".app-hotkey-option").forEach(row => {
      row.classList.toggle("is-chosen", row === option);
      row.querySelector(".app-hotkey-check").innerHTML = row === option ? ICON.check(11) : "";
    });
    picker.querySelector(".app-hotkey-select .app-hotkey-label").innerHTML = option.querySelector(".app-hotkey-label").innerHTML;
    picker.classList.remove("is-open");
  } else if (!option) {
    picker.classList.toggle("is-open");
  }
});
document.addEventListener("keydown", event => {
  if (event.key === "Escape") document.querySelectorAll(".app-hotkey-picker.is-open").forEach(open => open.classList.remove("is-open"));
});
