// The blocks of the window: title bar, league list, status bar, warnings, empty state, settings.
const leagueList = () => {
  const row = (name, mode, chosen) =>
    `<div class="app-league-row is-${mode} ${chosen ? "is-chosen" : ""}" data-mode="${mode}" title="${mode === "softcore" ? "Softcore" : "Hardcore"}"><span>${name}</span></div>`;
  const pair = (name, index) => row(name, "softcore", index === 0) + row(name, "hardcore", false);
  return `<div class="app-league-list">${["Forbidden Rites", "Runes of Aldur", "Standard"].map(pair).join("")}</div>`;
};
const topBar = (settings = false, league = "Forbidden Rites", open = false) => {
  const kind = document.documentElement.dataset.league === "hardcore" ? "Hardcore" : "Softcore";
  return `
  <div class="app-titlebar ${open ? "is-open" : ""}">
    <img class="app-emblem" src="../../assets/logo/emblem.svg" alt="Home" title="Home">
    <div class="app-league" title="${kind} · Change the league"><span class="app-league-name">${league}</span>${ICON.chevron(12)}</div>
    <div class="spacer"></div>
    <div class="app-icon-button ${settings ? "is-on" : ""}" title="Settings">${ICON.gear(15)}</div>
    <div class="app-icon-button" title="Minimize">${svg(13, '<path d="M3 8h10"/>')}</div>
    <div class="app-icon-button" title="Close">${svg(13, '<path d="M4 4l8 8M12 4l-8 8"/>')}</div>
  </div>${leagueList()}`;
};

const foot = (value = "poe2scout 12min ago · GGG trade 2min ago") => `<div class="foot a">${value}</div>`;
// One line per warning, in the same band.
const banner = (...values) => `<div class="banner"><span>${values.map(value => `<span>${value}</span>`).join("")}</span></div>`;
const newLeague = league => `
  <div class="app-notice"><span class="app-notice-text"><b>${league}</b> has started.</span>
    <span class="pill">Switch</span><span class="app-notice-close" title="Close">${svg(12, '<path d="M4 4l8 8M12 4l-8 8"/>')}</span></div>`;

// The download of the prices of `league`, `percent` done, in place of the invitation to copy, with
// one of the sentences drawn at random at each download.
const downloading = (league, percent, quote) => `<div class="empty b"><div class="dl">
  <span>Downloading the prices of <span class="dl-league">${league}</span></span>
  <div class="dl-bar" style="--dl: ${percent}%"><span></span></div>
  <span class="dl-quote">${quote}</span>
</div></div>`;
const empty = `<div class="empty b"><span>Copy an item in the game with</span><div class="keys"><span class="key">Ctrl</span>+<span class="key">C</span></div><span>The price shows up here.</span></div>`;

const settings = `<div class="band b settings">
    <h3>Settings</h3>
    <div class="field"><span class="label">Language</span>
      <div class="radio on"><i></i>System</div><div class="radio"><i></i>English</div><div class="radio"><i></i>Français</div></div>
    <div class="field"><div class="toggle"><span class="check"></span>Always on top</div></div>
    <div class="disclaimer">This product isn't affiliated with or endorsed by Grinding Gear Games in any way.</div>
  </div>`;

const win = (...parts) => `<div class="win">${parts.join("")}</div>`;
