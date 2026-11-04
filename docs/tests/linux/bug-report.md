# Rapport de bug (16b1) : les tests

## Pourquoi

Un bug à gauche de l'engrenage ouvre l'aperçu du rapport de bug à la place du contenu de la fenêtre : le résumé (version, système, langues, ligue, réglages), les dernières lignes du journal, et une case pour joindre l'objet affiché. **Ouvrir sur GitHub** ouvre le formulaire des issues du dépôt dans le navigateur, ses champs remplis par l'adresse ; rien n'est envoyé par l'application. Fais les tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

**N'envoie aucune issue** : ouvre le formulaire, regarde ses champs, puis ferme l'onglet.

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

## Avant

1. Ferme Exile Market, Exiled Exchange 2 et le jeu.
2. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR).

3. Connecte-toi à GitHub dans ton navigateur, pour voir le formulaire comme un joueur connecté.

## Les tests

1. **Avant le premier objet** : sans rien copier, survole le bug, à gauche de l'engrenage, puis clique dessus.
   ✅ L'infobulle dit « Signaler un bug » (ou « Report a bug ») ; l'aperçu s'ouvre, le bug reste allumé ; il n'y a **pas** de case pour joindre un objet ; le résumé dit « Interface: … » sans « Items: ».

2. **Le résumé** : regarde la boîte « Envoyé avec le rapport ».
   ✅ Cinq lignes : `Exile Market 0.1.0`, `Linux · KDE Plasma · Wayland` (ou `X11`), les langues, la ligue et son mode (`softcore` ou `hardcore`), puis `Always on top: …` et `Hotkey: on, Ctrl+D` (tes touches). Tout est en anglais, même si l'interface est en français.

3. **Le journal** : regarde la boîte « Journal ».
   ✅ Elle montre les dernières lignes, déroulée jusqu'en bas ; chaque ligne commence par l'heure sans la date (`18:48:03 INFO …`) ; elle défile à l'intérieur sans agrandir la fenêtre ; une fenêtre autre que le jeu est écrite *another window*, en gris et en italique (fais un Alt+Tab vers une autre application avant d'ouvrir l'aperçu si tu n'en vois pas).

4. **Fermer** : clique de nouveau sur le bug ; rouvre-le, puis appuie sur **Échap** ; rouvre-le, puis clique sur l'**engrenage**.
   ✅ Le bug ferme l'aperçu ; Échap aussi ; l'engrenage ouvre les réglages à sa place (l'engrenage allumé, le bug éteint), et le bug depuis les réglages rouvre l'aperçu.

5. **Depuis un objet** : lance le jeu, copie un objet unique (Ctrl+D ou Ctrl+C), puis clique sur le bug dans la fenêtre principale.
   ✅ La case « Joindre l'objet affiché : **son nom** » est cochée, le nom en gras après les deux-points ; le résumé dit aussi « Items: … » (la langue du jeu).

   Puis ferme l'aperçu, copie un objet **magique** au nom long (un flacon avec ses deux affixes), et rouvre l'aperçu.
   ✅ Si le nom ne tient pas, il est coupé par « … », jamais le début de la phrase ; au survol, une infobulle montre la phrase entière.

6. **Ouvrir sur GitHub** : clique sur **Ouvrir sur GitHub**.
   ✅ Le navigateur ouvre le formulaire « Bug report » du dépôt, sans erreur ; « Version and settings » contient le résumé, « Item » le texte de l'objet tel que copié, « Log » les mêmes lignes que la boîte de l'aperçu (la première et la dernière identiques), les espaces, les deux-points et les parenthèses intacts ; l'aperçu reste ouvert ; le journal dit « Bug report opened on GitHub: N characters, the item joined », avec N au plus 6000.

7. **Sans l'objet** : décoche la case, puis rouvre sur GitHub. Si ta session est courte, copie d'abord une vingtaine d'objets, pour que le journal ne tienne plus en entier.
   ✅ La boîte du journal montre plus de lignes qu'avant (sa première ligne commence plus tôt) ; le champ « Item » du formulaire est vide ; « Log » a les lignes de l'aperçu.

8. **Un objet copié ferme l'aperçu** : aperçu ouvert, copie un autre objet dans le jeu.
   ✅ L'aperçu se ferme et la fenêtre montre le nouvel objet.

9. **Un objet illisible** : dans Konsole, sélectionne et copie (Ctrl+Maj+C) le texte `Item Class: Belts` seul, sur une ligne. La fenêtre dit que l'objet n'a pas pu être lu ; ouvre alors l'aperçu.
   ✅ La case dit « Joindre l'objet qui n'a pas pu être lu », cochée ; sur GitHub, « Item » contient `Item Class: Belts`.

10. **L'objet gardé sous les réglages** : copie un objet, ouvre les réglages, puis clique sur le bug.
    ✅ L'aperçu propose toujours de joindre cet objet, par son nom.

11. **Rouvert en bas** : dans l'aperçu, remonte tout en haut de la boîte du journal, ferme l'aperçu, puis rouvre-le.
    ✅ La boîte du journal est de nouveau déroulée jusqu'à sa dernière ligne.

12. **En français et en anglais** : change la langue dans les réglages, puis rouvre l'aperçu.
    ✅ Les textes de l'aperçu changent de langue ; le résumé et le journal restent en anglais.

13. **Une petite fenêtre** : réduis la fenêtre à sa hauteur minimale, aperçu ouvert.
    ✅ L'aperçu défile ; le bouton **Ouvrir sur GitHub** reste atteignable.

14. **Déconnecté de GitHub** : dans une fenêtre de navigation privée de ton navigateur par défaut (ou après t'être déconnecté de GitHub), rouvre sur GitHub.
    ✅ GitHub demande de se connecter ; une fois connecté, il revient au formulaire, ses champs toujours remplis.

15. **Tes propres scénarios** : ce qui te vient à l'esprit.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Avant le premier objet | | |
| 2. Le résumé | | |
| 3. Le journal | | |
| 4. Fermer | | |
| 5. Depuis un objet | | |
| 6. Ouvrir sur GitHub | | |
| 7. Sans l'objet | | |
| 8. Un objet copié ferme l'aperçu | | |
| 9. Un objet illisible | | |
| 10. L'objet gardé sous les réglages | | |
| 11. Rouvert en bas | | |
| 12. En français et en anglais | | |
| 13. Une petite fenêtre | | |
| 14. Déconnecté de GitHub | | |
| 15. Tes propres scénarios | | |
