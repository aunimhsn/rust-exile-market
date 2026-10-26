// The styles and the variants read from the address, the orbs, the icons, and the sections to fill.
const THEMES = {
  forge: { title: "A · Forge", text: "Close to the game: engraved capitals for the item names (Cinzel), a sober text face (Source Sans 3), gold accent, warm near-black grays." },
  slate: { title: "B · Slate", text: "Modern and sharp: one geometric face (Manrope), numbers in a monospace face (JetBrains Mono), white as the main color so that the colors stay for the item rarities, cool near-black grays." },
  compact: { title: "C · Compact", text: "Dense, made for the narrow width: condensed capitals for the names (Barlow Semi Condensed), Barlow for the text, amber accent, neutral near-black grays." },
};
const VARIANTS = {
  theme: { label: "Style", options: Object.fromEntries(Object.entries(THEMES).map(([k, v]) => [k, v.title])), fallback: "slate" },
  league: { label: "League", options: { softcore: "Softcore", hardcore: "Hardcore" }, fallback: "softcore" },
};
const params = new URLSearchParams(location.search);
const chosen = Object.fromEntries(Object.entries(VARIANTS).map(([key, v]) => [key, params.get(key) in v.options ? params.get(key) : v.fallback]));
let theme = chosen.theme;

const ORB = { div: "divine-orb", chaos: "chaos-orb", ex: "exalted-orb" };
const orb = key => `<img src="../../assets/currencies/${ORB[key]}.png" alt="">`;

// Icons, drawn with the current text color.
const svg = (size, body) => `<svg class="ico" width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${body}</svg>`;
const ICON = {
  search: size => svg(size, '<circle cx="7" cy="7" r="4.6"/><path d="M10.4 10.4 14 14"/>'),
  refresh: size => svg(size, '<path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9"/><path d="M13.6 2.6v3.3h-3.3"/>'),
  chevron: size => svg(size, '<path d="M6 4l4 4-4 4"/>'),
  down: size => svg(size, '<path d="M4 6l4 4 4-4"/>'),
  downDouble: size => svg(size, '<path d="M4 3.5l4 4 4-4"/><path d="M4 8.5l4 4 4-4"/>'),
  check: size => `<svg class="ico" width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3.2 8.4l3.1 3.1 6.5-7"/></svg>`,
  // The "settings" icon of Lucide (ISC license), on its 24 px grid.
  gear: size => `<svg class="ico" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/></svg>`,
};

// Filled by the files of sections/, in the order of their script tags.
const SECTIONS = [];
