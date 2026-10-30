# Textes de l'interface en français.
# Chaque message doit aussi exister dans en.ftl, la langue de référence.

## Barre de titre

# Le mode de la ligue, donné seulement par la couleur de son bloc et dans son infobulle.
league-mode-softcore = Softcore
league-mode-hardcore = Hardcore
# Infobulle du bloc de la ligue : son mode, puis ce que fait un clic.
league-tooltip = { $mode } · Changer de ligue
# À la place de la ligue, avant que les ligues arrivent au premier lancement.
league-loading = Chargement…
# Bulle sous le bloc de la ligue, quand une nouvelle ligue a commencé : son nom, puis un bouton.
new-league = { $league } a commencé.
new-league-switch = Y aller
# Infobulle de l'emblème, qui ramène à l'objet.
home-tooltip = Accueil
settings-tooltip = Réglages
minimize-tooltip = Réduire
close-tooltip = Fermer

## Barre d'état

status-bar = { $prices } · { $trade }
# Aucun prix de la ligue, et la base des prix est injoignable.
status-prices-unavailable = poe2scout : pas encore de prix
# Pendant le premier téléchargement des prix d'une ligue.
status-prices-downloading = poe2scout : téléchargement des prix…
# L'âge des prix, par exemple « il y a 12min ».
status-prices = poe2scout { $age }
status-prices-offline = poe2scout { $age } (hors ligne)
status-trade-idle = GGG trade : pas encore de recherche
# L'âge de la dernière recherche du site d'échange, par exemple « il y a 2min ».
status-trade = GGG trade { $age }
# Pendant une longue attente avant la prochaine recherche, par exemple « 4min ».
status-trade-wait = GGG trade : prochaine recherche dans { $wait }
# Pendant que les réglages sont ouverts : la version et l'auteur ; le site, en dernier, est un lien.
status-author = { $app } { $version } · Créé par { $author } · { $site }

## Fenêtre avant le premier objet

# Suivi des touches « Ctrl + C ».
empty-copy = Copiez un objet en jeu avec
empty-price = Son prix s'affichera ici.

## Objet copié illisible

item-unreadable = Cet objet n'a pas pu être lu.
item-unreadable-hint = Copiez-le de nouveau, ou copiez un autre objet.

## Téléchargement des prix

# À la place de l'invitation à copier un objet, tant qu'aucun prix de la ligue n'est connu.
download-title = Téléchargement des prix de { $league }
# Sous la barre, une tirée au hasard à chaque téléchargement.
download-quote-1 = On compte les Orbes exaltés un par un…
download-quote-2 = On négocie avec les marchands de Wraeclast…
download-quote-3 = On astique les Orbes divins…
download-quote-4 = On vérifie qu'aucun Miroir n'est vendu 1 Orbe du chaos…
download-quote-5 = On ignore les messages « still available? »…
download-quote-6 = On range les onglets du coffre. Encore…
download-quote-7 = Orbes du chaos en Orbes exaltés : le compte est bon…
download-quote-8 = On cherche le vendeur parti AFK dans son repaire…
download-quote-9 = Une requête par seconde : poe2scout mérite du respect…
download-quote-10 = On repère les offres appâts…

## Âges, arrondis à leur plus grande unité

age-now = à l'instant
age-minutes = il y a { $minutes }min
age-hours = il y a { $hours }h
age-days = il y a { $days }j

## Attentes, arrondies vers le haut

# Par exemple « 8s », « 4min ».
wait-seconds = { $seconds }s
wait-minutes = { $minutes }min

## Avertissements

clipboard-unreadable = Le presse-papiers ne peut pas être lu.
# L'âge des prix, par exemple « il y a 3h ».
prices-unreachable = La base des prix est injoignable. Dernière mise à jour { $age }.
prices-unavailable = Prix indisponibles, nouvelle tentative…

## Bloc objet

# Ouvre la recherche de l'objet sur le site d'échange, dans le navigateur.
check-on-trade = Voir sur l'échange
# Les tags après la classe de l'objet, courts, proches des mots du client du jeu.
tag-corrupted = Corrompu
tag-twice-corrupted = Double corruption
tag-sanctified = Sanctifié
tag-fractured = Fissuré
tag-mirrored = Reflété
tag-unidentified = Non identifié
# L'infobulle d'un tag d'état, un filtre de la recherche : gris, les offres avec ou sans cet état ;
# allumé par un clic, les offres de cet état seulement. Double corruption gris dit « Corrompues ou non ».
state-corrupted-or-not = Corrompues ou non
state-corrupted-only = Offres corrompues seulement
state-twice-corrupted-only = Offres doublement corrompues seulement
state-sanctified-or-not = Sanctifiées ou non
state-sanctified-only = Offres sanctifiées seulement
state-fractured-or-not = Fissurées ou non
state-fractured-only = Offres fissurées seulement
state-mirrored-or-not = Reflétées ou non
state-mirrored-only = Offres reflétées seulement
tag-quality = Q { $value }%
tag-sockets = { $value ->
    [one] { $value } châsse
   *[other] { $value } châsses
}
tag-item-level = niv. { $value }
tag-stack = Pile { $size }/{ $max }
# Les valeurs principales de la base, chacune suivie de son nombre en gras : le DPS d'une arme,
# les défenses d'une armure ou d'un bouclier.
dps-physical = DPS physique
dps-elemental = DPS élémentaire
dps-chaos = DPS de chaos
defence-armour = Armure
defence-evasion = Évasion
defence-energy-shield = Bouclier d'énergie
defence-block = Blocage
defence-runic-ward = Sauvegarde runique
# Le résumé d'un objet qui octroie des aptitudes, par exemple « Niveau 18 Sceau de puissance », jointes par « · ».
granted-skill = Octroie l'aptitude : { $skill }
# La ligne des prérequis, ses parties jointes par « · » : « Requiert niveau 65 · 145 Dex ».
requires = Requiert { $parts }
requirement-level = niveau { $value }
requirement-strength = { $value } For
requirement-dexterity = { $value } Dex
requirement-intelligence = { $value } Int

## Stats de l'objet

# La pilule sous le bloc objet, et une fois dépliée.
show-stats = { $count ->
    [one] Afficher { $count } stat
   *[other] Afficher { $count } stats
}
hide-stats = Masquer les stats
# Les petits titres des stats.
group-enchant = Enchantement
group-runes = Runes
group-implicit = Implicite
group-prefixes = Préfixes
group-suffixes = Suffixes
group-explicit = Explicite
# Les modificateurs d'un unique, sans préfixe ni suffixe.
group-unique = Unique
# Les filtres du type de base, du niveau d'objet, des châsses, de la qualité et du niveau requis d'un objet rare, magique ou normal.
group-item = Objet
group-weapon = Arme
# Les totaux que le site d'échange additionne sur plusieurs modificateurs.
group-pseudo = Pseudo
# À côté de Préfixes et Suffixes : combien l'objet en a, sur le maximum possible.
affix-fill = { $filled } sur { $max }
affix-fill-open = { $filled } sur { $max } · { $open ->
    [one] { $open } libre
   *[other] { $open } libres
}
# La pilule d'un objet sans modificateurs, qui n'a que des filtres, et une fois dépliée.
show-filters = Afficher les filtres
hide-filters = Masquer les filtres
# À côté de la pilule : les filtres cochés pour la recherche.
active-filters = { $count ->
    [one] { $count } filtre
   *[other] { $count } filtres
}
# À droite du premier petit titre, au-dessus des champs des filtres.
filter-range = min – max
# L'infobulle du champ d'un filtre : les jets qu'il accepte, puis, en gris, pour une ligne où une valeur basse est meilleure.
filter-rolls = Jets de { $low } à { $high }
filter-lower-better = Plus c'est bas, mieux c'est
# Les filtres des propriétés d'un objet, avec leur valeur.
filter-base-type = Base : { $base }
# Sous le type de base décoché : la classe de l'objet copié, cherchée à sa place.
filter-item-class = Classe cherchée : { $class }
filter-item-level = Niveau de l'objet : { $value }
filter-sockets = Châsses : { $value }
filter-quality = Qualité : { $value }%
filter-required-level = Niveau requis : { $value }
filter-physical-dps = DPS physique : { $value }
filter-elemental-dps = DPS élémentaire : { $value }
# Les filtres des totaux pseudo, avec leur valeur.
pseudo-elemental-resistance = +{ $value }% aux Résistances Élémentaires au total
pseudo-chaos-resistance = +{ $value }% à la Résistance au chaos au total
pseudo-life = +{ $value } de Vie maximale au total
pseudo-mana = +{ $value } de Mana maximal au total
pseudo-strength = +{ $value } de Force au total
pseudo-dexterity = +{ $value } de Dextérité au total
pseudo-intelligence = +{ $value } d'Intelligence au total
pseudo-all-attributes = +{ $value } à tous les Attributs au total
# Dans le groupe Objet : le nombre de modificateurs non révélés cherché.
pseudo-unrevealed-modifiers = Modificateurs non révélés : { $value }
# L'infobulle d'une ligne que le site d'échange ne peut pas chercher (un modificateur ajouté par un patch), sur deux lignes.
filter-unsearchable = Le site d'échange ne peut pas chercher cette ligne.
filter-unsearchable-hint = Il se peut que les offres ne l'aient pas.
# Après la plage du palier d'un modificateur : comment il a été ajouté à l'objet, dans sa couleur.
origin-desecrated = Profané
origin-fractured = Fissuré
origin-crafted = Artisanat
# Le modificateur vaal d'un unique corrompu : le mot du jeu, le même dans toutes les langues.
origin-vaal = Vaal
# Un modificateur profané pas encore révélé, dans son groupe ; et la note sous les stats de son objet.
unrevealed-prefix = Préfixe non révélé
unrevealed-suffix = Suffixe non révélé
unrevealed-note = Un modificateur non révélé peut changer l'objet : ses autres stats partent décochées.
# Sous les stats d'un objet rare ou magique copié depuis l'échange du jeu, sans les affixes ni les paliers de la copie avancée.
trade-copy-note = Les paliers sont trouvés d'après les valeurs.
# Après la plage d'un palier trouvé d'après les valeurs, quand la ligne peut être un préfixe ou un suffixe : l'affixe de ce palier, en gris.
affix-prefix = Préfixe
affix-suffix = Suffixe
# L'infobulle de « Suffixe » sous une ligne qui additionne un préfixe et un suffixe de la même stat, le préfixe écrit au-dessus.
added-up-with-prefix = Additionné au préfixe sur cette ligne
# Après la plage de la part d'un hybride dans une ligne qui additionne deux modificateurs de la même stat, et sous ses autres lignes, en gris ; avec son affixe quand l'autre part est de l'autre affixe (un préfixe hybride additionné à un suffixe pur).
hybrid = Hybride
hybrid-prefix = Préfixe hybride
hybrid-suffix = Suffixe hybride
# L'infobulle de « Hybride » sur la ligne qui additionne la part d'un hybride : les autres lignes de l'hybride, p. ex. « Avec +195 au Score de Précision ».
hybrid-with = Avec { $lines }
# Deux autres lignes d'un hybride dans l'infobulle ci-dessus : « +38 à l'Armure et +35 au Score d'Évasion ».
hybrid-lines-and = { $first } et { $second }
# L'infobulle de « Hybride » sous les autres lignes de l'hybride : la plage de sa part sur la ligne qui l'additionne, puis cette ligne, p. ex. « Avec 75–79 des 258% d'Augmentation des Dégâts Physiques ».
hybrid-part = Avec { $ranges } des { $line }

## Bloc prix

price-label = Prix
# Le prix d'un objet rare, magique ou normal, d'après les offres d'objets similaires.
estimated-price-label = Prix estimé
# À droite du titre du prix.
offers = { $count ->
    [one] { $count } offre
   *[other] { $count } offres
}
stack-of = Pile de { $size }
# Sous le prix d'une pile : « Prix unitaire : 0.25 div · 2.49 c · 182 ex ».
unit-price = Prix unitaire : { $prices }
# La variation d'abord, avec un vrai signe moins.
trend-rise = +{ $percent }% sur les { $days } derniers jours
trend-fall = −{ $percent }% sur les { $days } derniers jours
trend-stable = Stable sur les { $days } derniers jours
uncertain = { $count ->
    [0] Aucune offre récente : le prix est incertain
    [one] Une seule offre : le prix est incertain
   *[other] Seulement { $count } offres : le prix est incertain
}
# À droite du titre d'un prix estimé : les offres similaires trouvées.
similar = { $count ->
    [one] { $count } similaire
   *[other] { $count } similaires
}
# Sous un prix estimé d'après moins de 5 offres, les offres suspectes mises à part.
estimate-uncertain = { $count ->
    [one] Une seule offre similaire : l'estimation est incertaine
   *[other] Seulement { $count } offres similaires : l'estimation est incertaine
}
# Sous un prix estimé : comment il est calculé.
estimate-median = { $count ->
    [one] Prix de la seule offre similaire.
   *[other] Médiane des { $count } offres similaires.
}
estimate-median-cheapest = Médiane des { $count } offres similaires les moins chères.
# Un unique non identifié dont la base a plusieurs uniques.
identify-to-price = Identifiez l'objet pour obtenir son prix.
possible-uniques = Uniques possibles
# À la place du prix d'un unique possible sans offre.
no-offers = Aucune offre
# Les objets rares, magiques et normaux, dont le prix s'estime d'après les offres d'objets similaires.
no-market-price-rare = Pas de prix de marché pour les objets rares.
no-market-price-magic = Pas de prix de marché pour les objets magiques.
no-market-price-normal = Pas de prix de marché pour les objets normaux.
search-to-estimate = Recherchez des offres similaires pour l'estimer.
# Un objet à nom fixe dont le prix n'est pas encore connu, ou manquant.
prices-downloading = Les prix de { $league } sont en cours de téléchargement…
# Avant que la ligue soit connue, au premier lancement.
prices-downloading-no-league = Téléchargement des prix…
french-names-downloading = Les noms français des objets sont en cours de téléchargement…
no-price-found = Aucun prix trouvé pour cet objet dans { $league }.

## Bloc offres

offers-label = Offres
# Avant la première recherche de l'objet, à côté du bloc de recherche.
offers-search-hint = Une recherche par clic, dans les limites du site d'échange.
offers-search-tooltip = Chercher les offres
offers-searching = Recherche sur le site d'échange…
# Sous le titre, par exemple « recherche il y a 2min ».
offers-searched = recherche { $age }
# À droite du titre : les offres affichées, parmi celles trouvées.
offers-count = { $shown } sur { $total } en ligne
# Les titres des colonnes des offres.
offers-column-price = Prix
offers-column-per-unit = Par unité
offers-column-stock = Stock
offers-column-listed = Publiée
offers-again-tooltip = Chercher à nouveau
# L'infobulle du bloc sombre pendant qu'on attend la réponse de la dernière recherche ou
# des offres suivantes.
offers-busy-tooltip = En attente de la réponse du site d'échange
# L'infobulle du compte à rebours avant la prochaine recherche, par exemple « 8s ».
offers-next-search-tooltip = Prochaine recherche dans { $wait }
# Sous le titre, pendant une longue attente, par exemple « 4min ».
offers-limit = Limite du site d'échange : recherche dans { $wait }.
offers-none = Aucune offre en ligne avec ces filtres.
offers-none-hint = Décochez une stat ou baissez un minimum, puis cherchez à nouveau.
offers-unreachable = Le site d'échange est injoignable.
offers-identify = Identifiez l'objet pour chercher ses offres.
# Le double chevron sous la liste.
offers-more-tooltip = Afficher plus d'offres
# L'infobulle d'une offre : un prix très en dessous du marché, le statut du vendeur, et la
# pastille qui copie le message (whisper) du site d'échange.
offers-suspicious = Attention : très en dessous du prix du marché.
seller-online = En ligne
seller-away = Absent
seller-offline = Hors ligne
copy-whisper = Copier le whisper
whisper-copied = Copié

## Réglages

settings-title = Réglages
# Libellé au-dessus des choix de la langue.
settings-language = Langue
# La langue du système, quand l'interface l'a.
language-system = Système
# Chaque langue est nommée dans sa propre langue, pour qu'un joueur trouve la sienne dans n'importe quelle langue.
language-english = English
language-french = Français
settings-always-on-top = Toujours au premier plan
# Demandée par Grinding Gear Games pour les outils de la communauté.
disclaimer = Ce produit n'est ni affilié à Grinding Gear Games ni approuvé par ce studio, de quelque manière que ce soit.
