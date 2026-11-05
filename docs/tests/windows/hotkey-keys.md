# Les touches du raccourci, écrites dans le fichier des réglages, sous Windows : les tests (14c3b1)

Pour le testeur Windows. Le raccourci n'est plus forcément Ctrl+D : ses touches se lisent dans le fichier des réglages (`keys = "Alt+D"`). Le point délicat, c'est la copie : quand tu tiens Shift (Ctrl+Shift+D), Exile Market le relâche le temps d'envoyer Ctrl+C, puis le presse de nouveau, pour que le jeu lise bien Ctrl+C et que le clavier reste comme tu le tiens ; quand tu tiens Alt (Alt+D), il le laisse tenu et le jeu reçoit Ctrl+Alt+C, la copie avancée, qui donne le même texte. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide). Renvoie ensuite le tableau et le fichier de log.

**Le signe d'un Alt coincé** : dans le jeu, tenir Alt sur un objet montre ses descriptions avancées (les tiers des modificateurs). Si, après une copie, l'infobulle d'un objet survolé reste avancée alors que tu ne tiens plus Alt, c'est que le jeu croit Alt encore tenu : note-le.

## Avant

**Modifie le fichier des réglages seulement quand Exile Market est fermé** : il lit les touches à son lancement, et réécrit le fichier à chaque changement dans ses réglages (une case cochée), avec les touches qu'il a lues. Un fichier modifié pendant qu'il tourne est donc ignoré, puis écrasé.

1. Ferme Exile Market et Exiled Exchange 2 s'il tourne.
2. Télécharge la nouvelle archive Windows `exile-market-test-x86_64-pc-windows-msvc.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger), décompresse-la, puis lance `exile-market.exe` une fois et ferme-le (il crée son fichier des réglages au premier changement : coche puis décoche « Always on top » dans ses réglages avant de le fermer, « Toujours au premier plan » si ton Windows est en français).
3. Ouvre le fichier des réglages dans le Bloc-notes : `%APPDATA%\exile-market\config\settings.toml` (colle ce chemin dans la barre d'adresse de l'Explorateur). Sous la ligne `[hotkey]` (ajoute-la à la fin du fichier si elle n'y est pas), écris la ligne `keys = "Alt+D"`, puis enregistre :

   ```
   [hotkey]
   on = true
   keys = "Alt+D"
   ```

4. Relance `exile-market.exe`. Si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même ».
5. Lance Path of Exile 2 en **plein écran fenêtré** (pas en plein écran exclusif), **sans** le lancer en administrateur, et va dans ton repaire.
6. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log`. Il est écrit au fur et à mesure : ouvre-le dans le Bloc-notes à la fin, ou rouvre-le pour voir les nouvelles lignes.

## Les tests

1. **Les touches lues** : regarde la fenêtre vide d'Exile Market, puis ses réglages (l'engrenage).
   ✅ La fenêtre vide invite à copier avec `Alt` + `D` ; dans les réglages, les touches `Alt` + `D` à droite de la case. Le log dit « Price-check hotkey: Alt+D ».

2. **Alt+D** : dans le jeu, **Alt+D** sur un objet de ton inventaire, en lâchant D puis Alt.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec le prix de l'objet. Le log dit « Copy requested by the price-check hotkey … (Ctrl pressed by the application) », sans « released ». Ensuite, Alt lâché, survole un objet : son infobulle est normale (pas avancée).

3. **Alt tenu sur plusieurs objets** : tiens Alt, et appuie sur D sur trois objets l'un après l'autre sans lâcher Alt. Lâche Alt à la fin.
   ✅ Chaque D ouvre la fenêtre par-dessus le jeu avec le prix de l'objet survolé, les trois fois. Après avoir lâché Alt, l'infobulle d'un objet survolé est normale.

4. **Alt lâché avant D** : appuie sur Alt, puis D, puis **lâche Alt d'abord**, puis D.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec le prix de l'objet ; ensuite, l'infobulle d'un objet survolé est normale, et une lettre tapée dans le chat du jeu (Entrée, une lettre, puis Échap) s'écrit normalement.

5. **Alt lâché très vite** : fais **Alt+D** d'un geste rapide, en lâchant les deux touches presque en même temps, dix fois de suite sur des objets différents.
   ✅ Chaque fois, le prix s'affiche ; à la fin, l'infobulle d'un objet survolé est normale. Si elle reste avancée une fois, appuie et relâche Alt : elle doit redevenir normale ; note combien de fois sur dix c'est arrivé.

6. **Ctrl+Shift+D** : ferme Exile Market, écris `keys = "Ctrl+Shift+D"` dans le fichier des réglages, relance-le, puis dans le jeu **Ctrl+Shift+D** sur un objet, puis sur trois objets en tenant Ctrl et Shift.
   ✅ Les réglages montrent `Ctrl` + `Shift` + `D`. Chaque D ouvre la fenêtre avec le prix ; le log dit « (Ctrl still held, Shift released around it) ». Ctrl et Shift lâchés, rien d'étrange dans le jeu.

7. **Une touche refusée** : ferme Exile Market, écris `keys = "Ctrl+C"`, relance-le.
   ✅ Le log dit « The hotkey "Ctrl+C" can't be used (Ctrl+C and Ctrl+Alt+C are the copies the hotkey sends): Ctrl+D instead » ; les réglages et la fenêtre vide montrent `Ctrl` + `D`, et **Ctrl+D** copie dans le jeu comme avant.

8. **Retour à Ctrl+D** : ferme Exile Market, supprime la ligne `keys`, relance-le, puis **Ctrl+D** sur un objet, puis sur trois objets en tenant Ctrl.
   ✅ Comme avant : le prix s'affiche à chaque fois, et le log dit « (Ctrl still held) » sans « released around it ».

9. **Libre** : essaie ce qui te passe par la tête (`keys = "F5"`, `keys = "alt + 1"`, Alt+D pendant qu'un champ min de la fenêtre par-dessus le jeu est en cours d'édition…).
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
| 9. Libre | | |
