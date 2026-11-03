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

1. **Le bouton et sa liste** : ouvre les réglages (l'engrenage). Clique sur le bouton `Ctrl+D`, puis de nouveau sur lui ; rouvre la liste et clique sur une zone vide de la fenêtre ; rouvre-la et coche « Always on top » (« Toujours au premier plan »), puis rouvre-la et décoche-le ; rouvre-la et appuie sur Échap, puis encore sur Échap. Enfin, rouvre la liste, ferme les réglages avec l'engrenage et rouvre-les.
   ✅ Ouverte, la liste est sous le bouton : `Ctrl+D` coché avec « Default » (« Par défaut » en français) à droite, puis `Ctrl+G`, `Alt+G`, `F3`, `F4`. Le second clic sur le bouton la ferme, le clic sur la zone vide aussi ; le clic sur « Always on top » ferme la liste **et** coche ou décoche la case ; le premier Échap ferme la liste seulement, le second ferme les réglages. Rouverts, les réglages n'ont pas la liste, et rien ne s'efface en fondu sous le bouton.

2. **La liste au clavier** : dans les réglages, appuie sur Tab jusqu'à ce que le bouton `Ctrl+D` s'éclaircisse, puis Entrée. Appuie sur la flèche du haut, puis cinq fois sur la flèche du bas, puis Entrée. Rouvre la liste avec Entrée, et appuie sur Échap.
   ✅ Entrée ouvre la liste, la ligne `Ctrl+D` éclairée. La flèche du haut reste sur `Ctrl+D` ; les flèches du bas descendent ligne par ligne et s'arrêtent sur `F4`, sans sortir de la liste. Entrée choisit `F4` et ferme la liste ; le bouton reste éclairci, Entrée le rouvre ; Échap la ferme, et le bouton reste éclairci. Choisis ensuite `Ctrl+D` à la souris.

3. **Ctrl+G, choisi dans la liste** : choisis `Ctrl+G`, puis dans le jeu **Ctrl+G** sur un objet de ton inventaire, puis **Ctrl+D** sur un autre.
   ✅ Le bouton montre `Ctrl+G`. Le log dit « Price-check hotkey changed from Ctrl+D to Ctrl+G ». Ctrl+G ouvre la fenêtre par-dessus le jeu avec le prix ; Ctrl+D ne copie plus rien.

4. **Gardé au lancement suivant** : ferme Exile Market et relance-le.
   ✅ Le log dit « Price-check hotkey: Ctrl+G » ; la fenêtre vide invite à copier avec `Ctrl` + `G`, et Ctrl+G copie dans le jeu.

5. **Alt+G, F3 et F4** : choisis `Alt+G`, puis **Alt+G** sur un objet dans le jeu ; fais de même avec `F3`, puis `F4`.
   ✅ Chaque touche ouvre la fenêtre avec le prix, et la précédente ne copie plus. Après Alt+G, Alt lâché, l'infobulle d'un objet survolé est normale (pas avancée).

6. **La touche prise** : choisis `Ctrl+D`, ferme le jeu, lance Exiled Exchange 2 (il prend Ctrl+D), puis le jeu, et reviens dans les réglages d'Exile Market. Si le message n'apparaît pas, repasse au jeu puis reviens, jusqu'à ce qu'il apparaisse : les deux applications se disputent Ctrl+D à chaque retour au jeu, et la première la garde. Message affiché, réduis la fenêtre à sa hauteur minimale (bord du bas vers le haut) et ouvre la liste ; choisis ensuite `Ctrl+G` et fais **Ctrl+G** dans le jeu.
   ✅ Avec Ctrl+D, en jaune et sur deux lignes : « **Ctrl+D** is taken by another application. » puis « Choose another key, or copy with **Ctrl+C**. ». À la hauteur minimale, la liste tient entière dans la fenêtre, sous le bouton, sans le recouvrir. Une fois Ctrl+G choisi, le message disparaît, et Ctrl+G copie dans le jeu. Ferme Exiled Exchange 2 ensuite, et rechoisis `Ctrl+D`.

7. **Tes propres scénarios** : essaie ce qui te passe par la tête (changer de touche pendant que la fenêtre par-dessus le jeu est ouverte, plusieurs changements de suite…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le bouton et sa liste | | |
| 2. La liste au clavier | | |
| 3. Ctrl+G, choisi dans la liste | | |
| 4. Gardé au lancement suivant | | |
| 5. Alt+G, F3 et F4 | | |
| 6. La touche prise | | |
| 7. Tes propres scénarios | | |
