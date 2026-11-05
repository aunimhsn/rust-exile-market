# Clavier de la fenêtre par-dessus le jeu sous Windows : les tests (14b3b)

Pour le testeur Windows. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide). Renvoie ensuite le tableau et le fichier de log.

## Avant

1. Ferme Exiled Exchange 2 s'il tourne : il prend aussi Ctrl+D.
2. Télécharge la nouvelle archive Windows `exile-market-test-x86_64-pc-windows-msvc.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger), décompresse-la, puis lance `exile-market.exe`. Si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même ».
3. Lance Path of Exile 2 en **plein écran fenêtré** (pas en plein écran exclusif), **sans** le lancer en administrateur, et va dans ton repaire.
4. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log` (colle ce chemin dans la barre d'adresse de l'Explorateur).

Pour chaque test : ouvre l'inventaire, **Ctrl+D** sur un objet **rare**, puis clique sur **« Show N stats »** dans la fenêtre qui s'ouvre. Les champs **min** et **max** sont les petites cases à droite de chaque stat.

## Les tests

1. **Taper un nombre** : clique dans le champ **min** d'une stat, efface-le, tape `12`.
   ✅ `12` s'écrit dans le champ, les flacons ne se boivent pas (les touches 1 et 2 n'arrivent pas au jeu), et la barre du bas de la fenêtre dit « Keys go to this window · Enter or Esc: back to the game » (en français : « Les touches vont dans cette fenêtre · Entrée ou Échap : retour au jeu »).

2. **Entrée** : appuie sur **Entrée**, puis sur `I`.
   ✅ La barre du bas redevient normale, et `I` ferme l'inventaire tout de suite (le jeu a repris le clavier). La fenêtre reste ouverte.

3. **Échap** : rouvre l'inventaire, clique dans un champ **max**, tape `30`, appuie sur **Échap**, puis sur `I`.
   ✅ Comme au test 2 : `I` ferme l'inventaire, et le menu du jeu ne s'ouvre pas.

4. **Clic ailleurs dans la fenêtre** : rouvre l'inventaire, clique dans un champ min, tape `5`, puis clique sur une **case à cocher** de la fenêtre, puis appuie sur `I`.
   ✅ La case change, et `I` ferme l'inventaire.

5. **Clic dans le jeu** : rouvre l'inventaire, clique dans un champ min, tape `5`, puis clique **dans le jeu**, hors de la fenêtre, puis appuie sur `I`.
   ✅ La fenêtre se ferme, et `I` ferme l'inventaire.

6. **Ctrl+D pendant la saisie** : rouvre la fenêtre sur un objet rare, clique dans un champ min, tape `5`, puis fais **Ctrl+D** sur **un autre** objet de l'inventaire.
   ✅ La fenêtre montre le nouvel objet, la feuille de personnage du jeu (touche `C`) ne s'ouvre pas, et `I` ferme ensuite l'inventaire. Refais-le une dizaine de fois, Ctrl et D tapés le plus vite possible : ✅ même chose à chaque fois.

7. **Alt+Tab pendant la saisie** : clique dans un champ min, tape `5`, puis **Alt+Tab** vers une autre application (le navigateur), tape quelques lettres dedans, puis reviens au jeu (Alt+Tab ou clic).
   ✅ Les lettres vont dans l'autre application, pas dans le champ ; de retour dans le jeu, ses touches marchent (`I`).

8. **Fenêtre principale pendant la saisie** : clique dans un champ min de la fenêtre par-dessus le jeu, tape `5`, puis clique dans la **fenêtre principale** d'Exile Market (si elle est cachée par le jeu, passe ce test).
   ✅ La fenêtre par-dessus le jeu se ferme, et la fenêtre principale passe devant.

9. **Libre** : utilise la fenêtre comme un joueur (plusieurs objets, des nombres tapés, une recherche avec « Search »). Note tout ce qui te paraît bizarre.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Taper un nombre | | |
| 2. Entrée | | |
| 3. Échap | | |
| 4. Clic ailleurs dans la fenêtre | | |
| 5. Clic dans le jeu | | |
| 6. Ctrl+D pendant la saisie | | |
| 7. Alt+Tab pendant la saisie | | |
| 8. Fenêtre principale pendant la saisie | | |
| 9. Libre | | |
