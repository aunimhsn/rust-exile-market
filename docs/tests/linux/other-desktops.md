# Un autre bureau que KDE (GNOME, gamescope…) : les tests

## Pourquoi

Le raccourci et la fenêtre par-dessus le jeu n'ont été essayés que sur KDE (KWin). Leur code repose sur des comportements que chaque bureau applique à sa façon (revue du bureau du 2026-11-10, hypothèses 1 à 9). Ce guide les essaie une par une sur un autre bureau, GNOME (Mutter) d'abord, en session **X11 ou XWayland**. Un ✅ suffit quand ça marche ; quand ça cloche, copie les lignes du journal autour du test : c'est lui qui dit dans quel cas on est.

## Avant

1. Ferme Exile Market et tout autre outil de prix.
2. Lance Exile Market (`cargo run --release` dans le dossier de la branche, ou l'exécutable de la version à tester).
3. Ouvre le journal dans un terminal, pour le suivre pendant les tests :

   ```
   tail -f ~/.local/state/exile-market/exile-market.log
   ```

4. Lance le jeu en **plein écran fenêtré**, dans ton repaire, avec un flacon sur la touche `1`. Fais les tests 1 à 7 sur **un seul écran**.

## Les tests

1. **Le bureau dans le journal** : regarde le début du journal.
   ✅ Une ligne **« Window manager: … »** nomme le bureau (par exemple `Mutter` ou `GNOME Shell`), pas `unknown`.

2. **Le jeu reste la fenêtre active pendant la saisie** (hypothèse 1) : **Ctrl+D** sur un objet rare, **« Show N stats »**, clique dans un champ **min**, tape `12`, puis **Ctrl+D sur un autre objet**.
   ✅ La fenêtre montre l'autre objet. Le journal dit **« The price-check hotkey gave the keyboard back to the game »**, puis **« Copy sent by the price-check hotkey to the window of class … »**. ❌ Si rien ne s'ouvre, ou si le journal dit « The copy can't be sent: … ».

3. **Le clavier reste dans la fenêtre** (hypothèse 2) : clique dans un champ **min**, tape `1`, attends cinq secondes, tape `2`.
   ✅ Le champ montre `12` et aucun flacon n'est bu : le bureau n'a pas repris le clavier.

4. **Le clavier revient au jeu** (hypothèse 3) : appuie sur **Échap**, puis sur `1`, puis sur `I`.
   ✅ Le flacon est bu et l'inventaire réagit, sans délai d'une demi-seconde. ❌ Si le journal dit « The keyboard can't be given back to the game: refused ».

5. **La fenêtre reçoit la souris sur un seul écran** (hypothèses 3 et 6) : bouge la souris dans le jeu deux secondes, **Ctrl+D** sur un objet, puis survole la fenêtre et clique sur **« Show N stats »**.
   ✅ Le curseur est celui du bureau au-dessus de la fenêtre, le survol et le clic marchent. Le journal dit **« The game holds the pointer: the window over the game is the active window »**. ❌ Si le curseur reste celui du jeu, ou si le journal dit « The window over the game can't become the active window: the desktop didn't activate its helper ».

6. **Rien de visible en plus** (hypothèse 4) : fenêtre ouverte, regarde sous elle, puis ouvre **Alt+Tab** et la vue d'ensemble du bureau (touche Super).
   ✅ Aucun pixel noir sous la fenêtre, aucune entrée en plus dans Alt+Tab ni dans la vue d'ensemble, aucune animation à chaque Ctrl+D.

7. **Le jeu reste au-dessus de la barre du bureau** (hypothèse 5) : fenêtre ouverte et survolée, regarde le haut et le bas de l'écran.
   ✅ La barre du bureau ne passe pas par-dessus le jeu.

8. **Deux écrans, résolution changée** (hypothèse 6) : sur deux écrans, **Ctrl+D** sur un objet et survole la fenêtre ; puis change la résolution du jeu dans ses options, et recommence.
   ✅ Le survol et les clics marchent dans les deux cas.

9. **Lancé avant le bureau** (hypothèse 7) : mets Exile Market au démarrage automatique de la session, ferme la session, rouvre-la, puis appuie sur **Ctrl+D** dans un terminal.
   ✅ Rien n'est copié dans le terminal, et le journal ne dit pas « the desktop doesn't tell its active window (_NET_SUPPORTED) ». Si c'est le cas, note-le : le raccourci écoute alors dans toutes les fenêtres pour la session, ce que `specs.md` prévoit.

10. **Le jeu reconnu** (hypothèse 8) : relance Exile Market, puis passe dans le jeu et regarde le journal. Si tu peux, recommence dans **gamescope** et dans le bureau virtuel de Wine, en relançant Exile Market à chaque fois.
    ✅ Le journal dit **« The active window, of class steam_app_2694490, is the game »**, une seule fois par lancement d'Exile Market : la première fois que le jeu est la fenêtre active. ❌ S'il dit « … isn't the game » avec la classe du jeu (par exemple `gamescope`) : note la classe et le titre.

11. **L'historique du presse-papiers** (hypothèse 9) : **Ctrl+D** sur une case vide, puis ouvre l'historique du presse-papiers du bureau (une extension sous GNOME).
    ✅ Aucun texte « Exile Market … » n'y apparaît.

12. **Cas étroits** : sur un seul écran, **Ctrl+D** sur une case vide juste après le test 4, puis **Échap** ; puis, avec un raccourci en `Ctrl+Shift` (fichier de réglages), relâche Shift pendant la copie.
    ✅ Échap ferme la fenêtre et non le menu du jeu ; Shift ne reste pas enfoncé dans le jeu.

13. **Test libre** : joue quelques minutes avec la fenêtre (plusieurs objets, Alt+Tab, capture d'écran…).
    ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu, avec le journal.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le bureau dans le journal | | |
| 2. Le jeu reste la fenêtre active | | |
| 3. Le clavier reste dans la fenêtre | | |
| 4. Le clavier revient au jeu | | |
| 5. La souris sur un seul écran | | |
| 6. Rien de visible en plus | | |
| 7. Le jeu au-dessus de la barre | | |
| 8. Deux écrans, résolution changée | | |
| 9. Lancé avant le bureau | | |
| 10. Le jeu reconnu | | |
| 11. L'historique du presse-papiers | | |
| 12. Cas étroits | | |
| 13. Test libre | | |
