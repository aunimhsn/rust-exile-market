# Fenêtre par-dessus le jeu : les tests

Fais les tests 1 à 15 sur ton installation habituelle, puis les tests 16 à 29 sur un seul écran. Fais-les d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

## Avant

1. Ferme Exile Market et Exiled Exchange 2.
2. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR).

3. Lance le jeu en **plein écran fenêtré**, dans ton repaire.

## Les tests

1. **Inventaire** : ouvre l'inventaire, **Ctrl+D** sur un objet.
   ✅ Une fenêtre s'ouvre **juste à gauche de l'inventaire** (ni par-dessus, ni avec un trou), son haut à la hauteur du curseur.

2. **Coffre** : ouvre le coffre, **Ctrl+D** sur un objet.
   ✅ La fenêtre passe **juste à droite du coffre**.

3. **Clic dans la fenêtre** : sur un objet rare, clique sur **« Show N stats »** dans la fenêtre, puis appuie deux fois sur `I`.
   ✅ Les stats se déplient, la fenêtre grandit et reste ouverte, le personnage ne bouge pas, et `I` ferme puis rouvre l'inventaire.

4. **Clic dans le jeu** : clique une fois dans le jeu, hors de la fenêtre.
   ✅ La fenêtre se ferme.

5. **Même objet** : refais **Ctrl+D** sur l'objet du test 3.
   ✅ La fenêtre se rouvre, ses stats toujours dépliées.

6. **Case vide** : ferme la fenêtre (un clic dans le jeu), puis **Ctrl+D** sur une case vide.
   ✅ Rien ne s'ouvre.

7. **Ctrl+C après la case vide** : après le test 6, attends **au moins une seconde**, puis fais **Ctrl+C** (pas D) sur l'objet du test 3, et regarde la fenêtre principale.
   ✅ Ses stats sont toujours dépliées et ses filtres inchangés : rien n'a bougé.

8. **Klipper** : ouvre l'historique du presse-papiers de KDE (l'icône du presse-papiers dans la barre des tâches).
   ✅ Le texte « Exile Market: copying an item from the game » n'y apparaît pas.

9. **Fenêtre principale réduite** : réduis la fenêtre principale d'Exile Market (son bouton –), retourne dans le jeu, **Ctrl+D** sur un objet, puis un clic dans le jeu.
   ✅ La fenêtre par-dessus le jeu s'ouvre quand même (un peu plus lentement, c'est normal), puis se ferme au clic. L'application ne plante pas.

10. **Processeur** : dans un terminal, lance `top -p $(pgrep -f target/release/exile-market)`. Dans le jeu, ouvre la fenêtre (Ctrl+D) et laisse-la ouverte une dizaine de secondes sans cliquer, puis regarde la colonne `%CPU`.
   ✅ Elle reste basse (quelques %), qu'Exile Market soit réduit ou non.

11. **Vingt Ctrl+D d'affilée** (facultatif) : enchaîne une vingtaine de Ctrl+D rapides sur des objets différents, puis sur le même.
   ✅ La fenêtre suit chaque objet, sans rester bloquée ni montrer l'objet d'avant.

12. **Ctrl+C ferme la fenêtre** : ouvre la fenêtre avec Ctrl+D sur un objet, puis fais **Ctrl+C** (pas D) sur un autre objet.
   ✅ La fenêtre par-dessus le jeu se ferme, et le nouvel objet s'affiche dans la fenêtre principale.

13. **Copy whisper** (« Copier le whisper ») : ouvre la fenêtre (Ctrl+D) sur un objet qui a des offres, clique sur la **loupe** si les offres ne sont pas déjà là, survole une offre d'un vendeur en ligne, puis clique sur la pastille. Regarde la fenêtre pendant 2 secondes. Clique ensuite dans le jeu, ouvre le chat (Entrée), colle (Ctrl+V), puis efface sans envoyer ; colle aussi dans un éditeur de texte (Kate).
   ✅ La pastille devient « ✓ Copied » (« ✓ Copié ») pendant 2 secondes, et la fenêtre par-dessus le jeu reste ouverte sur le même objet. Le texte collé est le message au vendeur (`@vendeur …`), dans le chat comme dans Kate : ni vide, ni l'ancien contenu du presse-papiers.

14. **Ctrl+D après un whisper** : après le test 13, **Ctrl+D** sur le même objet. Puis refais la copie du whisper, ferme la fenêtre (un clic dans le jeu), **Ctrl+D** sur une case vide, et colle dans le chat (efface sans envoyer).
   ✅ Sur le même objet, la fenêtre se rouvre, ses filtres conservés. Sur la case vide, rien ne s'ouvre, et le collage donne toujours le whisper.

15. **Le whisper d'une devise** : **Ctrl+D** sur une devise de l'échange (une Orbe, par exemple), cherche ses offres, copie le whisper d'une offre d'un vendeur en ligne, puis colle dans le chat (efface sans envoyer).
   ✅ Le texte collé ne contient ni `{0}` ni `{1}`, et ses montants sont ceux de l'offre.

## Sur un seul écran

Débranche ou désactive ton deuxième écran (ou fais ces tests sur une machine à un écran), jeu en **plein écran fenêtré**. Avant chaque test, bouge un peu la souris dans le jeu.

16. **La souris dans la fenêtre** : **Ctrl+D** sur un objet, puis passe la souris sur la fenêtre et sur ses offres, et clique sur une case à cocher.
   ✅ Le curseur au-dessus de la fenêtre est la flèche du bureau, pas celui du jeu ; les offres s'éclairent au survol ; la case se coche. La barre d'état dit « Les touches vont ici · Échap ou clic ailleurs : retour au jeu ». La barre des tâches reste cachée.

17. **Les touches du jeu** : la fenêtre ouverte, appuie sur `1`, puis sur **Tab**, puis sur **Espace**, puis sur **Entrée**.
   ✅ La flasque ne part pas, et rien ne bouge dans la fenêtre : aucun bouton ni case n'est encadré, aucune case ne se coche, aucune recherche ne part, aucun chiffre ne s'écrit dans un champ.

18. **Ctrl+D sur un autre objet** : la fenêtre ouverte, **Ctrl+D** sur un autre objet, survole ses offres, puis reste 5 secondes sans bouger la souris, et survole-les de nouveau. Recommence deux fois.
   ✅ La fenêtre passe sur le nouvel objet, et la souris y marche toujours, même après les 5 secondes. Dans le journal, aucune ligne « Another window became the active window » juste avant « Copy sent by the price-check hotkey ».

19. **Échap** : appuie sur **Échap**, puis sur `1`. Rouvre la fenêtre (Ctrl+D), clique dans un champ min, tape un nombre, puis appuie deux fois sur **Échap**.
   ✅ Le premier Échap ferme la fenêtre, et la flasque part ensuite. Dans le champ, le premier Échap quitte le champ (la fenêtre reste ouverte), le second la ferme.

20. **Clic dans le jeu** : **Ctrl+D** sur un objet, puis clique dans le jeu, hors de la fenêtre, puis appuie sur `1`.
   ✅ La fenêtre se ferme, et la flasque part.

21. **Clic dans la fenêtre principale** : **Ctrl+D** sur un objet, puis clique dans la fenêtre principale d'Exile Market (à côté du jeu, ou après Alt+Tab).
   ✅ La fenêtre par-dessus le jeu se ferme, et la fenêtre principale reste au premier plan : le jeu ne revient pas devant elle.

22. **Case vide** : **Ctrl+D** sur un objet, puis, la fenêtre ouverte, **Ctrl+D** sur une case vide de l'inventaire. Attends 3 secondes, puis survole les offres.
   ✅ La fenêtre reste sur le premier objet, et la souris y marche toujours après un instant. La barre d'état dit de nouveau « Les touches vont ici… ».
   Puis, toujours la fenêtre ouverte, **Ctrl+D** sur une case vide et aussitôt **Alt+Tab** vers une autre fenêtre ; attends 3 secondes.
   ✅ L'autre fenêtre reste devant : le jeu ne revient pas par-dessus.

23. **Infobulle épinglée** : **Ctrl+D** sur un objet qui a des offres, clique sur une offre pour épingler son infobulle, puis appuie sur **Échap**, puis encore sur **Échap**.
   ✅ Le premier Échap ferme l'infobulle seulement ; le second ferme la fenêtre.

24. **Alt+Tab** : **Ctrl+D** sur un objet, puis **Alt+Tab** vers une autre fenêtre, puis reviens tout de suite au jeu (Alt+Tab) et survole la fenêtre.
   ✅ Pendant l'Alt+Tab, la barre d'état montre de nouveau l'âge des sources. Revenu dans le jeu, après un instant, la souris marche dans la fenêtre, et la barre d'état dit « Les touches vont ici… ».

25. **Ctrl+D aussitôt revenu** : **Alt+Tab** vers une autre fenêtre, reviens au jeu, et fais aussitôt (moins d'une seconde) **Ctrl+D** sur un objet ; attends 2 secondes, puis survole la fenêtre.
   ✅ La souris marche dans la fenêtre, et la barre d'état dit « Les touches vont ici… ».

26. **Fenêtre principale réduite** : réduis la fenêtre principale d'Exile Market, puis **Ctrl+D** sur un objet, survole ses offres, et appuie sur **Échap**.
   ✅ La fenêtre s'ouvre, la souris y marche, Échap la ferme. L'application ne plante pas.

27. **Alt+F4** : **Ctrl+D** sur un objet, puis, la fenêtre ouverte, appuie une seule fois sur **Alt+F4**. Puis appuie sur **Échap**, et regarde si la fenêtre principale d'Exile Market est toujours là (Alt+Tab).
   ✅ Alt+F4 ne ferme rien : ni le jeu, ni Exile Market. Échap ferme la fenêtre, et un nouveau Ctrl+D la rouvre. (Un second Alt+F4, une fois la fenêtre fermée, quitterait le jeu.)

28. **L'application tuée la fenêtre ouverte** : dans un second onglet de Konsole, lance `sleep 8; pkill -x exile-market`, retourne aussitôt dans le jeu, fais **Ctrl+D** sur un objet, attends que l'application s'arrête (8 secondes), puis appuie sur `1`. Ce test regarde ce que fait le bureau quand l'application disparaît sans se fermer : son résultat dépend de KWin, pas d'Exile Market.
   ✅ Le jeu reste au premier plan (pas de barre des tâches), et la flasque part, au besoin après un clic dans le jeu.

29. **Test libre** : utilise la fenêtre comme en jeu pendant quelques minutes, sur un seul écran.
   ✅ Rien d'étrange : pas de point noir à l'écran, pas de touche perdue, pas de clic qui traverse.

## Tes résultats

| Test | Résultat | Si ça cloche |
|---|---|---|
| 1. Inventaire |  |  |
| 2. Coffre |  |  |
| 3. Clic dans la fenêtre |  |  |
| 4. Clic dans le jeu |  |  |
| 5. Même objet |  |  |
| 6. Case vide |  |  |
| 7. Ctrl+C après la case vide |  |  |
| 8. Klipper |  |  |
| 9. Fenêtre principale réduite |  |  |
| 10. Processeur |  |  |
| 11. Vingt Ctrl+D (facultatif) |  |  |
| 12. Ctrl+C ferme la fenêtre |  |  |
| 13. Copy whisper |  |  |
| 14. Ctrl+D après un whisper |  |  |
| 15. Le whisper d'une devise |  |  |
| 16. La souris dans la fenêtre |  |  |
| 17. Les touches du jeu |  |  |
| 18. Ctrl+D sur un autre objet |  |  |
| 19. Échap |  |  |
| 20. Clic dans le jeu |  |  |
| 21. Clic dans la fenêtre principale |  |  |
| 22. Case vide |  |  |
| 23. Infobulle épinglée |  |  |
| 24. Alt+Tab |  |  |
| 25. Ctrl+D aussitôt revenu |  |  |
| 26. Fenêtre principale réduite |  |  |
| 27. Alt+F4 |  |  |
| 28. L'application tuée la fenêtre ouverte |  |  |
| 29. Test libre |  |  |
