# Les touches du raccourci, écrites dans le fichier des réglages (14c3b1) : les tests

## Pourquoi

Le raccourci n'est plus forcément Ctrl+D : ses touches se lisent dans le fichier des réglages (`keys = "Alt+D"`), en attendant de les choisir dans les réglages (14c3b2). Le point délicat, c'est la copie : quand tu tiens Alt (Alt+D) ou Shift (Ctrl+Shift+D), Exile Market doit les relâcher le temps d'envoyer Ctrl+C, puis les presser de nouveau, pour que le jeu lise bien Ctrl+C et que le clavier reste comme tu le tiens. Ces tests vérifient qu'aucune touche ne reste coincée pour le jeu. Fais-les d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

**Le signe d'un Alt coincé** : dans le jeu, tenir Alt sur un objet montre ses descriptions avancées (les tiers des modificateurs). Si, après une copie, l'infobulle d'un objet survolé reste avancée alors que tu ne tiens plus Alt, c'est que le jeu croit Alt encore tenu : note-le.

## Avant

1. Ferme Exile Market, Exiled Exchange 2 et le jeu.
2. Ouvre le fichier des réglages dans un éditeur : `~/.config/exile-market/settings.toml`. Sous la ligne `[hotkey]` (ajoute-la à la fin du fichier si elle n'y est pas), écris la ligne `keys = "Alt+D"`, puis enregistre :

   ```
   [hotkey]
   on = true
   keys = "Alt+D"
   ```

3. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le worktree donné avec la PR).

4. Puis lance le jeu par Steam, en **plein écran fenêtré**, et va dans ton repaire.

## Les tests

1. **Les touches lues** : regarde la fenêtre vide d'Exile Market, puis ses réglages (l'engrenage).
   ✅ La fenêtre vide invite à copier avec `Alt` + `D` ; dans les réglages, les touches `Alt` + `D` à droite de la case. Le journal dit « Price-check hotkey: Alt+D ».

2. **Alt+D** : dans le jeu, **Alt+D** sur un objet de ton inventaire, en lâchant D puis Alt.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec le prix de l'objet. Le journal dit « Copy sent by the price-check hotkey … (Ctrl pressed by the application, Alt released around it) ». Ensuite, Alt lâché, survole un objet : son infobulle est normale (pas avancée).

3. **Alt tenu sur plusieurs objets** : tiens Alt, et appuie sur D sur trois objets l'un après l'autre sans lâcher Alt. Lâche Alt à la fin.
   ✅ Chaque D ouvre la fenêtre par-dessus le jeu avec le prix de l'objet survolé, les trois fois. Après avoir lâché Alt, l'infobulle d'un objet survolé est normale.

4. **Alt lâché avant D** : appuie sur Alt, puis D, puis **lâche Alt d'abord**, puis D.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec le prix de l'objet. Ensuite, l'infobulle d'un objet survolé est normale, et une lettre tapée dans le chat du jeu (Entrée, une lettre, puis Échap) s'écrit normalement.

5. **Alt lâché très vite** : fais **Alt+D** d'un geste rapide, en lâchant les deux touches presque en même temps, dix fois de suite sur des objets différents.
   ✅ Chaque fois, le prix s'affiche ; à la fin, l'infobulle d'un objet survolé est normale. Si l'infobulle reste avancée une fois, appuie et relâche Alt : elle doit redevenir normale ; note combien de fois sur dix c'est arrivé.

6. **Ctrl+Shift+D** : ferme Exile Market, écris `keys = "Ctrl+Shift+D"` dans le fichier des réglages, relance-le, puis dans le jeu **Ctrl+Shift+D** sur un objet, puis sur trois objets en tenant Ctrl et Shift.
   ✅ Les réglages montrent `Ctrl` + `Shift` + `D`. Chaque D ouvre la fenêtre avec le prix ; le journal dit « (Ctrl still held, Shift released around it) ». Ctrl et Shift lâchés, rien d'étrange dans le jeu (pas de déplacement, ni de clic qui agit comme avec Shift tenu).

7. **Une touche refusée** : ferme Exile Market, écris `keys = "Ctrl+C"`, relance-le.
   ✅ Le journal dit « The hotkey "Ctrl+C" can't be used (Ctrl+C is the copy the hotkey sends): Ctrl+D instead » ; les réglages et la fenêtre vide montrent `Ctrl` + `D`, et **Ctrl+D** copie dans le jeu comme avant.

8. **Retour à Ctrl+D** : ferme Exile Market, supprime la ligne `keys`, relance-le, puis **Ctrl+D** sur un objet, puis sur trois objets en tenant Ctrl.
   ✅ Comme avant : le prix s'affiche à chaque fois, et le journal dit « (Ctrl still held) » sans « released around it ».

9. **Tes propres scénarios** : essaie ce qui te passe par la tête (`keys = "F5"`, `keys = "alt + 1"`, Alt+D pendant qu'un champ min de la fenêtre par-dessus le jeu est en cours d'édition…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Les touches lues | | |
| 2. Alt+D | | |
| 3. Alt tenu sur plusieurs objets | | |
| 4. Alt lâché avant D | | |
| 5. Alt lâché très vite | | |
| 6. Ctrl+Shift+D | | |
| 7. Une touche refusée | | |
| 8. Retour à Ctrl+D | | |
| 9. Tes propres scénarios | | |
