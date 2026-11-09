// Sections of step 17a, the executable: its icon and the message when its window can't open. Neither is drawn by
// the application itself (css/system.css).
//
// Window can't open (17a): when the window fails to open (no X11 nor XWayland on Linux, no OpenGL 2.1, the least
// egui draws with), the application can't show anything of its own: the message goes to a dialog of the system
// (MessageBox on Windows, kdialog or zenity on Linux, which open on Wayland too, an alert on macOS), to the
// terminal when it was launched from one, and to the log. One message per cause the player can act on, the error
// itself under it; any other cause asks to report it on GitHub with the log, since the bug icon of the window
// isn't there. In the language of the settings, or of the system before the first settings.
//
// Application icon (17a): the picture the system shows for the application (taskbar, window switcher, the .exe in
// the File Explorer), made from the emblem (specifications, Icons and logo). Compared at the sizes of the systems
// (16, 24, 32 and 48 px) on a dark and a light taskbar: the emblem alone in the flat red of the download page; A,
// the white emblem on the dark tile of the title bar; D, the white emblem alone with a thin dark edge for the light
// taskbars. The red chosen by the owner on 2026-11-09 (17a2), the color of exile-market.sylent.tools, which holds on
// both taskbars without an edge, where the white of D fades. The emblem of the title bar stays white, tinted like
// the other icons of the interface: the shape carries the identity, the color of the application icon the brand.
// The icon itself is assets/logo/icon.svg, rendered by scripts/app-icon.py.

const SYS_SIZES = [16, 24, 32, 48];
const sysIcon = (variant, size) => {
  const look = { A: "is-tile is-white", red: "is-bare is-red", D: "is-bare is-white" }[variant];
  return `<span class="sys-icon ${look}" style="width:${size}px;height:${size}px"><span class="sys-emblem"></span></span>`;
};
const sysBar = (variant, tone) => `<div class="sys-bar is-${tone}">${SYS_SIZES.map(size =>
  `<span class="sys-size">${sysIcon(variant, size)}<small>${size} px</small></span>`).join("")}</div>`;
const sysBoard = variant => `<div class="sys-board">${sysBar(variant, "dark")}${sysBar(variant, "light")}
  <div class="sys-large">${sysIcon(variant, 128)}${sysIcon(variant, 64)}</div></div>`;

const SYS_TEXTS = {
  en: {
    display: ["Exile Market can't open its window: it needs X11, or XWayland on a Wayland desktop.",
      "Install XWayland (the package xwayland on most distributions), then launch Exile Market again."],
    opengl: ["Exile Market can't open its window: it needs OpenGL 2.1 or later, which the graphics driver doesn't provide.",
      "Update the graphics driver, then launch Exile Market again."],
    other: ["Exile Market can't open its window.",
      "Report it on GitHub with its log, so that it can be fixed: https://github.com/aunimhsn/rust-exile-market/issues",
      "Log: /home/player/.local/state/exile-market/exile-market.log"],
    details: "Details:",
  },
  fr: {
    display: ["Exile Market ne peut pas ouvrir sa fenêtre : il lui faut X11, ou XWayland sur un bureau Wayland.",
      "Installez XWayland (le paquet xwayland sur la plupart des distributions), puis relancez Exile Market."],
    opengl: ["Exile Market ne peut pas ouvrir sa fenêtre : il lui faut OpenGL 2.1 ou plus récent, que le pilote graphique ne fournit pas.",
      "Mettez à jour le pilote graphique, puis relancez Exile Market."],
    other: ["Exile Market ne peut pas ouvrir sa fenêtre.",
      "Signalez-le sur GitHub avec son journal, pour qu'il soit corrigé : https://github.com/aunimhsn/rust-exile-market/issues",
      "Journal : /home/player/.local/state/exile-market/exile-market.log"],
    details: "Détails :",
  },
};
const SYS_ERRORS = {
  display: "winit EventLoopError: os error: Failed to open connection to X server",
  opengl: "Found no OpenGL configs for the template ConfigTemplate { color_buffer_type: Rgb, … }",
  other: "Failed to create the main window: the operation couldn't be completed",
};
const sysDialog = (cause, lang = "en") => {
  const texts = SYS_TEXTS[lang];
  return `<div class="sys-dialog"><div class="sys-dialog-title">Exile Market</div>
    <div class="sys-dialog-body"><span class="sys-dialog-sign">!</span><div class="sys-dialog-text">
      ${texts[cause].map(line => `<p>${line}</p>`).join("")}
      <p class="is-details">${texts.details} ${SYS_ERRORS[cause]}</p></div></div>
    <div class="sys-dialog-buttons"><span>OK</span></div></div>`;
};

SECTIONS.unshift(["Window can't open (17a): the dialog of the system, one message per cause", [
  ["No X11 nor XWayland (Linux)", sysDialog("display")],
  ["No OpenGL 2.1", sysDialog("opengl")],
  ["Any other cause: report it", sysDialog("other")],
  ["En français : pas d’OpenGL 2.1", sysDialog("opengl", "fr")],
  ["En français : une autre cause", sysDialog("other", "fr")],
]]);

SECTIONS.unshift(["Application icon (17a): the red emblem for the taskbar, the window switcher and the .exe", [
  ["Chosen · red emblem alone, the flat red of the download page", sysBoard("red")],
  ["Rejected · A · white emblem on the dark tile of the title bar", sysBoard("A")],
  ["Rejected · D · white emblem alone, a thin dark edge", sysBoard("D")],
]]);
