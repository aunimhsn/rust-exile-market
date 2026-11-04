# L'avis de mise à jour (15) : les tests

## Pourquoi

Au lancement, Exile Market demande une fois à GitHub sa dernière version. Si elle est plus récente que celle qui tourne, une bulle sous le bloc de la ligue dit « Exile Market **0.2.0** is available. », avec **Download** qui ouvre la page de la version, et une croix. Fermée, la bulle ne revient pas pour cette version pendant la ligue en cours ; elle revient pour une version plus récente, et quand une nouvelle ligue commence. Tant qu'une version plus récente existe, la barre d'état des réglages la dit, en lien. Fais ces tests d'affilée, sans m'attendre. Un ✅ suffit quand ça marche ; détaille seulement ce qui cloche. Puis dis-moi « fait ».

Ces tests ne se font qu'une fois une première version publiée sur GitHub (étape 17) : avant, GitHub n'en connaît aucune, et rien ne s'affiche.

Pour lire le journal, garde un second terminal ouvert :

```
tail -f ~/.local/state/exile-market/exile-market.log
```

## Avant

1. Ferme Exile Market.
2. Dans `Cargo.toml` (celui de la racine), remplace la `version` par une version plus ancienne que la dernière publiée sur GitHub, par exemple `0.0.9`. **Ne commite pas ce changement**, remets la version à la fin.
3. Dans le fichier des réglages, `~/.config/exile-market/settings.toml`, supprime la section `[closed_update]` si elle existe, et mets `league = "Standard"` (pour voir aussi la bulle d'une nouvelle ligue). Enregistre.
4. Lance, dans Konsole :

   ```
   cargo run --release
   ```

## Les tests

1. **La bulle** : regarde la fenêtre quelques secondes après le lancement.
   ✅ Sous la bulle de la nouvelle ligue, alignée à gauche, une seconde bulle dit « Exile Market **0.1.0** is available. » (la dernière version publiée, en gras), avec **Download** et une croix. Le journal dit « Update check: 0.1.0 is available ».

2. **Le lien des réglages** : ouvre les réglages, puis survole le lien de la barre d'état.
   ✅ La bulle disparaît pendant que les réglages sont ouverts. La barre d'état dit « Exile Market 0.0.9 · 0.1.0 available · Made by Sylent · sylent.art » ; « 0.1.0 available » est gris, s'éclaircit et se souligne sous le pointeur, comme `sylent.art`. Un clic ouvre la page de la version dans le navigateur. Ferme les réglages.

3. **Download** : clique sur **Download**.
   ✅ Le navigateur ouvre la page de la version sur GitHub (`…/releases/tag/v0.1.0`), et la bulle disparaît. La bulle de la nouvelle ligue reste, remontée à sa place.

4. **De retour au lancement suivant** : ferme Exile Market et relance-le.
   ✅ La bulle de la mise à jour revient : tu as ouvert la page, mais tu n'as pas installé la version.

5. **Fermée pour la ligue** : clique sur la croix de la bulle de la mise à jour, puis ferme Exile Market, regarde le fichier des réglages, et relance-le.
   ✅ Le fichier contient une section `[closed_update]` avec `version = "0.1.0"` et `league = "…"`, le nom de la ligue en cours. Au lancement, la bulle de la mise à jour ne revient pas ; le lien des réglages est toujours là.

6. **Revenue avec une nouvelle ligue** : ferme Exile Market, et dans `[closed_update]` remplace le nom de la `league` par un autre, par exemple `Old League`. Relance.
   ✅ La bulle revient : pour Exile Market, une nouvelle ligue a commencé depuis sa fermeture.

7. **En français** : dans les réglages, choisis *Français*, ferme-les, puis relance Exile Market.
   ✅ La bulle dit « Exile Market **0.1.0** est disponible. », sur deux lignes, avec **Télécharger** ; dans les réglages, « 0.1.0 disponible ».

8. **Sans réseau** : ferme Exile Market et relance-le sans réseau :

   ```
   unshare -rn cargo run --release --offline
   ```

   ✅ Aucune bulle de mise à jour ni lien dans les réglages ; le journal dit « Update check: the releases can't be read (…) ».

9. **À jour** : ferme Exile Market, remets la `version` de `Cargo.toml` comme avant (`git checkout Cargo.toml`), et relance.
   ✅ Aucune bulle de mise à jour ni lien dans les réglages ; le journal dit « Update check: 0.1.0 is the latest version » (ou la version en cours).

10. **Tes propres scénarios** : ce qui te vient à l'esprit.

## Résultats

| Test | Résultat | Remarque |
|---|---|---|
| 1. La bulle | | |
| 2. Le lien des réglages | | |
| 3. Download | | |
| 4. De retour au lancement suivant | | |
| 5. Fermée pour la ligue | | |
| 6. Revenue avec une nouvelle ligue | | |
| 7. En français | | |
| 8. Sans réseau | | |
| 9. À jour | | |
| 10. Tes propres scénarios | | |
