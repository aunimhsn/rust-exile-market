# Filtres des derniers objets mémorisés : les tests

## Pourquoi

Tu avais trouvé le souci au test libre de 14b2 : Ctrl+D sur A, un min tapé, Ctrl+D sur B, puis de nouveau sur A, et A revenait avec ses filtres remis à zéro. Maintenant, les 10 derniers objets copiés gardent leurs filtres et leurs dernières offres. Fais les tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

## Avant

1. Ferme Exile Market et Exiled Exchange 2.
2. Lance, dans Konsole (pas dans le terminal de VS Code : son bac à sable Flatpak n'a pas X11, la fenêtre ne s'ouvre pas) :

   ```
   cargo run --release
   ```

   dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR).

3. Lance le jeu en **plein écran fenêtré**, dans ton repaire, avec au moins trois objets rares dans l'inventaire (A, B, C).

## Les tests

1. **Le cas trouvé** : **Ctrl+D** sur A, **« Show N stats »**, coche une stat et tape `12` dans son **min**, puis décoche un total pseudo (ou coche une autre stat, si A n'en a pas). Puis **Ctrl+D** sur B, puis de nouveau sur A.
   ✅ B arrive avec ses filtres par défaut ; A revient avec son `12` et ton autre choix.

2. **Les offres** : sur A, clique sur la **loupe**, et attends les offres. **Ctrl+D** sur B, puis sur A.
   ✅ A revient avec ses offres et leur âge (« searched …s ago »), sans nouvelle recherche.

3. **Le double chevron** : sur A, quand plus de 10 offres sont trouvées (« 10 of 13 online »), **Ctrl+D** sur B, **loupe** sur B, attends ses offres. Puis **Ctrl+D** sur A et clique sur le **double chevron** sous la liste.
   ✅ Les 10 offres suivantes ajoutées sont bien des offres de A (même base, et proches de A pour les stats cochées), pas celles de B. C'est aussi le point 8 de la revue : la suite d'une ancienne recherche se charge encore. Refais-le en laissant passer **5 minutes** avant de revenir sur A ; si le message « The trade site can't be reached. » apparaît, note-le.

4. **Ctrl+C aussi** : dans la fenêtre principale (pas la fenêtre par-dessus le jeu), copie A avec **Ctrl+C**, puis B, puis A.
   ✅ A revient avec ses filtres et ses offres, dans la fenêtre principale comme dans celle par-dessus le jeu.

5. **Un nombre en cours de saisie** : **Ctrl+D** sur A, clique dans un champ **max** et tape `99999` sans appuyer sur Entrée. **Ctrl+D** sur B, puis sur A.
   ✅ A revient avec le max ramené à la plus haute valeur de la stat (comme quand on quitte le champ), pas `99999`.
   Puis la même chose dans la **fenêtre principale** : tape `99999` dans un max de A sans valider, reviens dans le jeu, **Ctrl+C** sur B, puis sur A.
   ✅ Pareil, et le curseur n'est pas resté dans le champ de B.

6. **Le même objet** : sur A avec ses filtres, copie un autre texte (n'importe quel mot hors du jeu), puis **Ctrl+C** sur A.
   ✅ A garde ses filtres et ses offres.

7. **Le changement de ligue** : avec des offres sur A, **Ctrl+D** sur B, puis change de ligue dans la fenêtre principale, puis **Ctrl+D** sur A.
   ✅ A revient avec ses filtres, mais sans offres (« One search per click… »).

8. **Test libre** : trie un peu ton butin, comme d'habitude, en passant d'un objet à l'autre.
   ✅ Rien d'étrange : note tout ce qui te surprend.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. Le cas trouvé | | |
| 2. Les offres | | |
| 3. Le double chevron | | |
| 4. Ctrl+C aussi | | |
| 5. Un nombre en cours de saisie | | |
| 6. Le même objet | | |
| 7. Le changement de ligue | | |
| 8. Test libre | | |

Le journal : `~/.local/state/exile-market/exile-market.log`.
