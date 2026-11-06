# Fenêtre par-dessus le jeu : les 10 tests indispensables

Version mise à jour après trois revues de code : refais les tests 1 à 6 (le code a changé), puis les 7 à 10, nouveaux. Fais-les d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

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

13. **Copy whisper** : ouvre la fenêtre (Ctrl+D) sur un objet qui a des offres, clique sur la **loupe**, survole une offre d'un vendeur en ligne, puis clique sur **« Copy whisper »**. Clique dans le jeu, ouvre le chat (Entrée), colle (Ctrl+V), puis efface sans envoyer ; colle aussi dans un éditeur de texte (Kate).
   ✅ La pastille devient « ✓ Copied » pendant 2 secondes, et le texte collé est le message au vendeur (`@vendeur …`), dans le chat comme dans Kate : ni vide, ni l'ancien contenu du presse-papiers. L'objet affiché ne change pas.

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
