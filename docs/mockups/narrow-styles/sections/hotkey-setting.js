// Section Hotkey setting (14c3): Ctrl+D copies the item under the cursor in the game and shows its price over it
// (14a, 14b), grabbed only while the game is in front (14c1, 14c2). The settings now turn it on or off (14c3a) and
// choose its key (14c3b), drawn here at once so that the whole setting is approved together. A key already taken
// by another application (Exiled Exchange 2 takes Ctrl+D too, the likeliest case) only goes to the log today:
// three places to tell the player were drawn: A, the settings only; B, the empty window; C, a band at the top.
// The key is only known taken once the game comes to the front, when the application tries to grab it; the
// message goes away as soon as the key is grabbed (the other application quit) or the hotkey is turned off. The
// owner's choices on 2026-11-03: C, the band at the top and the line in the settings; the keys written joined and
// in bold in the warnings (**Ctrl+D**, **Ctrl+C**); the rest of the setting as drawn.

// The keys of a hotkey as key caps, e.g. ["Ctrl", "D"].
const hkKeys = keys => keys.map(key => `<span class="key">${key}</span>`).join("+");

// The settings with the hotkey group: `on` checked or not, `keys` the key caps or `capture` while the player
// presses the new key, `note` a line under the row (`warn` in the color of the warnings).
const hkSettings = ({ on = true, keys = ["Ctrl", "D"], capture = false, note = "", warn = false } = {}) => {
  const box = `<span class="check ${on ? "on" : ""}">${on ? ICON.check(11) : ""}</span>`;
  const keyCaps = capture
    ? `<span class="app-hotkey-keys is-capturing" title="Esc: cancel">Press the keys…</span>`
    : `<span class="app-hotkey-keys ${on ? "" : "is-off"}" title="Change the key">${hkKeys(keys)}</span>`;
  const line = note ? `<span class="app-hotkey-note ${warn ? "warn" : ""}">${note}</span>` : "";
  return `<div class="band b settings">
    <h3>Settings</h3>
    <div class="field"><span class="label">Language</span>
      <div class="radio on"><i></i>System</div><div class="radio"><i></i>English</div><div class="radio"><i></i>Français</div></div>
    <div class="field"><div class="toggle"><span class="check"></span>Always on top</div></div>
    <div class="field"><span class="label">Hotkey</span>
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
  <span class="ov-note-line warn"><b>Ctrl+D</b> is taken by another application: choose another key in the settings.</span>`);

SECTIONS.unshift(["Hotkey key taken (14c3a): C chosen, a band at the top and a line in the settings", [
  ["Chosen · C, a warning band at the top, whatever the window shows", win(topBar(), banner(HK_BANNER), item(HH), price(HH_PRICE), offers({ state: "idle" }), foot())],
  ["Chosen · C, the same band before the first item, Ctrl + C invited", win(topBar(), banner(HK_BANNER), hkEmptyC, foot())],
  ["Chosen · C, and in the settings a line under the row of the hotkey", hkWin({ note: HK_TAKEN, warn: true })],
  ["Rejected · B, in the empty window: Ctrl + C invited, a line under it", win(topBar(), hkEmptyTaken, foot())],
]]);
SECTIONS.unshift(["Hotkey setting (14c3): on or off, and its key", [
  ["On, Ctrl + D by default: a click on the keys changes them", hkWin({ note: HK_HELP })],
  ["Choosing the key (14c3b): the next keys pressed, Esc cancels", hkWin({ capture: true, note: "Hold Ctrl or Alt with a letter, a number or a function key. Esc: cancel." })],
  ["A key refused (14c3b): the previous one stays", hkWin({ keys: ["Ctrl", "D"], note: "<b>Ctrl+C</b> copies the item already: choose another key.", warn: true })],
  ["Another key chosen (14c3b)", hkWin({ keys: ["Alt", "D"], note: HK_HELP })],
  ["Off: the keys faint, Ctrl + D left to the game", hkWin({ on: false, note: HK_HELP })],
  ["Off, before the first item: Ctrl + C invited", win(topBar(), hkEmptyC, foot())],
  ["On, before the first item (today)", win(topBar(), hkEmptyD, foot())],
]]);
