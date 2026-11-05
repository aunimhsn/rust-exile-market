# Rapport de bug (16b) : les tests

## Pourquoi

Un bug à gauche de l'engrenage ouvre l'aperçu du rapport de bug à la place du contenu de la fenêtre : le résumé (version, système, langues, ligue, réglages), les dernières lignes du journal de cette session ou de la précédente (deux pastilles jointes, la blanche glisse vers l'autre au clic ; la précédente choisie d'office après un plantage), et une case pour joindre l'objet affiché. **Copier le journal** copie tout le journal choisi. **Ouvrir sur GitHub** ouvre le formulaire des issues du dépôt dans le navigateur, ses champs remplis par l'adresse ; rien n'est envoyé par l'application. Fais les tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

**N'envoie aucune issue** : ouvre le formulaire, regarde ses champs, puis ferme l'onglet.

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

## Avant

1. Ferme Exile Market par sa croix (Ctrl+C dans le terminal compte comme un plantage : la session suivante choisirait « La précédente » d'office), Exiled Exchange 2 et le jeu.
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

4. **Le choix du journal** : regarde à droite du libellé « Journal ». Clique sur **La précédente**, puis sur **Cette session**. Puis clique vite sur l'une puis l'autre, avant la fin du glissement. Enfin, appuie sur Tab jusqu'à la pastille non choisie, puis sur Entrée.
   ✅ Deux pastilles jointes sur fond sombre, « Cette session » en blanc (si la session précédente s'est fermée normalement). Au clic, la pastille blanche glisse vers l'autre, calmement (environ un quart de seconde, elle ralentit en arrivant, sans rebond), sa largeur suit le mot ; la boîte montre alors les lignes de la session précédente (son heure de fin, sa dernière ligne `… Exile Market closed`), puis de nouveau celles de cette session. Recliquée en plein glissement, la pastille repart de là où elle est, sans saut ; la main du pointeur n'apparaît que sur la pastille non choisie. Au clavier, Entrée fait glisser la pastille blanche, un peu plus sombre tant qu'elle a le focus.

5. **Après un plantage** : dans un autre terminal, tue Exile Market sans le laisser se fermer : `pkill -9 -f target/release/exile-market`. Relance-le (`cargo run --release`), puis ouvre l'aperçu.
   ✅ « La précédente » est déjà en blanc à l'ouverture, sans glisser ; la boîte montre la session tuée, qui s'arrête sur sa dernière ligne, **sans** « Exile Market closed ». Puis ferme Exile Market normalement (la croix), relance-le et rouvre l'aperçu : ✅ « Cette session » est choisie.

6. **La première session** : ferme Exile Market, puis lance-le avec un dossier de journal vide : `XDG_STATE_HOME=$(mktemp -d) cargo run --release`. Ouvre l'aperçu.
   ✅ Pas de pastilles : « Journal » seul au-dessus de la boîte. Ferme ensuite Exile Market et relance-le normalement (`cargo run --release`).

7. **Copier le journal** : clique sur **Copier le journal**, puis colle (Ctrl+V) dans un éditeur de texte (Kate). Choisis ensuite l'autre journal, puis reviens au premier ; puis ferme l'aperçu et rouvre-le.
   ✅ La pastille devient « ✓ Copié » (la coche avant le mot, en gris), à droite de « Ouvrir sur GitHub ». Le texte collé est tout le journal de la session choisie, écrit comme dans la boîte (l'heure sans la date, *another window*), de sa première ligne (`… INFO Log file: ~/…`) à sa dernière, plus de lignes que la boîte quand la session est longue ; la fenêtre ne réagit pas à ce texte (pas de « objet illisible »). L'autre journal choisi, la pastille redevient « Copier le journal », et « ✓ Copié » revient avec le journal copié ; rouvert, l'aperçu montre « Copier le journal ».

8. **Fermer** : clique de nouveau sur le bug ; rouvre-le, puis appuie sur **Échap** ; rouvre-le, puis clique sur l'**engrenage**.
   ✅ Le bug ferme l'aperçu ; Échap aussi ; l'engrenage ouvre les réglages à sa place (l'engrenage allumé, le bug éteint), et le bug depuis les réglages rouvre l'aperçu.

9. **Depuis un objet** : lance le jeu, copie un objet unique (Ctrl+D ou Ctrl+C), puis clique sur le bug dans la fenêtre principale.
   ✅ La case « Joindre l'objet affiché : **son nom** » est cochée, le nom en gras après les deux-points ; le résumé dit aussi « Items: … » (la langue du jeu).

   Puis ferme l'aperçu, copie un objet **magique** au nom long (un flacon avec ses deux affixes), et rouvre l'aperçu.
   ✅ Si le nom ne tient pas, il est coupé par « … », jamais le début de la phrase ; au survol, une infobulle montre la phrase entière.

10. **Ouvrir sur GitHub** : clique sur **Ouvrir sur GitHub**.
    ✅ Le navigateur ouvre le formulaire « Bug report » du dépôt, sans erreur ; « Version and settings » contient le résumé, « Item » le texte de l'objet tel que copié, « Log » les mêmes lignes que la boîte de l'aperçu (la première et la dernière identiques), les espaces, les deux-points et les parenthèses intacts ; l'aperçu reste ouvert ; le journal dit « Bug report opened on GitHub: N characters, the item joined, the log of this session », avec N au plus 6000. Choisis « La précédente » et rouvre sur GitHub : ✅ « Log » a les lignes de la session précédente seulement, et le journal dit « the log of the previous session ».

11. **Sans l'objet** : décoche la case, puis rouvre sur GitHub. Si ta session est courte, copie d'abord une vingtaine d'objets, pour que le journal ne tienne plus en entier.
    ✅ La boîte du journal montre plus de lignes qu'avant (sa première ligne commence plus tôt) ; le champ « Item » du formulaire est vide ; « Log » a les lignes de l'aperçu.

12. **Un objet copié ferme l'aperçu** : aperçu ouvert, copie un autre objet dans le jeu.
    ✅ L'aperçu se ferme et la fenêtre montre le nouvel objet.

13. **Un objet illisible** : dans Konsole, sélectionne et copie (Ctrl+Maj+C) le texte `Item Class: Belts` seul, sur une ligne. La fenêtre dit que l'objet n'a pas pu être lu ; ouvre alors l'aperçu.
    ✅ La case dit « Joindre l'objet qui n'a pas pu être lu », cochée ; sur GitHub, « Item » contient `Item Class: Belts`.

14. **L'objet gardé sous les réglages** : copie un objet, ouvre les réglages, puis clique sur le bug.
    ✅ L'aperçu propose toujours de joindre cet objet, par son nom.

15. **Rouvert en bas** : dans l'aperçu, remonte tout en haut de la boîte du journal, ferme l'aperçu, puis rouvre-le.
    ✅ La boîte du journal est de nouveau déroulée jusqu'à sa dernière ligne.

16. **En français et en anglais** : change la langue dans les réglages, puis rouvre l'aperçu.
    ✅ Les textes de l'aperçu changent de langue ; le résumé et le journal restent en anglais.

17. **Une petite fenêtre** : réduis la fenêtre à sa hauteur minimale, aperçu ouvert.
    ✅ L'aperçu défile ; le bouton **Ouvrir sur GitHub** reste atteignable.

18. **La limite de l'adresse, connecté** : connecté à GitHub, fais une longue session (copie une trentaine d'objets, dont un objet rare aux nombreux modificateurs), garde-le joint, puis ouvre sur GitHub.
    ✅ Le formulaire s'ouvre sans erreur (ni 414, ni 500, ni page blanche), ses trois champs remplis ; dans le journal, N proche de 6000. Sinon, dis-moi N et l'erreur.

19. **Déconnecté de GitHub** : dans une fenêtre de navigation privée de ton navigateur par défaut (ou après t'être déconnecté de GitHub), rouvre sur GitHub.
    ✅ GitHub demande de se connecter ; une fois connecté, il revient au formulaire, ses champs toujours remplis.

20. **Tes propres scénarios** : ce qui te vient à l'esprit.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Avant le premier objet | | |
| 2. Le résumé | | |
| 3. Le journal | | |
| 4. Le choix du journal | | |
| 5. Après un plantage | | |
| 6. La première session | | |
| 7. Copier le journal | | |
| 8. Fermer | | |
| 9. Depuis un objet | | |
| 10. Ouvrir sur GitHub | | |
| 11. Sans l'objet | | |
| 12. Un objet copié ferme l'aperçu | | |
| 13. Un objet illisible | | |
| 14. L'objet gardé sous les réglages | | |
| 15. Rouvert en bas | | |
| 16. En français et en anglais | | |
| 17. Une petite fenêtre | | |
| 18. La limite de l'adresse, connecté | | |
| 19. Déconnecté de GitHub | | |
| 20. Tes propres scénarios | | |
