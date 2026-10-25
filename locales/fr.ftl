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
tag-quality = Q { $value }%
tag-sockets = { $value ->
    [one] { $value } châsse
   *[other] { $value } châsses
}
tag-item-level = niv. { $value }
tag-stack = Pile { $size }/{ $max }
# Le nombre de préfixes et suffixes au palier 1, le meilleur.
tag-tier-1 = { $value } T1
# Le résumé d'une arme, ses parties jointes par « · ».
dps-physical = DPS physique { $dps }
dps-elemental = DPS élémentaire { $dps }
dps-chaos = DPS de chaos { $dps }
# Le résumé d'un objet qui octroie des aptitudes, par exemple « Niveau 18 Sceau de puissance », jointes par « · ».
granted-skill = Octroie l'aptitude : { $skill }
# La ligne des prérequis, ses parties jointes par « · » : « Requiert niveau 65 · 145 Dex ».
requires = Requiert { $parts }
requirement-level = niveau { $value }
requirement-strength = { $value } For
requirement-dexterity = { $value } Dex
requirement-intelligence = { $value } Int

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
