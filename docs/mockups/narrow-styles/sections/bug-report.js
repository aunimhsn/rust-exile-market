// Section Bug report (16): a way to report a bug from the application, so that the first players' bugs reach the
// owner. The report opens the issue form of the repository in the browser, prefilled through its address
// (`/issues/new?template=bug-report.yml&…`), with what the application knows: the version, the system, the
// languages, the settings and the last lines of the log; the player signs in to their own GitHub account,
// describes what happened there, and sends it. No GitHub token in the application.
// The player sees the report before it opens, as it will be sent: a preview in place of what the window shows.
// The address has a limit: GitHub answers it from about 6,900 characters (500, then 414 from 9,000; measured on
// 2026-11-04), so the report keeps under about 6,000 encoded characters: the log keeps its last lines that fit,
// written short (the time only, without the date), and "Copy the log" copies the whole of it, to paste into the
// issue when it matters.
// The names of the windows other than the game (`steam`, `firefox`, `discord`…), which the log writes since step
// 14c1, are left out of the report, written "another window": they never help with a bug and tell what the player
// runs. The texts copied outside the game are never logged already.
// The entry, the owner's idea on 2026-11-04: a bug icon in the title bar (Lucide `bug`, like the gear), always
// in sight while the first players find the bugs, and one click from the item where the player sees one (a wrong
// tier, an odd price): the item shown is joined to the report. Made more discreet later, once the application is
// stable and the issues fewer (step 21). The owner's choices on 2026-11-04: the Lucide bug (five drawings compared),
// left of the gear (right of it rejected), and GitHub only: a Discord channel can't be prefilled (no address
// writes a message, and a webhook would put a secret in the application), nor follow a bug to its fix. Rejected
// before the bug: A, a link in the status bar of the settings; B, a row of the settings; C, B and a pill under an
// item that couldn't be read.
// With it: the log of the previous session, chosen at once when it ended without "Exile Market closed" (a crash:
// the player opens the application again to report it), the choice hidden in the first session.

const RP_SUMMARY = `Exile Market 0.1.0
Linux · KDE Plasma · X11
Interface: French · Items: French
League: Forbidden Rites (softcore)
Always on top: off · Hotkey: on, F4`;

// The last lines of the log, written short; the names of the other windows left out.
const RP_LOG = `18:47:59 INFO Settings saved
18:48:01 INFO Price-check hotkey F4 listened to while the game is the active window
18:48:03 INFO Copy sent by the price-check hotkey to the game (Ctrl pressed by the application)
18:48:03 INFO Item copied: Classe d'objet: Foci
18:48:04 INFO Item copied: Classe d'objet: Masses à deux mains
18:48:05 INFO Item copied: Classe d'objet: Objets monétaires empilables
18:48:05 INFO Price of Orb of Transmutation: 0.92 ex
18:48:06 INFO Item copied: Classe d'objet: Charmes
18:48:06 INFO Price of Breath of the Mountains: 1.00 ex
18:48:20 INFO Trade site: 59 offers found
18:49:57 INFO Price-check hotkey turned off
18:50:08 INFO Price-check hotkey turned on
18:50:12 INFO Item copied: Classe d'objet: Masses à deux mains
18:51:09 INFO The active window, of class <span class="is-hidden">another window</span>, isn't the game`;
// A previous session that ended on a panic, without "Exile Market closed".
const RP_PANIC_LOG = `21:02:11 INFO Item copied: Item Class: Belts
21:02:11 INFO Price of Headhunter: 12.4 ex
21:02:40 INFO Item copied: Item Class: Wands
<span class="is-panic">21:02:40 ERROR Panic: index out of bounds: the len is 3 but the index is 3 (src/game_data/modifiers.rs:412)</span>`;

const rpBox = (text, log = false) => `<div class="app-report-box ${log ? "is-log" : ""}">${text}</div>`;

// The drawings of the bug, on a 24 px grid like the gear, compared with tabs at the owner's request on 2026-11-04:
// Lucide `bug` (ISC license) chosen, the shape the players know from the other applications, in the stroke of the
// gear; the others, drawn for the mock-up, rejected.
const RP_GLYPHS = {
  lucide: ["Lucide bug", '<path d="m8 2 1.88 1.88"/><path d="M14.12 3.88 16 2"/><path d="M9 7.13v-1a3.003 3.003 0 1 1 6 0v1"/><path d="M12 20c-3.3 0-6-2.7-6-6v-3a4 4 0 0 1 4-4h4a4 4 0 0 1 4 4v3c0 3.3-2.7 6-6 6"/><path d="M12 20v-9"/><path d="M6.53 9C4.6 8.8 3 7.1 3 5"/><path d="M6 13H2"/><path d="M3 21c0-2.1 1.7-3.9 3.8-4"/><path d="M20.97 5c0 2.1-1.6 3.8-3.5 4"/><path d="M22 13h-4"/><path d="M17.2 17c2.1.1 3.8 1.9 3.8 4"/>'],
  // A beetle seen from above: its shell split in two, three legs a side, two antennae.
  beetle: ["Beetle", '<ellipse cx="12" cy="14" rx="5.5" ry="7"/><path d="M9 7.6a3 3 0 0 1 6 0"/><path d="M12 8v13"/><path d="M10.2 5.2 8.5 2.5M13.8 5.2l1.7-2.7"/><path d="M6.6 11 3 9M6.5 14.5H2.5M7.2 18 3.5 20.5M17.4 11 21 9M17.5 14.5h4M16.8 18l3.7 2.5"/>'],
  // The same beetle, its shell filled: heavier, read at once.
  solid: ["Solid beetle", '<ellipse cx="12" cy="14" rx="5.5" ry="7" fill="currentColor"/><path d="M9 7.6a3 3 0 0 1 6 0" fill="currentColor"/><path d="M12 8.5v12" stroke="var(--bg-a)" stroke-width="1.4"/><path d="M10.2 5.2 8.5 2.5M13.8 5.2l1.7-2.7"/><path d="M6.6 11 3 9M6.5 14.5H2.5M7.2 18 3.5 20.5M17.4 11 21 9M17.5 14.5h4M16.8 18l3.7 2.5"/>'],
  // A ladybug: a round shell, its line and four spots.
  ladybug: ["Ladybug", '<circle cx="12" cy="14" r="7.5"/><path d="M8.6 7.6a3.6 3.6 0 0 1 6.8 0"/><path d="M12 6.8v14.6"/><path d="M10.3 5.1 8.8 2.6M13.7 5.1l1.5-2.5"/><circle cx="8.8" cy="12" r="1.1" fill="currentColor" stroke="none"/><circle cx="15.2" cy="12" r="1.1" fill="currentColor" stroke="none"/><circle cx="9" cy="16.8" r="1.1" fill="currentColor" stroke="none"/><circle cx="15" cy="16.8" r="1.1" fill="currentColor" stroke="none"/>'],
  // A spider, closer to the dark world of the game: a body, a head and eight legs bent at the knee.
  spider: ["Spider", '<circle cx="12" cy="14.5" r="3.6"/><circle cx="12" cy="8.6" r="2.2"/><path d="M9.3 12.4 5.5 8.5 4 3.5M14.7 12.4l3.8-3.9L20 3.5"/><path d="M8.5 13.8 4 12.5l-2.5 2M15.5 13.8l4.5-1.3 2.5 2"/><path d="M8.7 15.8 4.5 18l-1 3.5M15.3 15.8l4.2 2.2 1 3.5"/><path d="M10.2 17.6 8.5 21.5M13.8 17.6l1.7 3.9"/>'],
};
// The five drawings side by side, at the size of the title bar and enlarged.
const rpGlyphs = () => `<div class="win app-report-glyphs">${Object.entries(RP_GLYPHS).map(([key, [label, body]]) => {
  const draw = size => `<svg class="ico" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
  return `<div class="app-report-glyph-cell">${draw(48)}${draw(15)}<span>${label}</span></div>`;
}).join("")}</div>`;
const RP_BUG = size => `<svg class="ico" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${RP_GLYPHS.lucide[1]}</svg>`;
// The title bar with the bug: `place` "left" of the gear (chosen on 2026-11-04) or "right" of it (rejected), `on` lit while the report
// is open, `faint` the more discreet look of later, `hover` the pointer over it, its tooltip shown.
const rpTopBar = ({ place = "left", on = false, faint = false, hover = false, settings = false, fr = false } = {}) => {
  const tip = hover ? `<span class="app-tip">${fr ? "Signaler un bug" : "Report a bug"}</span>` : "";
  const bug = `<div class="app-icon-button app-report-bug ${on ? "is-on" : ""} ${faint ? "is-faint" : ""} ${hover ? "is-hovered" : ""}" title="${fr ? "Signaler un bug" : "Report a bug"}">${RP_BUG(15)}${tip}</div>`;
  return topBar(settings).replace(/<div class="app-icon-button[^"]*" title="Settings">[\s\S]*?<\/div>/,
    gear => (place === "left" ? bug + gear : gear + bug));
};

// The slides of the white pill compared (16b2): its duration and its curve, the width following the word. The
// owner chose "smooth" on 2026-11-04, the calmest; the overshoot and the quick one rejected.
const RP_MOTIONS = {
  overshoot: "180ms cubic-bezier(.34, 1.36, .64, 1)",
  smooth: "220ms cubic-bezier(.22, 1, .36, 1)",
  quick: "140ms cubic-bezier(.25, .8, .25, 1)",
};

// The preview, in place of what the window shows: what the report carries, the log of this session or of the
// previous one (no choice in the first session, without a previous log), the item shown, the white "Open on GitHub" and "Copy the log". `previous` the log of the previous
// session chosen (`panic` when it ended on a crash), `item` the name of the item shown (or none; `unreadable` an
// item that couldn't be read, its text joined), `copied` after a click on "Copy the log", `fr` the French window
// (the report stays in English, the language of the issues). `choice` draws the choice of the log (16b2):
// "segmented", two pills joined on the right of its label, the chosen one white (chosen on 2026-11-04); "tabs" on
// the top of its box, or "radios" on the right of its label (16a), rejected. `motion` the slide of the white pill
// at a click, a key of RP_MOTIONS.
const rpPreview = ({ first = false, previous = false, panic = false, item = "Headhunter", unreadable = false, itemOn = true, copied = false, fr = false, choice = "segmented", motion = "smooth" } = {}) => {
  const tx = (en, french) => (fr ? french : en);
  const names = [[!previous, tx("This session", "Cette session")], [previous, tx("The previous one", "La précédente")]];
  const sessions = first || choice === "tabs" ? "" : choice === "segmented"
    ? `<span class="app-report-segmented" style="--slide: ${RP_MOTIONS[motion]}"><i class="app-report-thumb"></i>${names.map(([on, name]) => `<span class="${on ? "on" : ""}">${name}</span>`).join("")}</span>`
    : `<span class="app-report-sessions">${names.map(([on, name]) => `<span class="radio ${on ? "on" : ""}"><i></i>${name}</span>`).join("")}</span>`;
  const tabs = !first && choice === "tabs"
    ? `<div class="app-report-tabs">${names.map(([on, name]) => `<span class="${on ? "on" : ""}">${name}</span>`).join("")}</div>`
    : "";
  const box = `<span class="check ${itemOn ? "on" : ""}">${itemOn ? ICON.check(11) : ""}</span>`;
  const what = unreadable
    ? tx("Join the item that couldn't be read", "Joindre l'objet qui n'a pas pu être lu")
    : `<span>${tx("Join the item shown:", "Joindre l'objet affiché\u00a0:")}</span><b>${item}</b>`;
  // The name cut with "…" when it doesn't fit, the whole sentence in the tooltip (the owner's choice on 2026-11-04, with
  // the colon in place of a comma).
  const whole = unreadable ? "" : ` title="${tx("Join the item shown:", "Joindre l'objet affiché\u00a0:")} ${item}"`;
  const itemRow = item || unreadable ? `<div class="app-report-item"><div class="toggle">${box}<span class="app-report-item-text"${whole}>${what}</span></div></div>` : "";
  const log = previous && panic ? RP_PANIC_LOG : RP_LOG;
  const copy = copied
    ? `<span class="pill is-copied">${ICON.check(11)}${tx("Copied", "Copié")}</span>`
    : `<span class="pill">${tx("Copy the log", "Copier le journal")}</span>`;
  return `<div class="band b settings">
    <h3>${tx("Report a bug", "Signaler un bug")}</h3>
    <div class="app-report-intro">${tx(
      "The report opens on GitHub with the text below. Describe what happened there, then send it.",
      "Le rapport s'ouvre sur GitHub avec le texte ci-dessous. Décrivez-y ce qui s'est passé, puis envoyez-le.")}</div>
    <div class="app-report-label"><span class="label">${tx("Sent with the report", "Envoyé avec le rapport")}</span></div>
    ${rpBox(RP_SUMMARY)}
    <div class="app-report-label"><span class="label">${tx("Log", "Journal")}</span>${sessions}</div>
    ${tabs}${rpBox(log, true)}
    <div class="app-report-note" style="margin-top: 6px">${tabs ? tx(
      "Only the log of the tab shown is sent, its last lines. The other windows are written “another window”.",
      "Seul le journal de l'onglet affiché est envoyé, ses dernières lignes. Les autres fenêtres sont écrites « another window ».") : tx(
      "Its last lines. The other windows are written “another window”.",
      "Ses dernières lignes. Les autres fenêtres sont écrites « another window ».")}</div>
    ${itemRow}
    <div class="app-report-actions"><span class="pill is-main">${tx("Open on GitHub", "Ouvrir sur GitHub")}</span>${copy}</div>
    <div class="app-report-note" style="margin-top: 8px">${tx("A GitHub account is needed.", "Un compte GitHub est nécessaire.")}</div>
  </div>`;
};
const rpPreviewWin = (options = {}) => win(rpTopBar({ on: true, fr: options.fr }), rpPreview(options), foot());

const RP_ITEM = [item(HH), price(HH_PRICE), offers({ state: "idle" }), foot()];
const RP_UNREADABLE = `<div class="band b"><div class="msg">This item couldn't be read.</div><div class="line muted">Copy it again, or copy another item.</div></div>`;

// The entries rejected on 2026-11-04, kept to remember them: A, a link in the status bar of the settings; B, a
// row of the settings with a pill.
const rpSettingsA = () => hlWin().replace(/Exile Market 0\.1\.0 · Made by Sylent · /,
  `Exile Market 0.1.0 · <a href="#">Report a bug</a> · Made by Sylent · `);
const rpSettingsB = () => hlWin().replace(`<div class="disclaimer">`, `<div class="field"><span class="label">Bug report</span>
      <div class="app-report-row"><span class="app-report-note">Opens a report on GitHub, which you read first.</span><span class="pill">Report a bug</span></div></div>
    <div class="disclaimer">`);

SECTIONS.unshift(["Bug report (16): the Lucide bug left of the gear, and the report seen before it opens on GitHub", [
  ["Chosen · the bug left of the gear, over an item", win(rpTopBar(), ...RP_ITEM)],
  ["Chosen · under the pointer: white, its tooltip", win(rpTopBar({ hover: true }), ...RP_ITEM)],
  ["The report, opened from an item: the bug lit, the item joined", rpPreviewWin()],
  ["The previous session ended on a crash: its log chosen at once", rpPreviewWin({ previous: true, panic: true })],
  ["The first session: no previous log, no choice", rpPreviewWin({ first: true })],
  ["Opened over an item that couldn't be read: its text joined", rpPreviewWin({ item: null, unreadable: true })],
  ["Opened before the first item: nothing to join", rpPreviewWin({ item: null })],
  ["A long name (a magic flask): cut with “…”, the whole sentence in its tooltip", rpPreviewWin({ item: "Concentrated Ultimate Life Flask of the Surgeon" })],
  ["En français, un nom long coupé", rpPreviewWin({ fr: true, item: "Flacon de vie ultime concentré du chirurgien" })],
  ["After Copy the log", rpPreviewWin({ copied: true })],
  ["En français : le rapport reste en anglais, la langue des issues", rpPreviewWin({ fr: true, item: "Chasseur de têtes" })],
  ["Before the first item, the bug in the title bar", win(rpTopBar(), hkEmptyD, foot("poe2scout 12min ago"))],
  ["Over an item that couldn't be read", win(rpTopBar(), RP_UNREADABLE, foot())],
  ["Later · more discreet once the issues are fewer: fainter (step 21)", win(rpTopBar({ faint: true }), ...RP_ITEM)],
  ["Chosen · Lucide bug; the other drawings rejected", rpGlyphs()],
  ["Rejected · right of the gear, next to the window buttons", win(rpTopBar({ place: "right" }), ...RP_ITEM)],
  ["Rejected · A · a link in the status bar of the settings", rpSettingsA()],
  ["Rejected · B · a row of the settings", rpSettingsB()],
]]);
