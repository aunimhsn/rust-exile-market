// Section Keyboard in the window over the game (14b2): the game keeps the keyboard while the window over the game
// is open (14b1), so that a number typed in a min or max field reaches the game (its flasks and charms on 1 to 5).
// A click in a field gives the keyboard to the window (X11 `SetInputFocus`, as Exiled Exchange 2 does with
// `xcb_set_input_focus`); the keyboard goes back to the game as soon as no field is edited (Enter, Esc, Tab out of
// the last field, a click elsewhere in the window) and when the window closes (a click in the game). The owner's
// choices on 2026-11-01: a click in a field takes it, a click in the game gives it back, and so does leaving the
// field, so that the keys of the flasks work again at once while the window stays open. How the window shows that
// the game no longer gets the keys: option B, chosen by the owner on 2026-11-01, a line in the status bar in place
// of the sources while a field is edited, which says where the keys go and how to give them back (rejected: the
// white edge of the field alone, A, which doesn't say that the game gets nothing; a white edge around the window,
// C). The window of the Storm Bite of section Window over the game (14b1), its stats shown, the field of the
// Physical DPS edited.

// The window over the game with the field of the Physical DPS edited (a minimum typed, 250), its status bar
// holding `foot` on the left when it is given.
const kbWindow = ({ foot: note } = {}) => {
  let html = OV_ITEMS.bow.window(true);
  const at = html.indexOf("Physical DPS: 245");
  const field = html.indexOf(`class="input app-range`, at);
  html = `${html.slice(0, field)}class="input app-range kb-typing${html.slice(field + `class="input app-range`.length)}`;
  html = html.slice(0, field) + html.slice(field).replace(`value="220"`, `value="250"`);
  if (note) html = html.replace(/<div class="foot a ov-foot"><span>[^<]*<\/span>/, `<div class="foot a ov-foot"><span class="kb-note">${note}</span>`);
  return `<div class="ov-real">${html}</div>`;
};

SECTIONS.unshift(["Keyboard in the window over the game (14b2): B chosen, a line in the status bar", [
  ["No field edited: the game keeps the keyboard", `<div class="ov-real">${OV_ITEMS.bow.window(true)}</div>`],
  ["Chosen · B, a field edited: the keys go to the window, the status bar says it", kbWindow({ foot: "Keys go to this window · Enter or Esc: back to the game" })],
]]);
