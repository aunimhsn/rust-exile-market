# Clavier dans la fenêtre par-dessus le jeu (14b2) : les tests

## Pourquoi

La sonde a montré que le clavier peut aller à la fenêtre et revenir au jeu. Voici maintenant la vraie version, avec la ligne de la barre d'état (option B) et Ctrl+D qui fait quitter le champ. Fais les tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

## Avant

1. Ferme Exile Market et Exiled Exchange 2.
2. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR).

3. Lance le jeu en **plein écran fenêtré**, dans ton repaire, avec un flacon sur la touche `1`.

## Les tests

1. **La ligne** : ouvre l'inventaire, **Ctrl+D** sur un objet rare, clique sur **« Show N stats »**, puis clique dans le champ **min** d'une stat et tape `12`.
   ✅ Les chiffres s'écrivent, aucun flacon n'est bu, et la barre d'état dit **« Keys go to this window · Enter or Esc: back to the game »** à la place des sources.

2. **Échap** : appuie sur **Échap**, puis sur `1`.
   ✅ La barre d'état montre de nouveau les sources, le menu du jeu ne s'ouvre pas, et `1` boit le flacon.

3. **Entrée** : clique dans un champ, tape `3`, appuie sur **Entrée**, puis deux fois sur `I`.
   ✅ Le chat ne s'ouvre pas, la ligne disparaît, et l'inventaire se ferme puis se rouvre.

4. **Clic ailleurs dans la fenêtre** : clique dans un champ, puis sur une **case à cocher** de la fenêtre, puis appuie sur `1`.
   ✅ La ligne disparaît et `1` boit le flacon.

5. **Clic dans le jeu** : clique dans un champ, puis clique **dans le jeu**, puis appuie sur `I`.
   ✅ La fenêtre se ferme et `I` marche.

6. **Ctrl+D pendant la saisie** : rouvre la fenêtre, clique dans un champ, tape `6`, puis **Ctrl+D sur un autre objet**, puis appuie sur `I`.
   ✅ La fenêtre montre l'autre objet, la ligne a disparu, aucun champ n'est en cours de saisie, et `I` marche.

7. **Ctrl+D sur une case vide pendant la saisie** : clique dans un champ, puis **Ctrl+D sur une case vide**, puis appuie sur `I`.
   ✅ La fenêtre reste ouverte, la ligne disparaît, et `I` marche.

8. **Rouvrir après une saisie** : clique dans un champ, clique dans le jeu (la fenêtre se ferme), puis **Ctrl+D** sur le même objet.
   ✅ La fenêtre se rouvre **sans** la ligne, et `1` boit le flacon : elle n'a pas repris le clavier toute seule.

9. **Clic dans la fenêtre principale** : clique dans un champ de la fenêtre par-dessus le jeu, puis clique dans la **fenêtre principale** d'Exile Market (sur un autre écran, ou par-dessus le jeu si elle y est).
   ✅ La fenêtre par-dessus le jeu se ferme, et la fenêtre principale garde le clavier (par exemple, tu peux taper dans l'un de ses champs).

10. **Alt+Tab pendant la saisie** : rouvre la fenêtre, clique dans un champ, tape `7`, fais **Alt+Tab** vers une autre fenêtre, puis reviens au jeu (Alt+Tab ou un clic), puis appuie sur `1`.
    ✅ La ligne a disparu, `1` boit le flacon ; un nouveau clic dans un champ fait réapparaître la ligne, et les chiffres s'y écrivent.

11. **Un nombre trop grand, fenêtre fermée en pleine saisie** : clique dans le champ **min** d'une stat qui a une fourchette (par exemple `T2 135–154`), tape `999`, puis clique **dans le jeu** sans appuyer sur Entrée. Regarde la même stat dans la **fenêtre principale**.
    ✅ Le champ n'y montre pas `999`, mais le plus grand nombre que la stat accepte, comme quand tu quittes un champ avec Entrée.

12. **Français** : passe l'interface en français dans les réglages, puis refais le test 1.
    ✅ La ligne dit « Les touches vont dans cette fenêtre · Entrée ou Échap : retour au jeu », en entier.

13. **Tes propres scénarios** : essaie ce qui te passe par la tête (Tab entre les champs, plusieurs objets de suite, la fenêtre principale réduite…).
    ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. La ligne | | |
| 2. Échap | | |
| 3. Entrée | | |
| 4. Clic ailleurs dans la fenêtre | | |
| 5. Clic dans le jeu | | |
| 6. Ctrl+D pendant la saisie | | |
| 7. Ctrl+D sur une case vide | | |
| 8. Rouvrir après une saisie | | |
| 9. Clic dans la fenêtre principale | | |
| 10. Alt+Tab pendant la saisie | | |
| 11. Nombre trop grand, fenêtre fermée | | |
| 12. Français | | |
| 13. Tes propres scénarios | | |
