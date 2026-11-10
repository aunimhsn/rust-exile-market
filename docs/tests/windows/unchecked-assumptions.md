# Trois cas jamais essayés sur Windows : les tests

## Pourquoi

Le code du raccourci et de la fenêtre par-dessus le jeu suppose trois choses sur Windows qu'aucun test n'a encore vérifiées (revue du bureau du 2026-11-10, hypothèses 10 à 12). Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche, avec le journal (`%LOCALAPPDATA%\exile-market\data\exile-market.log`, ou le bouton du rapport de bug).

## Avant

1. Ferme Exile Market et tout autre outil de prix.
2. Lance Exile Market, puis le jeu en **plein écran fenêtré**, dans ton repaire.

## Les tests

1. **Le jeu lancé en administrateur** (hypothèse 10) : ferme le jeu, relance-le **en tant qu'administrateur** (ou Steam en administrateur), Exile Market restant lancé normalement, puis **Ctrl+D** sur un objet.
   ✅ La fenêtre s'ouvre avec l'objet. ❌ Si rien ne s'ouvre alors que le journal dit « Copy requested by the price-check hotkey… » : note-le, et regarde si le texte que tu avais copié avant est encore dans le presse-papiers (colle-le dans le Bloc-notes).

2. **Un clic dans la fenêtre** (hypothèse 11) : avec de la musique dans le jeu, **Ctrl+D** sur un objet rare, puis clique plusieurs fois sur des **cases à cocher** de la fenêtre.
   ✅ Le son du jeu ne se coupe pas et l'image ne saccade pas à chaque clic.

3. **Ctrl+D juste après le retour au jeu** (hypothèse 12) : **Alt+Tab** vers une autre fenêtre, **Alt+Tab** vers le jeu, et appuie sur **Ctrl+D** sur un objet le plus vite possible ; recommence cinq fois.
   ✅ La fenêtre s'ouvre à chaque fois. ❌ Si une fois rien ne s'ouvre et que le jeu reçoit la touche `D` : note combien de fois sur cinq.

4. **Test libre** : ce qui te passe par la tête autour de ces trois cas.
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le jeu en administrateur | | |
| 2. Un clic dans la fenêtre | | |
| 3. Ctrl+D juste après le retour | | |
| 4. Test libre | | |
