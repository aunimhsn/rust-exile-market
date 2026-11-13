# Ctrl+D dans le jeu seulement sous Windows : les tests (14c2)

Pour le testeur Windows. Jusqu'ici, Exile Market prenait Ctrl+D dans toutes les fenêtres : dans un navigateur, Ctrl+D devenait Ctrl+C au lieu d'ajouter un favori. Maintenant, la touche n'est prise que pendant que le jeu est au premier plan (ou la fenêtre par-dessus le jeu, après un clic dedans) ; ailleurs, Ctrl+D reste à la fenêtre. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide). Renvoie ensuite le tableau et le fichier de log.

## Avant

1. Ferme Exiled Exchange 2 s'il tourne : il prend aussi Ctrl+D (le test 10 le relance exprès).
2. Télécharge la nouvelle archive Windows `exile-market-windows-x64.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger), décompresse-la, puis lance `exile-market.exe`. Si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même ».
3. Lance Path of Exile 2 en **plein écran fenêtré** (pas en plein écran exclusif), **sans** le lancer en administrateur, et va dans ton repaire.
4. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log` (colle ce chemin dans la barre d'adresse de l'Explorateur). Il est écrit au fur et à mesure : ouvre-le dans le Bloc-notes à la fin, ou rouvre-le pour voir les nouvelles lignes.

## Les tests

1. **Jeu lancé après l'application** : dès que tu es en jeu, sans Alt+Tab, ouvre l'inventaire et fais **Ctrl+D** sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre avec son prix, et le log dit une fois **« The foreground window, of class …, is the game »** (recopie la classe dans la remarque) et **« Price-check hotkey Ctrl+D listened to while the game is the active window »**.

2. **La fenêtre principale** : fais Alt+Tab vers la fenêtre principale d'Exile Market, **clique dedans**, puis fais **Ctrl+D**.
   ✅ Rien ne se passe, et le log n'écrit **aucune** nouvelle ligne « Copy requested by the price-check hotkey … ».

3. **Un navigateur** : fais Alt+Tab vers un navigateur (Chrome, Edge, Firefox) ouvert sur une page, puis **Ctrl+D**.
   ✅ Le navigateur propose d'ajouter la page aux favoris (annule-le), le log écrit une fois « The foreground window, of class …, isn't the game » pour le navigateur, mais aucune ligne « Copy requested … ». D'autres lignes de ce genre peuvent apparaître, une fois chacune (le sélecteur d'Alt+Tab, la barre des tâches) : c'est normal.

4. **Retour au jeu** : reviens au jeu (Alt+Tab ou un clic), puis **aussitôt** Ctrl+D sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre du premier coup.

5. **Plusieurs Alt+Tab** : fais cinq allers-retours entre le jeu et le navigateur, puis **Ctrl+D** dans le jeu.
   ✅ La copie marche, et le log n'a pas écrit une ligne par Alt+Tab (ni « listened to », ni « can't be listened to », ni la classe d'une fenêtre déjà écrite).

6. **Pendant une saisie** : **Ctrl+D** sur un objet **rare**, clique sur « Show N stats », clique dans le champ **min** d'une stat, tape `5`, puis **Ctrl+D sur un autre objet**.
   ✅ La fenêtre montre l'autre objet : la touche reste prise pendant que la fenêtre par-dessus le jeu a le clavier.

7. **Après un clic dans la fenêtre** : clique sur une **case à cocher** de la fenêtre par-dessus le jeu, puis aussitôt **Ctrl+D** sur un autre objet.
   ✅ La fenêtre montre l'autre objet.

8. **Fenêtre principale réduite** : réduis la fenêtre principale d'Exile Market (le bouton « – » de sa barre de titre), reviens au jeu, fais Alt+Tab vers le navigateur puis retour au jeu, puis **Ctrl+D** sur un objet.
   ✅ La fenêtre par-dessus le jeu s'ouvre du premier coup, comme au test 4.

9. **Le dossier du jeu** : ouvre le dossier d'installation du jeu dans l'Explorateur (sa fenêtre s'appelle « Path of Exile 2 » ; par Steam : clic droit sur le jeu › Gérer › Parcourir les fichiers locaux), **ne sélectionne aucun fichier**, puis fais **Ctrl+D**.
   ✅ Rien ne se passe, le log écrit une fois « The foreground window, of class CabinetWClass, isn't the game », et aucune ligne « Copy requested … ». Si la fenêtre ne s'appelle pas exactement « Path of Exile 2 », note son titre.

10. **Exiled Exchange 2** (seulement si tu l'as) : lance Exiled Exchange 2, reviens au jeu, puis **Ctrl+D** sur un objet. Quitte ensuite Exiled Exchange 2, fais Alt+Tab hors du jeu et retour, puis **Ctrl+D**.
   ✅ Note ce qui s'ouvre la première fois (Exile Market, Exiled Exchange 2, ou les deux) et ce que dit le log ; la seconde fois, Exile Market s'ouvre.

11. **Libre** : essaie ce qui te passe par la tête (le jeu fermé puis relancé, Ctrl+D tenu pendant un Alt+Tab, le bureau Windows au premier plan…). Note tout ce qui te paraît bizarre.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Jeu lancé après l'application | | |
| 2. La fenêtre principale | | |
| 3. Un navigateur | | |
| 4. Retour au jeu | | |
| 5. Plusieurs Alt+Tab | | |
| 6. Pendant une saisie | | |
| 7. Après un clic dans la fenêtre | | |
| 8. Fenêtre principale réduite | | |
| 9. Le dossier du jeu | | |
| 10. Exiled Exchange 2 | | |
| 11. Libre | | |
