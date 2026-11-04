# Rapport de bug, sous Windows : les tests (16b)

Pour le testeur Windows. Un bug à gauche de l'engrenage ouvre l'aperçu du rapport de bug à la place du contenu de la fenêtre : le résumé (version, système, langues, ligue, réglages), les dernières lignes du journal de cette session ou de la précédente (deux pastilles jointes, la blanche glisse vers l'autre au clic ; la précédente choisie d'office après un plantage), et une case pour joindre l'objet affiché. **Copier le journal** copie tout le journal choisi. **Ouvrir sur GitHub** ouvre le formulaire des issues du dépôt dans le navigateur, ses champs remplis par l'adresse ; rien n'est envoyé par l'application. Fais les tests d'affilée. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche (une capture d'écran aide). Renvoie ensuite le tableau et le fichier de log.

**N'envoie aucune issue** : ouvre le formulaire, regarde ses champs, puis ferme l'onglet.

## Avant

1. Ferme Exile Market et Exiled Exchange 2 s'il tourne.
2. Télécharge la nouvelle archive Windows `rust-exile-market-test-x86_64-pc-windows-msvc.zip` (lien donné par le propriétaire ; il faut être connecté à un compte GitHub pour la télécharger) et décompresse-la.
3. Lance `rust-exile-market.exe`. Si Windows SmartScreen bloque : « Informations complémentaires », puis « Exécuter quand même ».
4. Connecte-toi à GitHub dans ton navigateur par défaut, pour voir le formulaire comme un joueur connecté.
5. Le log est dans `%LOCALAPPDATA%\exile-market\data\exile-market.log` (colle ce chemin dans la barre d'adresse de l'Explorateur), celui de la session précédente à côté, `exile-market.previous.log`. Il est écrit au fur et à mesure : ouvre-le dans le Bloc-notes à la fin, ou rouvre-le pour voir les nouvelles lignes.

## Les tests

1. **L'aperçu** : sans rien copier, survole le bug, à gauche de l'engrenage, puis clique dessus.
   ✅ L'infobulle dit « Report a bug » (ou « Signaler un bug ») ; l'aperçu s'ouvre à la place du contenu, le bug reste allumé ; pas de case pour joindre un objet.

2. **Le résumé** : regarde la boîte « Sent with the report ».
   ✅ Cinq lignes : `Exile Market 0.1.0`, `Windows 11 23H2 (build 22631)` (ta version de Windows), les langues, la ligue et son mode, puis `Always on top: …` et `Hotkey: on, Ctrl+D` (tes touches). Tout est en anglais.

3. **Le journal** : regarde la boîte « Log ».
   ✅ Les dernières lignes, déroulée jusqu'en bas ; chaque ligne commence par l'heure sans la date (`18:48:03 INFO …`) ; la boîte défile à l'intérieur sans agrandir la fenêtre ; aucun chemin ne montre ton nom de compte Windows (`C:\Users\<toi>` est écrit `~`) ; une fenêtre autre que le jeu est écrite *another window*, en gris et en italique.

4. **Le choix du journal** : regarde à droite du libellé « Log ». S'il n'y a pas de pastilles (le premier lancement de l'application sur ce PC), ferme Exile Market par sa croix, relance-le et rouvre l'aperçu. Clique sur **The previous one**, puis sur **This session** ; puis vite sur l'une puis l'autre, avant la fin du glissement.
   ✅ Deux pastilles jointes sur fond sombre, celle choisie en blanc. Au clic, la pastille blanche glisse vers l'autre, calmement (environ un quart de seconde, sans rebond), sa largeur suit le mot ; la boîte montre alors les lignes de la session précédente, puis de nouveau celles de cette session. Recliquée en plein glissement, la pastille repart de là où elle est, sans saut.

5. **Après un plantage** : ouvre PowerShell et tue Exile Market sans le laisser se fermer : `taskkill /F /IM rust-exile-market.exe`. Relance-le, puis ouvre l'aperçu.
   ✅ « The previous one » est déjà en blanc à l'ouverture, sans glisser ; la boîte montre la session tuée, qui s'arrête sur sa dernière ligne, **sans** « Exile Market closed ». Puis ferme Exile Market normalement (la croix), relance-le et rouvre l'aperçu : ✅ « This session » est choisie.

6. **La première session** : ferme Exile Market, supprime les deux fichiers du log (`exile-market.log` et `exile-market.previous.log`, dans `%LOCALAPPDATA%\exile-market\data\`), relance-le et ouvre l'aperçu.
   ✅ Pas de pastilles : « Log » seul au-dessus de la boîte.

7. **Copier le journal** : clique sur **Copy the log**, puis colle (Ctrl+V) dans le Bloc-notes. Choisis ensuite l'autre journal (après le test 6, ferme et relance d'abord Exile Market pour en avoir deux) ; puis ferme l'aperçu et rouvre-le.
   ✅ La pastille devient « ✓ Copied » (la coche avant le mot, en gris), à droite de « Open on GitHub ». Le texte collé est tout le journal de la session choisie, écrit comme dans la boîte, de sa première ligne (`… INFO Log file: ~\…`) à sa dernière ; la fenêtre ne réagit pas à ce texte. L'autre journal choisi, la pastille redevient « Copy the log », et « ✓ Copied » revient avec le journal copié ; rouvert, l'aperçu montre « Copy the log ».

8. **Depuis un objet** : lance Path of Exile 2 en plein écran fenêtré, copie un objet unique (Ctrl+D ou Ctrl+C), puis clique sur le bug dans la fenêtre principale.
   ✅ La case « Join the item shown: **son nom** » est cochée ; le résumé dit aussi « Items: … ».

9. **Ouvrir sur GitHub** : clique sur **Open on GitHub**. Puis choisis l'autre journal et rouvre sur GitHub.
   ✅ Le navigateur ouvre le formulaire « Bug report » du dépôt, sans erreur ; « Version and settings » contient le résumé, « Item » le texte de l'objet, « Log » les mêmes lignes que la boîte de l'aperçu ; l'aperçu reste ouvert ; le log dit « Bug report opened on GitHub: N characters, the item joined, the log of this session », avec N au plus 6000. La seconde fois, « Log » a les lignes de l'autre session seulement.

10. **Un objet copié ferme l'aperçu** : aperçu ouvert, copie un autre objet dans le jeu.
    ✅ L'aperçu se ferme et la fenêtre montre le nouvel objet.

11. **En français** : passe la langue en français dans les réglages, puis rouvre l'aperçu.
    ✅ « Signaler un bug », « Cette session » et « La précédente » tiennent sans être coupés ; le résumé et le journal restent en anglais.

12. **Tes propres scénarios** : essaie ce qui te passe par la tête (une petite fenêtre, Échap, l'engrenage pendant l'aperçu…).
    ✅ Rien d'étrange ; sinon, décris ce que tu as fait et ce que tu as vu.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. L'aperçu | | |
| 2. Le résumé | | |
| 3. Le journal | | |
| 4. Le choix du journal | | |
| 5. Après un plantage | | |
| 6. La première session | | |
| 7. Copier le journal | | |
| 8. Depuis un objet | | |
| 9. Ouvrir sur GitHub | | |
| 10. Un objet copié ferme l'aperçu | | |
| 11. En français | | |
| 12. Tes propres scénarios | | |
