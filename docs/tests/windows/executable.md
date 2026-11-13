# L'exécutable, sous Windows : les tests (17a)

Pour le testeur Windows. L'exécutable s'appelle désormais `exile-market.exe`. Il porte l'icône de l'application, l'emblème rouge, dans l'Explorateur, la barre des tâches et Alt+Tab, et Windows le nomme « Exile Market » (Gestionnaire des tâches, propriétés du fichier). Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide).

## Avant

1. Ferme Exile Market s'il tourne.
2. Télécharge la nouvelle archive Windows `exile-market-windows-x64.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger) et décompresse-la.

## Les tests

1. **Le fichier** : ouvre le dossier décompressé dans l'Explorateur.
   ✅ Le fichier s'appelle `exile-market.exe`, avec l'emblème rouge comme icône (en affichage « Grandes icônes » aussi, net, pas flou).

2. **Ses propriétés** : clic droit sur `exile-market.exe`, Propriétés, onglet Détails.
   ✅ Description du fichier et Nom du produit : « Exile Market » ; version du produit : 0.1.0.

3. **L'icône de la fenêtre** : lance `exile-market.exe` (si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même »), puis regarde la barre des tâches et fais Alt+Tab.
   ✅ L'emblème rouge dans la barre des tâches et dans Alt+Tab ; ses réglages d'avant sont gardés (langue, ligue, touche du raccourci). L'emblème de la barre de titre de l'application reste blanc.

4. **Le Gestionnaire des tâches** : Ctrl+Maj+Échap, onglet Processus.
   ✅ L'application y apparaît sous le nom « Exile Market », avec son icône.

5. **Tes propres scénarios** : ce que tu veux essayer d'autre.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le fichier | | |
| 2. Ses propriétés | | |
| 3. L'icône de la fenêtre | | |
| 4. Le Gestionnaire des tâches | | |
| 5. Tes propres scénarios | | |
