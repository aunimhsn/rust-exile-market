# Fenêtre par-dessus le jeu sous Windows : les tests (14b3a)

Pour le testeur Windows. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide). Renvoie ensuite le tableau et le fichier de log.

## Avant

1. Ferme Exiled Exchange 2 s'il tourne : il prend aussi Ctrl+D.
2. Télécharge l'archive Windows `rust-exile-market-test-x86_64-pc-windows-msvc.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger), décompresse-la, puis lance `rust-exile-market.exe`. Si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même ».
3. Lance Path of Exile 2 en **plein écran fenêtré** (pas en plein écran exclusif), **sans** le lancer en administrateur, et va dans ton repaire.
4. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log` (colle ce chemin dans la barre d'adresse de l'Explorateur).

## Les tests

1. **Inventaire** : ouvre l'inventaire, **Ctrl+D** sur un objet.
   ✅ Une fenêtre s'ouvre **juste à gauche de l'inventaire** (ni par-dessus, ni avec un trou), son haut à la hauteur du curseur, et le jeu garde la main : `I` ferme l'inventaire.

2. **Coffre** : ouvre le coffre, **Ctrl+D** sur un objet.
   ✅ La fenêtre passe **juste à droite du coffre**.

3. **Ctrl relâché vite** : fais une dizaine de Ctrl+D les plus brefs possible (Ctrl et D tapés presque ensemble), chacun sur un autre objet.
   ✅ La fenêtre s'ouvre à chaque fois sur le nouvel objet, et la feuille de personnage du jeu (touche `C`) ne s'ouvre jamais.

4. **Clic dans la fenêtre** : sur un objet rare, clique sur **« Show N stats »** dans la fenêtre, puis appuie deux fois sur `I`.
   ✅ Les stats se déplient, la fenêtre grandit et reste ouverte, le personnage ne bouge pas, et `I` ferme puis rouvre l'inventaire tout de suite (le jeu a repris le clavier).

5. **Case à cocher** : dans la fenêtre, décoche puis recoche une stat.
   ✅ La case change à chaque clic (aucun clic perdu).

6. **Clic dans le jeu** : clique une fois dans le jeu, hors de la fenêtre.
   ✅ La fenêtre se ferme.

7. **Même objet** : refais **Ctrl+D** sur l'objet du test 4.
   ✅ La fenêtre se rouvre, ses stats toujours dépliées.

8. **Case vide** : ferme la fenêtre (un clic dans le jeu), puis **Ctrl+D** sur une case vide.
   ✅ Rien ne s'ouvre.

9. **Historique du presse-papiers** : appuie sur **Windows+V**.
   ✅ Le texte « Exile Market: copying an item from the game » n'y apparaît pas (si l'historique est désactivé, note-le simplement).

10. **Fenêtre principale réduite** : réduis la fenêtre principale d'Exile Market (son bouton –), retourne dans le jeu, **Ctrl+D** sur un objet, puis un clic dans le jeu.
    ✅ La fenêtre par-dessus le jeu s'ouvre quand même, puis se ferme au clic. L'application ne plante pas.

11. **Barre des tâches** : pendant que la fenêtre par-dessus le jeu est ouverte, regarde la barre des tâches (Alt+Tab si le jeu la cache). Ferme-la d'un clic dans le jeu, rouvre-la avec Ctrl+D sur un autre objet, et regarde encore.
    ✅ Une seule entrée Exile Market, pas deux, les deux fois.

12. **Ctrl+C ferme la fenêtre** : ouvre la fenêtre avec Ctrl+D sur un objet, puis fais **Ctrl+C** (pas D) sur un autre objet.
    ✅ La fenêtre par-dessus le jeu se ferme, et le nouvel objet s'affiche dans la fenêtre principale.

13. **Voir sur l'échange** : ouvre la fenêtre (Ctrl+D), puis clique sur la pastille **« Check on trade »** en haut à droite.
    ✅ Le navigateur s'ouvre **devant** le jeu, sur la recherche de l'objet (pas derrière, avec juste un clignotement dans la barre des tâches). Clique ensuite dans le jeu : il repasse devant le navigateur, et ses touches marchent (`I`).

14. **Alt+Tab** : la fenêtre par-dessus le jeu ouverte, maintiens Alt et appuie sur Tab.
    ✅ Note combien d'entrées « Exile Market » tu vois (une ou deux), puis relâche sur le jeu : la fenêtre par-dessus le jeu est toujours là.

15. **Le jeu reste dessous** : fenêtre ouverte, clique dans la fenêtre (une case à cocher), puis attends 5 secondes.
    ✅ La fenêtre reste visible par-dessus le jeu, elle ne passe pas derrière.

16. **Deux écrans** (seulement si tu as deux écrans à des échelles différentes, par exemple 100 % et 150 % dans Paramètres > Affichage) : mets la fenêtre principale d'Exile Market sur l'autre écran que le jeu, puis Ctrl+D sur un objet dans le jeu, et clique dans la fenêtre qui s'ouvre.
    ✅ La fenêtre est bien à côté de l'inventaire, à la bonne taille, et un clic dedans ne la ferme pas. Note aussi si, à son ouverture, elle saute d'une place à une autre ou chevauche l'inventaire un instant.

17. **Jeu en administrateur** (facultatif) : relance le jeu en administrateur (clic droit, « Exécuter en tant qu'administrateur »), puis Ctrl+D sur un objet.
    ✅ Note simplement ce qui se passe (rien, ou la fenêtre s'ouvre) ; puis relance le jeu normalement.

18. **Gestionnaire de presse-papiers** (facultatif, seulement si tu en utilises un, comme Ditto) : après quelques Ctrl+D, ouvre son historique.
    ✅ Note si le texte « Exile Market: copying an item from the game » y apparaît.

19. **Processeur** : ouvre le Gestionnaire des tâches (Ctrl+Maj+Échap). Dans le jeu, ouvre la fenêtre (Ctrl+D) et laisse-la ouverte une dizaine de secondes sans cliquer, puis regarde la colonne Processeur d'Exile Market.
    ✅ Elle reste basse (quelques %).

20. **Test libre** : joue quelques minutes en faisant des Ctrl+D comme d'habitude (plusieurs objets d'affilée, l'inventaire puis le coffre).
    ✅ Rien d'étrange : la fenêtre suit chaque objet, le jeu garde le clavier.

À savoir : dans cette version, taper un nombre dans un champ min ou max de la fenêtre l'envoie au jeu (ses flacons). C'est attendu, la prochaine version corrige ça : ne le compte pas comme un échec.

## Tes résultats

| Test | Résultat | Si ça cloche |
|---|---|---|
| 1. Inventaire |  |  |
| 2. Coffre |  |  |
| 3. Ctrl relâché vite |  |  |
| 4. Clic dans la fenêtre |  |  |
| 5. Case à cocher |  |  |
| 6. Clic dans le jeu |  |  |
| 7. Même objet |  |  |
| 8. Case vide |  |  |
| 9. Historique du presse-papiers |  |  |
| 10. Fenêtre principale réduite |  |  |
| 11. Barre des tâches |  |  |
| 12. Ctrl+C ferme la fenêtre |  |  |
| 13. Voir sur l'échange |  |  |
| 14. Alt+Tab (combien d'entrées ?) |  |  |
| 15. Le jeu reste dessous |  |  |
| 16. Deux écrans (si possible) |  |  |
| 17. Jeu en administrateur (facultatif) |  |  |
| 18. Gestionnaire de presse-papiers (facultatif) |  |  |
| 19. Processeur |  |  |
| 20. Test libre |  |  |

Joins le fichier `exile-market.log` à ta réponse.
