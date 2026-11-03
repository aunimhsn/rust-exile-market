// Section Hotkey setting (14c3): Ctrl+D copies the item under the cursor in the game and shows its price over it
// (14a, 14b), grabbed only while the game is in front (14c1, 14c2). The settings now turn it on or off (14c3a) and
// choose its key (14c3b), drawn here at once so that the whole setting is approved together. A key already taken
// by another application (Exiled Exchange 2 takes Ctrl+D too, the likeliest case) only goes to the log today:
// three places to tell the player were drawn: A, the settings only; B, the empty window; C, a band at the top.
// The key is only known taken once the game comes to the front, when the application tries to grab it; the
// message goes away as soon as the key is grabbed (the other application quit) or the hotkey is turned off. The
// owner's choices on 2026-11-03: B, the line under the invitation of the empty window and the line in the
// settings (C first, then B), its two sentences on two lines; the keys written joined and in bold in the warnings
// (**Ctrl+D**, **Ctrl+C**); the rest of the setting as drawn.

// The keys of a hotkey as key caps, e.g. ["Ctrl", "D"].
const hkKeys = keys => keys.map(key => `<span class="key">${key}</span>`).join("+");

// The settings with the hotkey group: `on` checked or not, `keys` the key caps or `capture` while the player
// presses the new key, `note` a line under the row (`warn` in the color of the warnings).
const hkSettings = ({ on = true, keys = ["Ctrl", "D"], capture = false, note = "", warn = false, help = "" } = {}) => {
  const box = `<span class="check ${on ? "on" : ""}">${on ? ICON.check(11) : ""}</span>`;
  const keyCaps = capture
    // Waiting for the keys: the colors of the text and of the background swapped, pulsing softly (the owner's
    // choice on 2026-11-03, rather than a dashed edge).
    ? `<span class="app-hotkey-keys is-capturing" title="Esc: cancel">Press the keys…</span>`
    : `<span class="app-hotkey-keys ${on ? "" : "is-off"}" title="Change the key">${hkKeys(keys)}</span>`;
  const line = note ? `<span class="app-hotkey-note ${warn ? "warn" : ""}">${note}</span>` : "";
  return `<div class="band b settings">
    <h3>Settings</h3>
    <div class="field"><span class="label">Language</span>
      <div class="radio on"><i></i>System</div><div class="radio"><i></i>English</div><div class="radio"><i></i>Français</div></div>
    <div class="field"><div class="toggle"><span class="check"></span>Always on top</div></div>
    <div class="field">${help ? hkLabel(help) : `<span class="label">Hotkey</span>`}
      <div class="app-hotkey-row"><div class="toggle">${box}Price check in the game</div>${keyCaps}</div>
      ${line}</div>
    <div class="disclaimer">This product isn't affiliated with or endorsed by Grinding Gear Games in any way.</div>
  </div>`;
};
const hkFoot = foot(`<span class="app-author">Exile Market 0.1.0 · Made by Sylent · <a href="https://sylent.art" target="_blank" rel="noopener">sylent.art</a></span>`);
const hkWin = options => win(topBar(true), hkSettings(options), hkFoot);

const HK_HELP = "Copies the item under the cursor and shows its price over the game.";
const HK_BANNER = "<b>Ctrl+D</b> is taken by another application: copy with <b>Ctrl+C</b>, or choose another key in the settings.";
const HK_TAKEN = "<b>Ctrl+D</b> is taken by another application. Choose another key, or copy with <b>Ctrl+C</b>.";

// The invitation of the empty window with the Ctrl + D of the window over the game (14b1), and with Ctrl + C when
// the hotkey is off or taken, so that it never shows a key that does nothing.
const hkEmptyD = ovEmptyNote;
const hkEmptyC = ovEmptyNote.replace(`<span class="key">D</span>`, `<span class="key">C</span>`);
const hkEmptyTaken = hkEmptyC.replace(`<span>The price shows up here.</span>`, `<span>The price shows up here.</span>
  <span class="ov-note-line warn"><b>Ctrl+D</b> is taken by another application.<br>Choose another key in the settings.</span>`);

SECTIONS.unshift(["Hotkey key taken (14c3a): B chosen, a line in the empty window and in the settings", [
  ["Chosen · B, in the empty window: Ctrl + C invited, a line under it", win(topBar(), hkEmptyTaken, foot())],
  ["Chosen · B, and in the settings a line under the row of the hotkey", hkWin({ note: HK_TAKEN, warn: true })],
  ["Rejected · C, a warning band at the top, whatever the window shows", win(topBar(), banner(HK_BANNER), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot())],
]]);
SECTIONS.unshift(["Hotkey setting (14c3): on or off, and its key", [
  ["On, Ctrl + D by default: a click on the keys changes them", hkWin({ note: HK_HELP })],
  ["Choosing the key (14c3b): the next keys pressed, Esc cancels", hkWin({ capture: true, note: "Hold Ctrl or Alt with a letter or a number, or press F2 to F11. Esc: cancel." })],
  ["A key refused (14c3b): the previous one stays", hkWin({ keys: ["Ctrl", "D"], note: "<b>Ctrl+C</b> copies the item already: choose another key.", warn: true })],
  ["Another key chosen (14c3b)", hkWin({ keys: ["Alt", "D"], note: HK_HELP })],
  ["Off: the keys faint, Ctrl + D left to the game", hkWin({ on: false, note: HK_HELP })],
  ["Off, before the first item: Ctrl + C invited", win(topBar(), hkEmptyC, foot())],
  ["On, before the first item (today)", win(topBar(), hkEmptyD, foot())],
]]);

// Section Hotkey rules tooltip (14c3b): the owner's idea on 2026-11-03, a small "?" next to the label "Hotkey"
// which shows, on hover, the keys the player may choose and those refused, with a few examples; a tooltip like the
// others of the application rather than a pop-up. Two contents to compare: A, sentences with the keys in bold; B,
// the examples as key caps under two small labels. Shown open here; it opens on hover in the application.
const hkCaps = keys => `<span class="app-hotkey-chip">${keys.split("+").map(key => `<span class="key">${key}</span>`).join("+")}</span>`;
const HK_TIPS = {
  a: `<span class="app-hotkey-tip-line">Hold <b>Ctrl</b> or <b>Alt</b> with a letter, a number or <span class="app-hotkey-nowrap">F1–F12</span>.</span>
    <span class="app-hotkey-tip-line is-weak">For example <b>Ctrl+Shift+D</b>, <b>Alt+Q</b> or <b>Ctrl+F5</b>.</span>
    <span class="app-hotkey-tip-line is-weak">Refused: a key alone, the Windows key, <b>Ctrl+C</b>, <b>Ctrl+Alt+C</b> and <b>Alt+F4</b>, which the game or the system already use.</span>`,
  b: `<span class="app-hotkey-tip-title">Allowed</span>
    <span class="app-hotkey-tip-line is-weak">Ctrl or Alt with a letter, a number or <span class="app-hotkey-nowrap">F1–F12</span></span>
    <span class="app-hotkey-chips">${["Ctrl+Shift+D", "Alt+Q", "Ctrl+F5"].map(hkCaps).join("")}</span>
    <span class="app-hotkey-tip-title">Refused</span>
    <span class="app-hotkey-tip-line is-weak">A key alone, the Windows key, and the keys of the game or of the system</span>
    <span class="app-hotkey-chips">${["Ctrl+C", "Ctrl+Alt+C", "Alt+F4"].map(hkCaps).join("")}</span>`,
  // The owner's layout on 2026-11-03: two columns of key caps, four combinations of two keys each (the examples
  // to check in 14c3b against the default keys of Path of Exile 2 and of Exiled Exchange 2), then the function keys
  // on a line of their own. Refused lists only what can't work (option 1, the owner's choice on 2026-11-03): Ctrl+C,
  // the copy the application sends itself, and the keys the system keeps (Alt+Tab, the Windows key, Ctrl+Esc); a key
  // that works but takes a function from the game or the system (Ctrl+V, Alt+F4, Ctrl+F) is accepted, with a line
  // in the color of the warnings under the hotkey once chosen. Tried in the owner's game on 2026-11-03 (Bazzite
  // KDE): F1 shows the resources the game uses (accepted, with a warning); F2 to F11 do nothing (accepted alone,
  // the line of the function keys); Alt+F2 opens the search of KDE, which takes it before XWayland, so that the grab
  // succeeds and never hears it (refused); Alt+F4 closes the game (refused, no longer a warning). F12 is kept by
  // Windows for its debugger (`RegisterHotKey`, to check in 14c3b): refused alone. Then the owner's choice on
  // 2026-11-03: the second column is the "Danger zone", in yellow, the keys accepted with a warning (Ctrl+V, Ctrl+F,
  // Alt+F4, F1), the title matching what the application does with them; the keys that can't work (Ctrl+C,
  // Alt+Tab, Alt+F2, the Windows key, F12 alone, a letter alone) leave the tooltip, rare, refused when chosen with
  // a line in yellow too (red was tried and dropped: the owner keeps one color for every warning). Corrected in
  // 14c3b2 against the owner's `poe2_production_Config.ini`: Ctrl+E casts a skill of the second bar, Q and 2 a
  // skill and a flask that Alt held (the highlight) would block, so that the examples allowed are those of the list
  // (Ctrl+D, Ctrl+G, Alt+G) and Ctrl+B, Alt+B; F8 takes a screenshot in the game, so that it joins the danger zone
  // and leaves the line of the function keys.
  c: `<span class="app-hotkey-columns">
      <span class="app-hotkey-column"><span class="app-hotkey-tip-title">Allowed</span>
        ${["Ctrl+D", "Ctrl+G", "Alt+G", "Ctrl+B", "Alt+B"].map(hkCaps).join("")}</span>
      <span class="app-hotkey-column"><span class="app-hotkey-tip-title is-danger">Danger zone</span>
        ${["Ctrl+V", "Ctrl+F", "Alt+F4", "F1", "F8"].map(hkCaps).join("")}</span>
    </span>
    <span class="app-hotkey-tip-title">Function keys</span>
    <span class="app-hotkey-chips">${[["F2", "F7"], ["F9", "F11"]].map(([first, last]) =>
      `<span class="app-hotkey-chip"><span class="key">${first}</span><span class="app-hotkey-dots">…</span><span class="key">${last}</span></span>`).join("")}</span>`,
};
// The label "Hotkey" and its "?", the tooltip `help` (a key of HK_TIPS) open under it.
const hkLabel = help => `<span class="app-hotkey-label"><span class="label">Hotkey</span>
  <span class="app-hotkey-help is-hovered" title="">${svg(13, '<circle cx="8" cy="8" r="6.2"/><path d="M6.3 6.2a1.8 1.8 0 1 1 2.5 1.7c-.5.2-.8.6-.8 1.1v.4"/><path d="M8 11.4v.1"/>')}
    <span class="app-tip app-hotkey-tip">${HK_TIPS[help]}</span></span></span>`;

SECTIONS.unshift(["Hotkey rules tooltip (14c3b): allowed and danger zone, the keys tried in the game", [
  ["Two columns: five allowed, five in the danger zone (accepted with a warning), then F2 … F7 and F9 … F11 alone", hkWin({ help: "c", note: HK_HELP })],
  ["A risky key chosen: accepted, a warning under it", hkWin({ keys: ["Ctrl", "V"], note: "<b>Ctrl+V</b> no longer pastes in the game while it is your hotkey.", warn: true })],
  ["Another risky key chosen, F1 alone", hkWin({ keys: ["F1"], note: "<b>F1</b> no longer shows the resources the game uses while it is your hotkey.", warn: true })],
  ["F8 chosen: accepted, the screenshots of the game lost (14c3b2)", hkWin({ keys: ["F8"], note: "<b>F8</b> no longer takes screenshots in the game while it is your hotkey.", warn: true })],
  ["A key that can't work, refused, the previous one kept (out of the tooltip)", hkWin({ keys: ["Ctrl", "D"], note: "<b>Ctrl+C</b> copies the item already: choose another key.", warn: true })],
  ["Before · A, sentences, the keys in bold", hkWin({ help: "a", note: HK_HELP })],
  ["Before · B, the examples as key caps, under two labels", hkWin({ help: "b", note: HK_HELP })],
]]);
