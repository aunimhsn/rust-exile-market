// Section Bug report log (16b2): how the player chooses the log sent, this session or the previous one. One log
// is sent at a time (the owner's question on 2026-11-04): both would halve the lines of each in the address, and
// after a crash the session in progress holds only its start. The previous one is chosen at once when its session
// ended without "Exile Market closed"; no choice in the first session. Three ways, compared by eye: tabs on the
// top of the box of the log (the note under it then says that only the log of the tab shown is sent), two pills
// joined on the right of its label, or the radios of 16a. The owner leans to B (2026-11-04: the white tells a
// thing to click), with the white pill sliding to the other word at a click, never when the preview opens: three
// slides to try, by clicking the words.

SECTIONS.unshift(["Bug report log (16b2): this session or the previous one, three ways to choose", [
  ["B · click the words: 180 ms, a slight overshoot", rpPreviewWin({ previous: true, panic: true, choice: "segmented", motion: "overshoot" })],
  ["B · click the words: 220 ms, smooth", rpPreviewWin({ previous: true, panic: true, choice: "segmented", motion: "smooth" })],
  ["B · click the words: 140 ms, quick", rpPreviewWin({ previous: true, panic: true, choice: "segmented", motion: "quick" })],
  ["B · pills, en français : cette session", rpPreviewWin({ fr: true, item: "Chasseur de têtes", choice: "segmented" })],
  ["A · tabs on the box: after a crash, the previous one chosen", rpPreviewWin({ previous: true, panic: true, choice: "tabs" })],
  ["A · tabs, en français : cette session", rpPreviewWin({ fr: true, item: "Chasseur de têtes", choice: "tabs" })],
  ["C · the radios of 16a: after a crash, the previous one chosen", rpPreviewWin({ previous: true, panic: true })],
  ["C · radios, en français : cette session", rpPreviewWin({ fr: true, item: "Chasseur de têtes" })],
]]);

// The white pill under the word chosen: placed at once when the page is drawn, then slid at a click, the log of
// the session chosen shown in the box.
const rpPlaceThumb = segmented => {
  const on = segmented.querySelector("span.on");
  const thumb = segmented.querySelector(".app-report-thumb");
  thumb.style.width = `${on.offsetWidth}px`;
  thumb.style.transform = `translateX(${on.offsetLeft}px)`;
};
window.addEventListener("load", () => document.fonts.ready.then(() => {
  for (const segmented of document.querySelectorAll(".app-report-segmented")) {
    segmented.classList.add("is-still");
    rpPlaceThumb(segmented);
    segmented.offsetWidth; // the place taken before the slide comes back
    segmented.classList.remove("is-still");
  }
}));
document.addEventListener("click", event => {
  const word = event.target.closest(".app-report-segmented span");
  if (!word || word.classList.contains("on")) return;
  const segmented = word.parentElement;
  segmented.querySelector("span.on").classList.remove("on");
  word.classList.add("on");
  rpPlaceThumb(segmented);
  const log = segmented.closest(".band").querySelector(".app-report-box.is-log");
  log.innerHTML = word === segmented.querySelector("span:last-of-type") ? RP_PANIC_LOG : RP_LOG;
});
