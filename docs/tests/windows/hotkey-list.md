# La touche du raccourci choisie dans une liste, sous Windows : les tests (14c3b2)

Pour le testeur Windows. Les touches du raccourci se choisissent maintenant dans les réglages : un bouton en deux parties (les touches sur fond sombre, le chevron dans un carré blanc) ouvre une liste de cinq touches, `Ctrl+D` (par défaut), `Ctrl+G`, `Alt+G`, `F3` et `F4`. Une touche choisie remplace l'ancienne tout de suite, sans relancer Exile Market. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide). Renvoie ensuite le tableau et le fichier de log.

## Avant

1. Ferme Exile Market et Exiled Exchange 2 s'il tourne.
2. Télécharge la nouvelle archive Windows `rust-exile-market-test-x86_64-pc-windows-msvc.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger) et décompresse-la.
3. Si le fichier des réglages existe, `%APPDATA%\exile-market\config\settings.toml` (colle ce chemin dans la barre d'adresse de l'Explorateur), ouvre-le dans le Bloc-notes et supprime la ligne `keys = …` sous `[hotkey]`, pour repartir de Ctrl+D. Enregistre.
4. Lance `rust-exile-market.exe`. Si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même ».
5. Lance Path of Exile 2 en **plein écran fenêtré** (pas en plein écran exclusif), **sans** le lancer en administrateur, et va dans ton repaire.
6. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log`. Il est écrit au fur et à mesure : ouvre-le dans le Bloc-notes à la fin, ou rouvre-le pour voir les nouvelles lignes.

## Les tests

1. **Le bouton et sa liste** : ouvre les réglages (l'engrenage). Clique sur le bouton `Ctrl+D`, puis de nouveau sur lui ; rouvre la liste et clique ailleurs dans la fenêtre ; rouvre-la et appuie sur Échap, puis encore sur Échap.
   ✅ Ouverte, la liste est sous le bouton : `Ctrl+D` coché avec « Default » (« Par défaut » en français) à droite, puis `Ctrl+G`, `Alt+G`, `F3`, `F4`. Le second clic sur le bouton la ferme, le clic ailleurs aussi ; le premier Échap ferme la liste seulement, le second ferme les réglages.

2. **Ctrl+G, choisi dans la liste** : choisis `Ctrl+G`, puis dans le jeu **Ctrl+G** sur un objet de ton inventaire, puis **Ctrl+D** sur un autre.
   ✅ Le bouton montre `Ctrl+G`. Le log dit « Price-check hotkey changed from Ctrl+D to Ctrl+G ». Ctrl+G ouvre la fenêtre par-dessus le jeu avec le prix ; Ctrl+D ne copie plus rien.

3. **Gardé au lancement suivant** : ferme Exile Market et relance-le.
   ✅ Le log dit « Price-check hotkey: Ctrl+G » ; la fenêtre vide invite à copier avec `Ctrl` + `G`, et Ctrl+G copie dans le jeu.

4. **Alt+G, F3 et F4** : choisis `Alt+G`, puis **Alt+G** sur un objet dans le jeu ; fais de même avec `F3`, puis `F4`.
   ✅ Chaque touche ouvre la fenêtre avec le prix, et la précédente ne copie plus. Après Alt+G, Alt lâché, l'infobulle d'un objet survolé est normale (pas avancée).

5. **La touche prise** : choisis `Ctrl+D`, ferme le jeu, lance Exiled Exchange 2 (il prend Ctrl+D), puis le jeu, et reviens dans les réglages d'Exile Market ; choisis ensuite `Ctrl+G` et fais **Ctrl+G** dans le jeu.
   ✅ Avec Ctrl+D, en jaune et sur deux lignes : « **Ctrl+D** is taken by another application. » puis « Choose another key, or copy with **Ctrl+C**. ». Une fois Ctrl+G choisi, le message disparaît, et Ctrl+G copie dans le jeu. Ferme Exiled Exchange 2 ensuite, et rechoisis `Ctrl+D`.

6. **Tes propres scénarios** : essaie ce qui te passe par la tête (changer de touche pendant que la fenêtre par-dessus le jeu est ouverte, plusieurs changements de suite…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le bouton et sa liste | | |
| 2. Ctrl+G, choisi dans la liste | | |
| 3. Gardé au lancement suivant | | |
| 4. Alt+G, F3 et F4 | | |
| 5. La touche prise | | |
| 6. Tes propres scénarios | | |
