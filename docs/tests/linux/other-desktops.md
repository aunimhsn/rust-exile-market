# Un autre bureau que KDE (GNOME, gamescope…) : les tests

## Pourquoi

Le raccourci et la fenêtre par-dessus le jeu n'ont été essayés que sur KDE (KWin). Leur code repose sur des comportements que chaque bureau applique à sa façon (revue du bureau du 2026-11-10, hypothèses 1 à 9). Ce guide essaie les quatre qui comptent le plus (le clavier, puis la souris sur un seul écran) sur un autre bureau, GNOME (Mutter) d'abord, en session **X11 ou XWayland** ; les autres sont dans `backlog.md`, à essayer si un joueur signale un souci. Un ✅ suffit quand ça marche ; quand ça cloche, copie les lignes du journal autour du test : c'est lui qui dit dans quel cas on est.

## Avant

1. Ferme Exile Market et tout autre outil de prix.
2. Lance Exile Market (`cargo run --release` dans le dossier de la branche, ou l'exécutable de la version à tester).
3. Ouvre le journal dans un terminal, pour le suivre pendant les tests :

   ```
   tail -f ~/.local/state/exile-market/exile-market.log
   ```

4. Lance le jeu en **plein écran fenêtré**, dans ton repaire, avec un flacon sur la touche `1`. Fais les tests sur **un seul écran**.

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

6. **Test libre** : joue quelques minutes avec la fenêtre (plusieurs objets, Alt+Tab, capture d'écran…).
   ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu, avec le journal.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le bureau dans le journal | | |
| 2. Le jeu reste la fenêtre active | | |
| 3. Le clavier reste dans la fenêtre | | |
| 4. Le clavier revient au jeu | | |
| 5. La souris sur un seul écran | | |
| 6. Test libre | | |
