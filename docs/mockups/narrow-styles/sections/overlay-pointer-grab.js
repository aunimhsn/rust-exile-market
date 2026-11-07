// Section Window over the game on a single screen (fix of the pointer grab): on a single screen, the game in windowed
// fullscreen (run by Proton) grabs the pointer of X11 a second or two after it becomes the active window, so that
// the window over the game shows the cursor of the game and gets neither the hover nor the clicks (probes in the
// owner's game on 2026-11-07). Wine releases the grab as soon as another window is the active window of KWin, and
// takes it back when the game is active again. So, only while the grab is held, the window over the game makes a
// window of its own the active one when it opens: its cursor, hover and clicks come back, but the keys no longer
// reach the game (its flasks and charms) until a click in the game, which closes the window as before. With two
// screens or the game in a window, nothing changes (section Keyboard in the window over the game, 14b2). How the
// window tells it, three ways: A, a line in the status bar the whole time, in place of the sources; B, the same
// line, and Esc closes the window and gives the keys back to the game; C, the sources stay, and the line only
// shows once a key meant for the game (1 to 5, the flasks and charms) reached the window. The owner chose B on
// 2026-11-07: Esc is the natural key to close a window, and the line tells at once why the flasks don't answer; C
// would only tell it after a flask missed. The window of the Storm Bite of section Window over the game (14b1).

// The window over the game, its status bar holding `note` on the left; its field of the Physical DPS edited
// (a minimum typed, 250) when `typing`.
const pgWindow = ({ note, typing = false } = {}) => {
  let html = OV_ITEMS.bow.window(true);
  if (typing) {
    const at = html.indexOf("Physical DPS: 245");
    const field = html.indexOf(`class="input app-range`, at);
    html = `${html.slice(0, field)}class="input app-range kb-typing${html.slice(field + `class="input app-range`.length)}`;
    html = html.slice(0, field) + html.slice(field).replace(`value="220"`, `value="250"`);
  }
  if (note) html = html.replace(/<div class="foot a ov-foot"><span>[^<]*<\/span>/, `<div class="foot a ov-foot"><span class="kb-note">${note}</span>`);
  return `<div class="ov-real">${html}</div>`;
};

const PG_A = "Keys go to this window · Click the game: back";
const PG_A_FR = "Les touches vont ici · Clic dans le jeu\u00a0: retour";
const PG_B = "Keys go here · Esc or click outside: back to the game";
const PG_B_FR = "Les touches vont ici · Échap ou clic ailleurs\u00a0: retour au jeu";

SECTIONS.unshift(["Window over the game on a single screen: B chosen, the keys go to the window, Esc closes it", [
  ["Chosen · B · Opened: the same line, Esc closes the window too", pgWindow({ note: PG_B })],
  ["Chosen · B · Ouverte, en français", pgWindow({ note: PG_B_FR })],
  ["Chosen · B · A field edited: Esc leaves the field first, a second Esc closes", pgWindow({ note: PG_B, typing: true })],
  ["Rejected · A · Opened: the status bar says where the keys go", pgWindow({ note: PG_A })],
  ["Rejected · A · Ouverte, en français", pgWindow({ note: PG_A_FR })],
  ["Rejected · A · A field edited: the same line", pgWindow({ note: PG_A, typing: true })],
  ["Rejected · C · Opened: the sources, as on two screens", pgWindow()],
  ["Rejected · C · After a key of the game (1 to 5): the line of B", pgWindow({ note: PG_B })],
]]);
