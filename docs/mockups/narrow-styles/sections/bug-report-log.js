// Section Bug report log (16b2): how the player chooses the log sent, this session or the previous one. One log
// is sent at a time (the owner's question on 2026-11-04): both would halve the lines of each in the address, and
// after a crash the session in progress holds only its start. The previous one is chosen at once when its session
// ended without "Exile Market closed"; no choice in the first session. Three ways, compared by eye: tabs on the
// top of the box of the log (the note under it then says that only the log of the tab shown is sent), two pills
// joined on the right of its label, or the radios of 16a.

SECTIONS.unshift(["Bug report log (16b2): this session or the previous one, three ways to choose", [
  ["A · tabs on the box: after a crash, the previous one chosen", rpPreviewWin({ previous: true, panic: true, choice: "tabs" })],
  ["A · tabs, en français : cette session", rpPreviewWin({ fr: true, item: "Chasseur de têtes", choice: "tabs" })],
  ["B · two pills joined: after a crash, the previous one chosen", rpPreviewWin({ previous: true, panic: true, choice: "segmented" })],
  ["B · pills, en français : cette session", rpPreviewWin({ fr: true, item: "Chasseur de têtes", choice: "segmented" })],
  ["C · the radios of 16a: after a crash, the previous one chosen", rpPreviewWin({ previous: true, panic: true })],
  ["C · radios, en français : cette session", rpPreviewWin({ fr: true, item: "Chasseur de têtes" })],
]]);
