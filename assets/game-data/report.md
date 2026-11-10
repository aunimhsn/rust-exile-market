# Game data report

Written by `tools/game-data` with the data, from the commit cca30662bf31eaf38bd711e2ec1a6b899a06c40e of Exiled Exchange 2 and the commit 75885a099f2e9be1619aa3c793fe6ab0754a511f of the data of RePoE. Not read by the application: what the generator left out or found odd, to compare with the report of the previous patch (`git diff`).

## Bases in several categories

- `en: Runemastered Venerable Defender {"Buckler": 1, "Shield": 3}`
- `fr: Défenseur vénérable runomaîtrisé {"Buckler": 1, "Shield": 3}`

## Bases without tags

- `en: Timeless Jewel`
- `fr: Joyau intemporel`

## Modifier lines known without the + of their number

- `+#% Monster Elemental Resistances as #% Monster Elemental Resistances`

## Modifier lines without template, their modifier left out

- `# to Level of all Trap Skill Gems`
- `#% chance for Attack Hits to apply Incision (Time-Lost jewels: not known within the words of the passive skills, with its number)`
- `#% chance to Daze on Hit (Time-Lost jewels: not known within the words of the passive skills, with its number)`
- `#% increased Critical Hit Chance with Traps`
- `#% increased Fishing Line Strength`
- `#% increased Fishing Pool Consumption`
- `#% increased Fishing Range`
- `#% increased Quantity of Fish Caught`
- `#% increased Rarity of Fish Caught`
- `#% increased Trap Damage`
- `#% to Critical Damage Bonus with Traps`
- `Delirium Fog in Map spawns #% increased Mirror Shards`
- `Expeditions have +#% Surpassing chance to contain an additional Verisium Remnant`
- `Karui Stone Hook`
- `Rhoa Feather Lure`
- `minion_area_of_effect_+%`

## Modifiers of a currency that ESSENCES and ALLOYS don't list, left out

- `AlloyLocalWardIncreasePercent2`
- `AlloyManaNearbyAllyAttackSpeedHybrid1`
- `AlloySpiritPresenceAreaOfEffectHybrid1`
- `EssenceGrantedPassive`

## Prefixes and suffixes of a domain the application doesn't read, left out

- `atlas: 143`
- `crafted: 9`
- `expedition_relic: 62`
- `heist_area: 298`
- `heist_npc: 404`
- `heist_trinket: 123`
- `leaguestone: 2`
- `memory_line: 19`
- `monster: 190`
- `sanctum_relic: 138`
- `sentinel: 33`
- `strongbox: 175`
- `ultimatum_key: 31`
- `veiled: 2`

## Stats in English only

- `#% chance to cause Bleeding on Hit`
- `#% increased Mana Recovery from Flasks`
- `#% maximum Player Resistances`
- `Grants # Rage on Hit`

## Stats in French only

- `#% increased Rarity of Items found in Map`
- `#% to all Maximum Resistances`

## Stats with other trade ids in French

- `#% chance to inflict Bleeding on Hit`
- `#% increased Flask Mana Recovery rate`
- `#% increased Rarity of Items found`
- `Gain # Rage on Hit`
