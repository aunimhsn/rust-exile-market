# Le raccourci activé ou coupé, et sa touche prise (14c3a) : les tests

## Pourquoi

Les réglages ont maintenant un groupe **Hotkey** : la case « Price check in the game » active ou coupe Ctrl+D, et quand une autre application a déjà pris Ctrl+D (Exiled Exchange 2), Exile Market le dit dans la fenêtre vide et dans les réglages au lieu de l'écrire seulement dans le journal. Fais les tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

## Avant

1. Ferme Exile Market, Exiled Exchange 2 et le jeu.
2. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR).

3. Puis lance le jeu par Steam, en **plein écran fenêtré**, et va dans ton repaire.

## Les tests

1. **Les réglages** : dans la fenêtre principale, ouvre les réglages (l'engrenage).
   ✅ Sous « Always on top », le label **HOTKEY**, la case **Price check in the game** cochée, les touches `Ctrl` + `D` à droite, et la ligne pâle « Copies the item under the cursor and shows its price over the game. ».

2. **Couper le raccourci** : décoche la case, reviens au jeu, puis **Ctrl+D** sur un objet.
   ✅ Les touches pâlissent dans les réglages ; dans le jeu, rien ne s'ouvre (Ctrl+D va au jeu), et le journal dit « Price-check hotkey turned off ». **Ctrl+C** sur l'objet le montre toujours dans la fenêtre principale.

3. **La fenêtre vide, raccourci coupé** : ferme et relance Exile Market (la case reste décochée), sans copier d'objet.
   ✅ La fenêtre vide invite à copier avec `Ctrl` + `C`.

4. **Rallumer le raccourci** : coche la case, puis reviens au jeu et fais **Ctrl+D** sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec son prix ; le journal dit « Price-check hotkey turned on », puis « … listened to while the game is the active window ».

5. **La touche prise** (si tu as Exiled Exchange 2, comme les tests 6 et 7) : ferme Exile Market et le jeu, lance **Exiled Exchange 2**, puis Exile Market, puis le jeu. Une fois en jeu, **n'appuie pas sur Ctrl+D** (Exiled Exchange 2 copierait l'objet, et Exile Market l'afficherait à la place de sa fenêtre vide) : fais Alt+Tab vers la fenêtre principale d'Exile Market, regarde sa fenêtre vide, puis reviens au jeu ; fais cinq allers-retours.
   ✅ À chaque retour au jeu, Exile Market et Exiled Exchange 2 essaient tous deux de prendre Ctrl+D, et le premier la garde : les lignes peuvent donc apparaître à un passage et disparaître au suivant. Quand elles apparaissent, la fenêtre vide invite à copier avec `Ctrl` + `C`, et dessous, en jaune, « **Ctrl+D** is taken by another application. » puis, à la ligne, « Choose another key in the settings. » ; ouvre alors les réglages : la ligne sous le raccourci est en jaune, « **Ctrl+D** is taken by another application. Choose another key, or copy with **Ctrl+C**. » (referme-les ensuite). Quand elles n'apparaissent pas, la fenêtre vide invite à `Ctrl` + `D`. Note combien de fois sur cinq tu as vu les lignes jaunes, et, si tu les as vues, recopie la fin de la ligne « The price-check hotkey Ctrl+D can't be listened to: … » du journal dans la remarque.

6. **La touche libérée** : quitte Exiled Exchange 2, reviens au jeu, puis **Ctrl+D** sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre du premier coup ; dans les réglages de la fenêtre principale, la ligne sous le raccourci est pâle (« Copies the item… »).

7. **Couper le raccourci quand la touche est prise** : ferme et relance Exile Market (pour retrouver sa fenêtre vide), puis relance Exiled Exchange 2 et fais des allers-retours entre le jeu et la fenêtre principale jusqu'à voir les lignes jaunes dans sa fenêtre vide. Ouvre les réglages, vérifie que la ligne jaune y est, puis décoche la case. Referme les réglages (l'engrenage).
   ✅ Dans les réglages, la ligne jaune redevient la ligne pâle dès que la case est décochée ; une fois les réglages fermés, la fenêtre vide invite à `Ctrl` + `C`, sans ligne jaune. Pour finir, quitte Exiled Exchange 2 et recoche la case.

8. **En français** : passe la langue sur Français et regarde les réglages.
   ✅ « RACCOURCI », « Estimer le prix en jeu », « Copie l'objet sous le curseur et affiche son prix par-dessus le jeu. » ; rien de coupé ni de mal aligné.

9. **Tes propres scénarios** : essaie ce qui te passe par la tête (cocher et décocher vite plusieurs fois, la case changée pendant que la fenêtre par-dessus le jeu est ouverte…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Les réglages | | |
| 2. Couper le raccourci | | |
| 3. La fenêtre vide, raccourci coupé | | |
| 4. Rallumer le raccourci | | |
| 5. La touche prise | | |
| 6. La touche libérée | | |
| 7. Couper le raccourci quand la touche est prise | | |
| 8. En français | | |
| 9. Tes propres scénarios | | |
