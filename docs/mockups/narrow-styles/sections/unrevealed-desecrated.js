// Section Unrevealed desecrated (fix): a desecrated modifier not revealed yet named "Unrevealed desecrated prefix"
// or "suffix" (French "Préfixe profané non révélé"), so that its green band reads as desecrated, the color of
// the desecrated modifiers (owner, 2026-11-01); before: "Unrevealed prefix". The windows of the section
// Endgame modifiers (13k1), drawn with the new names.
SECTIONS.unshift(["Unrevealed desecrated (fix): the name says desecrated, like its green band", [
  ["English, a prefix", win(topBar(), item({ ...FOCUS_UNREVEALED, stats: focusUnrevealedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["French, a prefix", win(topBar(), item({ ...FOCUS_UNREVEALED_FR, stats: focusUnrevealedStatsFr }, true), price(RARE_ESTIMATE_BEFORE), foot())],
  ["English, a suffix", win(topBar(), item({ ...SCEPTRE_SANCTIFIED, stats: sceptreSanctifiedStats }, true), price(RARE_ESTIMATE_BEFORE), foot())],
]]);
