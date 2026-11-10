# Prix dans d'autres monnaies : les tests (#156)

Lancer l'application dans le dossier de la branche à tester (le dossier du projet, ou le worktree donné avec la PR) :

```sh
cargo run --release
```

Jeu en plein écran fenêtré, Ctrl+D sur l'objet (ou Ctrl+C, l'application ouverte à côté).

## 1. Waystone rare, toutes les offres

1. Ctrl+D sur une Waystone rare (T11 ou plus).
2. Lancer la recherche des offres, puis cliquer sur le double chevron jusqu'au bout de la liste.
3. Attendu :
   - chaque ligne a l'icône de sa monnaie et son nom après, en gris (« Exalted Orb », « Divine Orb », « Vaal Orb »…) ;
   - aucune offre en Waystones (`waystone-12`) ni avec un identifiant brut ;
   - le compteur « N of M online » compte les lignes affichées ;
   - si une page entière est écartée, le chevron reste seul, sans « No offer online ».

## 2. Infobulle d'une offre

1. Survoler une offre de la liste du test 1.
2. Attendu : le prix avec l'icône seule, sans le nom ; sous le nom, « Waystone · T12 · ilvl 81 », le palier en couleur mais pas en gras.

## 3. Bijou avec une qualité de catalyseur

1. Ctrl+D sur une amulette ou une bague avec une qualité de catalyseur (« Q 40% »).
2. Déplier les stats, cocher la qualité (décochée au départ) avec son minimum, cocher aussi un modificateur (sans stat cochée, un rare n'a pas d'estimation, donc pas d'offre suspecte), lancer la recherche.
3. Attendu : une offre d'un autre type de qualité est pâle, avec « · Caster quality » (ou « Attack », « Minion »…) en gris après le nom de la monnaie ; une offre suspecte a son montant barré et « ? » après le nom, un peu détaché.

## 4. En français

1. Refaire le test 1 avec le client du jeu en français.
2. Attendu : les noms en français (« Orbe exalté », « Orbe divin », « Orbe vaal ») ; un nom trop long coupé avec « … ».

## 5. Bourse

1. Ctrl+D sur une pile d'orbes (Orbe d'annulation, Orbe vaal…), lancer la recherche.
2. Attendu : « 255 [icône] Exalted Orb », le stock et la date bien alignés.

## 6. Test libre

Essaie tes propres scénarios : d'autres objets, des listes chargées jusqu'au bout, l'autre fenêtre (au-dessus du jeu ou principale).

## Résultats

| Test | OK ? | Remarque |
|---|---|---|
| 1. Waystone, toutes les offres | | |
| 2. Infobulle | | |
| 3. Qualité de catalyseur | | |
| 4. En français | | |
| 5. Bourse | | |
| 6. Test libre | | |
