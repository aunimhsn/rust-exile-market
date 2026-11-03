# La touche du raccourci choisie dans une liste (14c3b2) : les tests

## Pourquoi

Les touches du raccourci se choisissent maintenant dans les réglages : un bouton en deux parties (les touches sur fond sombre, le chevron dans un carré blanc) ouvre une liste de cinq touches sûres, `Ctrl+D` (par défaut), `Ctrl+G`, `Alt+G`, `F3` et `F4`. Une touche choisie est enregistrée et remplace l'ancienne tout de suite, sans relancer Exile Market : l'ancienne est rendue au jeu et aux autres applications, la nouvelle est prise dès que le jeu passe au premier plan. Ces tests vérifient la liste, le changement à chaud, et le message de la touche prise, maintenant sur deux lignes. Fais-les d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

## Avant

1. Ferme Exile Market, Exiled Exchange 2 et le jeu.
2. Dans le fichier des réglages, `~/.config/exile-market/settings.toml`, supprime la ligne `keys = …` sous `[hotkey]` (elle vaut `Alt+D` chez toi), pour repartir de Ctrl+D. Enregistre.
3. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le worktree donné avec la PR).

4. Puis lance le jeu par Steam, en **plein écran fenêtré**, et va dans ton repaire.

## Les tests

1. **Le bouton et sa liste** : ouvre les réglages (l'engrenage). Clique sur le bouton `Ctrl+D`, puis de nouveau sur lui ; rouvre la liste et clique ailleurs dans la fenêtre ; rouvre-la et appuie sur Échap, puis encore sur Échap.
   ✅ Le bouton montre `Ctrl+D` sur fond sombre et un chevron vers la droite dans un carré blanc ; la partie sombre s'éclaircit sous la souris. Ouverte, la liste est sous le bouton, alignée sur sa droite, le chevron tourné vers le bas : `Ctrl+D` coché avec « Default » à droite, puis `Ctrl+G`, `Alt+G`, `F3`, `F4`. Le second clic sur le bouton la ferme, le clic ailleurs aussi ; le premier Échap ferme la liste seulement, le second ferme les réglages.

2. **Ctrl+G, choisi dans la liste** : dans les réglages, choisis `Ctrl+G`. Puis dans le jeu, **Ctrl+G** sur un objet de ton inventaire, puis **Ctrl+D** sur un autre.
   ✅ La liste se ferme et le bouton montre `Ctrl+G`. Le journal dit « Price-check hotkey changed from Ctrl+D to Ctrl+G », puis, le jeu au premier plan, « Price-check hotkey Ctrl+G listened to while the game is the active window ». Ctrl+G ouvre la fenêtre par-dessus le jeu avec le prix ; Ctrl+D ne copie plus rien (le jeu le reçoit).

3. **Gardé au lancement suivant** : ferme Exile Market, regarde le fichier des réglages, relance-le.
   ✅ Le fichier dit `keys = "Ctrl+G"` ; au lancement, le journal dit « Price-check hotkey: Ctrl+G », la fenêtre vide invite à copier avec `Ctrl` + `G`, et Ctrl+G copie dans le jeu.

4. **Alt+G** : choisis `Alt+G`, puis dans le jeu **Alt+G** sur un objet, puis sur trois objets en tenant Alt. Alt lâché, survole un objet.
   ✅ Chaque G ouvre la fenêtre avec le prix ; le journal dit « (Ctrl pressed by the application) ». Alt lâché, l'infobulle de l'objet survolé est normale (pas avancée).

5. **F3 et F4** : choisis `F3`, puis **F3** sur un objet dans le jeu ; puis choisis `F4` et fais de même.
   ✅ F3, puis F4, ouvrent la fenêtre avec le prix, sans rien faire d'autre dans le jeu. Après le passage à F4, F3 ne copie plus.

6. **La touche prise, sur deux lignes** : choisis `Ctrl+D`, ferme le jeu, lance Exiled Exchange 2 (il prend Ctrl+D), puis le jeu, et reviens dans les réglages d'Exile Market.
   ✅ Sous la ligne du raccourci, en jaune et sur deux lignes : « **Ctrl+D** is taken by another application. » puis « Choose another key, or copy with **Ctrl+C**. ». La liste s'ouvre toujours au-dessus.

7. **Une autre touche quand elle est prise** : toujours avec Exiled Exchange 2 ouvert, choisis `Ctrl+G` dans la liste, puis dans le jeu **Ctrl+G** sur un objet.
   ✅ Le message jaune disparaît dès le choix, et la ligne grise « Copies the item under the cursor… » revient. Dans le jeu, Ctrl+G ouvre la fenêtre d'Exile Market avec le prix, et Ctrl+D reste à Exiled Exchange 2. Ferme Exiled Exchange 2 ensuite.

8. **Raccourci désactivé** : décoche « Price check in the game », choisis `F4` dans la liste, puis **F4** dans le jeu ; recoche la case, puis **F4** dans le jeu.
   ✅ Décoché, le texte du bouton pâlit mais la liste s'ouvre toujours ; F4 ne copie pas. Recoché, F4 copie dans le jeu.

9. **En français, Maj** : ferme Exile Market, écris `keys = "Ctrl+Shift+D"` sous `[hotkey]` dans le fichier, choisis le français si ce n'est pas déjà fait (ligne `language = "french"`), et relance-le.
   ✅ La fenêtre vide invite à copier avec `Ctrl` + `Maj` + `D` ; dans les réglages, le bouton montre `Ctrl+Maj+D`, la liste n'a aucune ligne cochée et dit « Par défaut » à droite de `Ctrl+D`. Ctrl+Shift+D copie dans le jeu. Choisis ensuite `Ctrl+D` dans la liste pour finir.

10. **Tes propres scénarios** : essaie ce qui te passe par la tête (changer de touche pendant que la fenêtre par-dessus le jeu est ouverte, plusieurs changements de suite, la liste au clavier avec Tab et Entrée…).
    ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le bouton et sa liste | | |
| 2. Ctrl+G, choisi dans la liste | | |
| 3. Gardé au lancement suivant | | |
| 4. Alt+G | | |
| 5. F3 et F4 | | |
| 6. La touche prise, sur deux lignes | | |
| 7. Une autre touche quand elle est prise | | |
| 8. Raccourci désactivé | | |
| 9. En français, Maj | | |
| 10. Tes propres scénarios | | |
