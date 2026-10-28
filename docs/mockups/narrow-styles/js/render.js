// The tabs, the sheet of the sections, and the league list that works.
// One group of tabs per variant; the address keeps the choices, to share them.
function render() {
  theme = chosen.theme;
  for (const key of Object.keys(VARIANTS)) document.documentElement.dataset[key] = chosen[key];
  const t = THEMES[theme];
  document.getElementById("styles").innerHTML = Object.entries(VARIANTS).map(([key, v]) =>
    `<div class="group"><span>${v.label}</span>${Object.entries(v.options).map(([value, label]) =>
      `<button type="button" data-variant="${key}" data-value="${value}" aria-pressed="${chosen[key] === value}">${label}</button>`).join("")}</div>`).join("");
  // Past ten sections, a table of contents under the header links to each, so that the owner sees them
  // all at once: the title of each section up to its colon.
  const contents = SECTIONS.length > 10
    ? `<nav class="contents" aria-label="Sections">${SECTIONS.map(([title], index) =>
      `<a href="#section-${index + 1}">${title.split(":")[0]}</a>`).join("")}</nav>`
    : "";
  document.getElementById("sheet").innerHTML = `
    <header><h1>Style ${t.title}</h1><p>${t.text} Narrow layout, 420 px. Sample data.</p>${contents}</header>
    ${SECTIONS.map(([title, slots], index) => `<section class="section" id="section-${index + 1}"><h2>${title}</h2><div class="row">${slots.map(([label, html]) => `<div class="slot"><span>${label}</span>${html}</div>`).join("")}</div></section>`).join("")}`;
}

document.getElementById("styles").addEventListener("click", event => {
  const { variant, value } = event.target.dataset;
  if (!variant) return;
  chosen[variant] = value;
  history.replaceState(null, "", "?" + new URLSearchParams(chosen));
  render();
});
// The league list works: the league block opens and closes it, a league chooses it; a click elsewhere or
// Escape closes it.
const closeLeagueLists = except => document.querySelectorAll(".app-titlebar.is-open").forEach(bar => {
  if (bar !== except) bar.classList.remove("is-open");
});
document.getElementById("sheet").addEventListener("click", event => {
  const block = event.target.closest(".app-league");
  const row = event.target.closest(".app-league-row");
  if (block) {
    const bar = block.closest(".app-titlebar");
    closeLeagueLists(bar);
    bar.classList.toggle("is-open");
  } else if (row) {
    const win = row.closest(".win");
    win.querySelector(".app-league-name").textContent = row.textContent;
    win.dataset.league = row.dataset.mode;
    win.querySelectorAll(".app-league-row.is-chosen").forEach(other => other.classList.remove("is-chosen"));
    row.classList.add("is-chosen");
    win.querySelector(".app-league").title = `${row.title} · Change the league`;
    closeLeagueLists();
  } else {
    closeLeagueLists();
  }
});
document.addEventListener("keydown", event => { if (event.key === "Escape") closeLeagueLists(); });
document.getElementById("file-warning").hidden = location.protocol !== "file:";
render();
