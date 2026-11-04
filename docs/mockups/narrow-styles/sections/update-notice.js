// Section Update notice (15): at launch, the application asks the GitHub releases for the latest version; when it
// is newer than the running one, it tells the player, with a link to its download page (the player installs it;
// the automatic updates are step 20). Closed, the bubble doesn't come back for that version, but a newer one shows
// it again. Without network, or before the first release, nothing is shown. Never over the game: the window over
// the game has no title bar and no bubble.
// Two places drawn: A, the bubble of a new league, under the league block; B, a bubble of the same look under
// the settings icon, where the version lives. A chosen by the owner on 2026-11-04: the two bubbles, when both show,
// stacked and aligned. C, no bubble but a white dot on the settings icon and a line with Download in the settings,
// removed by the owner on 2026-11-04: the dot and the line would stay for as long as the player doesn't update,
// and a Download button has no place in the settings.
// Proposed with A: once the bubble is closed, the settings still tell the new version, a link in their status bar
// after the running one, written like the link of sylent.art beside it (gray, lighter and underlined under the
// pointer), so that a player who closed the bubble finds the version again, without a button.

const UP_CLOSE = `<span class="app-notice-close" title="Close">${svg(12, '<path d="M4 4l8 8M12 4l-8 8"/>')}</span>`;
// The bubble: "Exile Market 0.2.0 is available.", a Download pill that opens the release page in the browser, and
// the cross. `right` under the settings icon (B), otherwise under the league block (A); `below` pushed under the
// bubble of a new league (A, both at once).
const upBubble = ({ right = false, below = false, fr = false } = {}) => `
  <div class="app-notice app-update ${right ? "is-right" : ""} ${below ? "is-below" : ""}"><span class="app-notice-text">${fr
    ? "Exile Market <b>0.2.0</b> est disponible."
    : "Exile Market <b>0.2.0</b> is available."}</span>
    <span class="pill" title="https://github.com/aunimhsn/rust-exile-market/releases/tag/v0.2.0">${fr ? "Télécharger" : "Download"}</span>${UP_CLOSE}</div>`;

// The status bar of the settings, the new version told after the running one, as a link to its page; `strong` in
// the color of the text, the first drawing, otherwise like the link of sylent.art.
const upSettings = ({ fr = false, strong = false } = {}) => hlWin({ fr }).replace(/Exile Market 0\.1\.0 · (Made by|Créé par) Sylent/,
  `Exile Market 0.1.0 · <a class="${strong ? "app-update-link" : ""}" href="https://github.com/aunimhsn/rust-exile-market/releases/latest" target="_blank" rel="noopener" title="https://github.com/aunimhsn/rust-exile-market/releases/latest">${fr ? "0.2.0 disponible" : "0.2.0 available"}</a> · $1 Sylent`);
const UP_ITEM = [item(HH), price(HH_PRICE), offers({ state: "idle" }), foot()];
SECTIONS.unshift(["Update notice (15): A chosen, the bubble under the league block, and the version in the settings", [
  ["Chosen · A · under the league block, like the bubble of a new league", win(topBar(), upBubble(), ...UP_ITEM)],
  ["Chosen · A · with a new league too: the two bubbles stacked, aligned", win(topBar(false, "Standard"), newLeague("Rise of the Abyssal"), upBubble({ below: true }), ...UP_ITEM)],
  ["Chosen · A · before the first item", win(topBar(), upBubble(), hkEmptyD, foot("poe2scout 12min ago"))],
  ["Chosen · A · en français", win(topBar(), upBubble({ fr: true }), ...UP_ITEM)],
  ["Proposed · the bubble closed: the settings still tell it, a link like sylent.art", upSettings()],
  ["Proposed · en français, les réglages", upSettings({ fr: true })],
  ["Before · the link in the color of the text", upSettings({ strong: true })],
  ["Rejected · B · under the settings icon", win(topBar(), upBubble({ right: true }), ...UP_ITEM)],
  ["Rejected · B · with a new league too: under it, on the right", win(topBar(false, "Standard"), newLeague("Rise of the Abyssal"), upBubble({ right: true, below: true }), ...UP_ITEM)],
]]);
