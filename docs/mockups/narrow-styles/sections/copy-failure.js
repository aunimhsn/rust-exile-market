// Section A copy that fails: "Copy whisper" and "Copy the log" tell when the text didn't reach the clipboard of the
// system, most often because another application holds it for a moment. "Copied" only shows once the system has
// accepted the text. Three ways, compared by eye: A, the pill itself says it and a click tries again; B, the pill
// says it for 2 seconds like "Copied", then offers the copy again; C, the pill doesn't change and a red line under
// it says why. The owner chose A on 2026-11-06: the words stay where the player looks and tell what to do. B may
// go unseen, and the line of C would sometimes be wrong: arboard also fails for other reasons than a busy clipboard.

const CF_TIP = {
  name: "Headhunter", price: "11", currency: "div", meta: "Heavy Belt · ilvl 80 · Corrupted", status: "away", pinned: true,
  implicit: [{ text: "28% increased Stun Threshold", value: 28, yours: 26 }, { text: "Has 2 Charm Slots", value: 2, yours: 2 }],
  mods: [
    { text: "+53 to maximum Life", value: 53, yours: 55 },
    { text: "+36 to Strength", value: 36, yours: 34 },
    { text: "+30 to Dexterity", value: 30, yours: 22 },
    { text: "When you kill a Rare monster, you gain its Modifiers for 60 seconds" },
  ],
};
const cfWhisper = tip => win(topBar(), item(HH), price(HH_PRICE), offers({ ...HH_OFFERS, hover: 4, tooltip: { ...CF_TIP, ...tip } }), foot());

const CF_NOTE = "The clipboard is busy. Try again.";
const CF_NOTE_FR = "Le presse-papiers est occupé. Réessayez.";

SECTIONS.unshift(["A copy that fails: the text didn't reach the clipboard of the system", [
  ["Chosen · A · Copy whisper: the pill says it, a click tries again", cfWhisper({ copyFailed: { pill: "Not copied, try again" } })],
  ["Chosen · A · Copy the log", rpPreviewWin({ copyFailed: { pill: "Not copied, try again" } })],
  ["Chosen · A · Copier le journal, en français", rpPreviewWin({ fr: true, item: "Chasseur de têtes", copyFailed: { pill: "Non copié, réessayer" } })],
  ["Rejected · B · Copy whisper: « Not copied » for 2 seconds, then « Copy whisper » again", cfWhisper({ copyFailed: { pill: "Not copied" } })],
  ["Rejected · B · Copy the log", rpPreviewWin({ copyFailed: { pill: "Not copied" } })],
  ["Rejected · C · Copy whisper: the pill unchanged, a red line under it", cfWhisper({ copyFailed: { note: CF_NOTE } })],
  ["Rejected · C · Copy the log", rpPreviewWin({ copyFailed: { note: CF_NOTE } })],
  ["Rejected · C · Copier le journal, en français", rpPreviewWin({ fr: true, item: "Chasseur de têtes", copyFailed: { note: CF_NOTE_FR } })],
]]);
