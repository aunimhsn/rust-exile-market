# Ctrl+D dans le jeu seulement (14c1) : les tests

## Pourquoi

Jusqu'ici, Ctrl+D était pris dans toutes les fenêtres X11 (le client Steam, la fenêtre principale d'Exile Market) et y devenait Ctrl+C. Maintenant, la touche n'est prise que pendant que le jeu est la fenêtre active ; ailleurs, Ctrl+D reste à la fenêtre. Fais les tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

## Avant

1. Ferme Exile Market, Exiled Exchange 2 et le jeu.
2. Lance, dans un terminal :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR).

3. Puis lance le jeu par Steam, en **plein écran fenêtré**, et va dans ton repaire.

## Les tests

1. **Jeu lancé après l'application** : dès que tu es en jeu, sans Alt+Tab, ouvre l'inventaire et fais **Ctrl+D** sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec son prix, et le journal dit une fois **« Price-check hotkey Ctrl+D listened to while the game is the active window »**.

2. **La fenêtre principale** : clique dans la fenêtre principale d'Exile Market, puis fais **Ctrl+D**.
   ✅ Rien ne se passe ; le journal écrit une fois « The active window, of class rust-exile-market, isn't the game », mais **aucune** ligne « Copy sent by the price-check hotkey … ».

3. **Le client Steam** : fais Alt+Tab vers le client Steam, puis **Ctrl+D**.
   ✅ Le journal écrit une fois la classe du client Steam (« … of class steam … isn't the game »), mais aucune ligne « Copy sent … ».

4. **Une fenêtre Wayland** : ouvre **Kate** (une application KDE, donc Wayland), écris une ligne, fais Alt+Tab du jeu vers Kate, puis **Ctrl+D** sur la ligne. Note aussi la valeur de **Configuration du système › Applications et fenêtres › Gestion des fenêtres › Prise en charge des applications X11 historiques** (le titre peut varier un peu).
   ✅ Kate fait son propre Ctrl+D (la ligne est mise en commentaire), et le journal n'écrit aucune ligne « Copy sent … ».

5. **Retour au jeu** : reviens au jeu (Alt+Tab ou un clic), puis aussitôt **Ctrl+D** sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre du premier coup.

6. **Plusieurs Alt+Tab** : fais cinq allers-retours entre le jeu et une autre fenêtre, puis **Ctrl+D** dans le jeu.
   ✅ La copie marche, et le journal n'a pas écrit une ligne par Alt+Tab (ni « listened to », ni « can't be listened to », ni la classe d'une fenêtre déjà écrite).

7. **Pendant une saisie** : **Ctrl+D** sur un objet rare, clique dans le champ **min** d'une stat, tape `5`, puis **Ctrl+D sur un autre objet**.
   ✅ La fenêtre montre l'autre objet : la touche reste prise tant que la fenêtre par-dessus le jeu a le clavier.

8. **Exiled Exchange 2** (seulement si tu l'as) : quitte le jeu, lance Exiled Exchange 2, relance le jeu, puis **Ctrl+D** sur un objet. Quitte ensuite Exiled Exchange 2, fais Alt+Tab hors du jeu et retour, puis **Ctrl+D**.
   ✅ Note ce qui s'ouvre la première fois (Exile Market, Exiled Exchange 2, ou les deux) et ce que dit le journal ; la seconde fois, Exile Market s'ouvre.

9. **Tes propres scénarios** : essaie ce qui te passe par la tête (le jeu fermé puis relancé, Ctrl+D tenu pendant un Alt+Tab, un navigateur…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Jeu lancé après l'application | | |
| 2. La fenêtre principale | | |
| 3. Le client Steam | | |
| 4. Une fenêtre Wayland | | |
| 5. Retour au jeu | | |
| 6. Plusieurs Alt+Tab | | |
| 7. Pendant une saisie | | |
| 8. Exiled Exchange 2 | | |
| 9. Tes propres scénarios | | |
