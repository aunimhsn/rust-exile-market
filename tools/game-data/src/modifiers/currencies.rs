//! The modifiers only a currency adds (an essence, an alloy of Verisium), which RePoE writes
//! rolling nowhere, and the bases each one rolls on.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::lines::Line;
use super::{EQUIPMENT, Source, SourceBase, SourceModifier, SpawnWeight};
use crate::Result;

/// A modifier only a currency adds (an essence, an alloy of Verisium), by its id in `mods.json`,
/// and the tags of the bases it rolls on, as `(tag, weight)`.
pub type CurrencyModifier = (&'static str, &'static [(&'static str, u32)]);

/// The modifiers each currency adds: those of the essences, then those of the alloys.
pub const CURRENCIES: &[(Currency, &[CurrencyModifier])] =
    &[(Currency::Essence, ESSENCES), (Currency::Alloy, ALLOYS)];

/// A currency that adds modifiers of its own.
#[derive(Clone, Copy, Debug)]
pub enum Currency {
    /// An essence ([`ESSENCES`]).
    Essence,
    /// An alloy of Verisium ([`ALLOYS`]).
    Alloy,
}

impl Currency {
    /// Its name with its article, in the errors of its list.
    fn name_with_article(self) -> &'static str {
        match self {
            Self::Essence => "an essence",
            Self::Alloy => "an alloy",
        }
    }
}

impl From<Currency> for Source {
    fn from(currency: Currency) -> Self {
        match currency {
            Currency::Essence => Self::Essence,
            Currency::Alloy => Self::Alloy,
        }
    }
}

/// The modifiers only an essence adds, of the domain of the equipment, and the tags of the bases
/// each one rolls on, as `(tag, weight)`, `default` at 0 after them: RePoE writes these
/// modifiers with `default` at 0 alone, and doesn't export the essences of Path of Exile 2.
/// From Craft of Exile (`essences` of its `poec_data.json`, 2026-10-30: the item classes of each
/// modifier), checked on the trade site the same day (the bows take the values of the
/// one-handed weapons, `+2 to Level of all Attack Skills`; the `+3` of the bows of Standard is
/// that of an older patch). The ordinary essences add ordinary modifiers, with their tiers.
/// Left out: the notable of the Essence of Delirium on a body armour (`EssenceGrantedPassive`,
/// `Allocates #`), whose value is the hash of the notable, not a roll.
pub const ESSENCES: &[CurrencyModifier] = &[
    ("EssenceAbyssPrefix", &[("default", 1)]),
    ("EssenceAbyssSuffix", &[("default", 1)]),
    (
        "EssenceAttackSkillLevel1H1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceAttackSkillLevel2H1",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    ("EssenceAuraEffect1", &[("sceptre", 1)]),
    ("EssenceBreach", &[("amulet", 1), ("ring", 1)]),
    ("EssenceColdRecoupLife1", &[("helmet", 1)]),
    ("EssenceCorruptForTwoEnchantments1", &[("belt", 1)]),
    (
        "EssenceDamageasExtraCold1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraCold2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraFire1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraFire2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraLightning1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraLightning2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraPhysical1",
        &[("bow", 1), ("one_hand_weapon", 1)],
    ),
    (
        "EssenceDamageasExtraPhysical2H",
        &[("bow", 0), ("two_hand_weapon", 1)],
    ),
    ("EssenceFireRecoupLife1", &[("belt", 1)]),
    ("EssenceGlobalDefences1", &[("amulet", 1)]),
    ("EssenceGoldDropped1", &[("gloves", 1)]),
    ("EssenceIncreasedLifePercent1", &[("body_armour", 1)]),
    ("EssenceIncreasedManaPercent1", &[("ring", 1)]),
    ("EssenceLightningRecoupLife1", &[("gloves", 1)]),
    (
        "EssenceLocalRuneAndSoulCoreEffect1",
        &[("boots", 1), ("gloves", 1)],
    ),
    ("EssenceManaCostReduction", &[("focus", 1), ("wand", 1)]),
    ("EssenceManaCostReduction2H", &[("staff", 1)]),
    ("EssenceOnslaughtonKill1", &[("weapon", 1)]),
    ("EssencePercentDexterity1", &[("amulet", 1)]),
    ("EssencePercentIntelligence1", &[("amulet", 1)]),
    ("EssencePercentStrength1", &[("amulet", 1)]),
    ("EssencePhysicalDamageTakenAsChaos1", &[("body_armour", 1)]),
    (
        "EssenceReducedCriticalDamageAgainstYou1",
        &[("body_armour", 1)],
    ),
    ("EssenceSpellSkillLevel1H1", &[("wand", 1)]),
    ("EssenceSpellSkillLevel2H1", &[("staff", 1)]),
];

/// The modifiers an alloy of Verisium adds to a rare item, in place of one of its modifiers, of
/// the domain of the equipment, and the tags of the bases each one rolls on, as `(tag,
/// weight)`, `default` at 0 after them: RePoE writes them with `default` at 0 alone. From Craft
/// of Exile (`essences` of its `poec_data.json`, where the alloys are listed too) and poe2db (the
/// page of each alloy), 2026-10-31, checked on the trade site the same day (league Runes of
/// Aldur): the trade site files them `crafted` (`P0`, `S0`, named `Verisium` and `of the
/// Stars`), and they roll whatever the level of the item (a helmet of level 7 with the
/// `ManaCostEfficiency` of level 25). The Transcendent Alloy adds the cast speed and cold damage
/// of the staves (`39–47%`, `11–16%`) to the wands and the foci too, as Craft of Exile has it:
/// poe2db gives them the one-handed values (`26–31%`, `7–11%`), which the trade site shows on
/// most of them, the oldest offers from 2026-06-18, while 2,729 wands and 420 foci of the league
/// have the values of the staves (a patch of the game, the items keeping theirs). The `weapon`
/// tag is that of the martial weapons, the bows to the warstaves. Left out: three other
/// modifiers of RePoE no alloy adds (`AlloyLocalWardIncreasePercent2`,
/// `AlloySpiritPresenceAreaOfEffectHybrid1`, `AlloyManaNearbyAllyAttackSpeedHybrid1`).
pub const ALLOYS: &[CurrencyModifier] = &[
    ("AlloyAccuracyAttackSpeedHybrid1", &[("weapon", 1)]),
    ("AlloyAilmentMagnitude1", &[("weapon", 1)]),
    ("AlloyArchonDuration1", &[("helmet", 1)]),
    ("AlloyAttackAreaOfEffect1", &[("gloves", 1)]),
    ("AlloyAttackSpeedIfMissingWardRecently1", &[("gloves", 1)]),
    ("AlloyAttackSpeedRing1", &[("ring", 1)]),
    (
        "AlloyAttributeIncreasedLocalPhysicalDamageHybrid1",
        &[("weapon", 1)],
    ),
    ("AlloyBallistaLimit1", &[("crossbow", 1)]),
    ("AlloyBellLimit1", &[("warstaff", 1)]),
    (
        "AlloyCastSpeedDamageAsExtraColdHybrid1",
        &[("staff", 1), ("wand", 1), ("focus", 1)],
    ),
    (
        "AlloyCastSpeedDamageAsExtraColdHybridOneHand1",
        &[("wand", 1), ("focus", 1)],
    ),
    ("AlloyCastSpeedGloves1", &[("gloves", 1)]),
    ("AlloyChanceToChain1", &[("quiver", 1)]),
    (
        "AlloyDamageAsExtraFireTwoHandWhileMissingRunicWard1",
        &[("staff", 1)],
    ),
    (
        "AlloyDamageAsExtraFireWhileMissingRunicWard1",
        &[("wand", 1)],
    ),
    ("AlloyDamagingAilmentDuration1", &[("gloves", 1)]),
    (
        "AlloyEffectOfResistanceMods1",
        &[("amulet", 1), ("belt", 1), ("ring", 1)],
    ),
    (
        "AlloyEffectOfSocketedAugments1",
        &[("weapon", 1), ("sceptre", 1), ("staff", 1), ("wand", 1)],
    ),
    ("AlloyElementalPenetration1", &[("gloves", 1)]),
    ("AlloyElementalSkillLimit1", &[("wand", 1)]),
    (
        "AlloyExposureEffect1",
        &[("staff", 1), ("wand", 1), ("focus", 1)],
    ),
    ("AlloyFlaskChargesPerSecond1", &[("belt", 1)]),
    ("AlloyLightningDamageIgnites1", &[("talisman", 1)]),
    ("AlloyLocalWardIncreasePercent1", &[("armour", 1)]),
    ("AlloyManaCostEfficiency1", &[("helmet", 1)]),
    ("AlloyMarkEffect", &[("bow", 1)]),
    (
        "AlloyMaximumElementalInfusions1",
        &[("staff", 1), ("wand", 1), ("sceptre", 1)],
    ),
    ("AlloyMaximumRunicWard1", &[("ring", 1)]),
    ("AlloyMaximumRunicWardPercent1", &[("amulet", 1)]),
    (
        "AlloyMaximumRunicWardWeapon1",
        &[("weapon", 1), ("sceptre", 1), ("staff", 1), ("wand", 1)],
    ),
    ("AlloyMeleeStrikeRange1", &[("spear", 1)]),
    ("AlloyMinionDamagingAilmentMagnitude1", &[("sceptre", 1)]),
    ("AlloyNaturesArchon1", &[("staff", 1)]),
    ("AlloyPresenceAreaOfEffect1", &[("body_armour", 1)]),
    ("AlloyPuppetMasterChance1", &[("sceptre", 1)]),
    ("AlloyPuppeteerStacks1", &[("sceptre", 1)]),
    ("AlloyRecoverRunicWardOnCharmUse1", &[("belt", 1)]),
    ("AlloyReducedSlowPotency1", &[("body_armour", 1)]),
    ("AlloyRemnantPickupRange1", &[("gloves", 1)]),
    ("AlloyRetainGlory1", &[("mace", 1)]),
    ("AlloyRunicWardOnBlock1", &[("shield", 1)]),
    ("AlloyRunicWardRechargeRate1", &[("belt", 1)]),
    ("AlloySkillEffectDuration1", &[("boots", 1)]),
    ("AlloySpellAreaOfEffect1", &[("helmet", 1)]),
    ("AlloySpellLevelManaHybrid1", &[("staff", 1), ("wand", 1)]),
    ("AlloySpiritOnBoots1", &[("boots", 1)]),
    ("AlloyTemporaryMinionSkillLimit1", &[("boots", 1)]),
    ("AlloyTotemPlacementSpeed1", &[("focus", 1), ("shield", 1)]),
];

/// The ids of the modifiers only a currency adds start with these words in `mods.json`.
const CURRENCY_ID_PREFIXES: [&str; 2] = ["Essence", "Alloy"];

/// The currency that adds a modifier of its lists, and the spawn weights it rolls with.
pub(super) struct CurrencySpawn {
    pub(super) currency: Currency,
    pub(super) spawn: Vec<SpawnWeight>,
}

/// Whether the modifier `id` is one only a currency adds, by its flag or by its name.
pub(super) fn is_of_a_currency(id: &str, modifier: &SourceModifier) -> bool {
    modifier.is_essence_only
        || CURRENCY_ID_PREFIXES
            .iter()
            .any(|prefix| id.starts_with(prefix))
}

/// The currency and the spawn weights of each modifier of `currencies` by its id, `default` at 0
/// after its own tags. An error when one is written twice, isn't a prefix or a suffix of the
/// equipment among `modifiers`, rolls without its currency, or has a tag no base of the
/// equipment among `released` has (mistyped, or gone): the list is to be checked again, after a
/// patch of the game.
pub(super) fn currency_spawns(
    modifiers: &BTreeMap<String, SourceModifier>,
    currencies: &[(Currency, &[CurrencyModifier])],
    released: &[&SourceBase],
) -> Result<HashMap<&'static str, CurrencySpawn>> {
    let equipment_tags: BTreeSet<&str> = released
        .iter()
        .filter(|base| base.domain == EQUIPMENT)
        .flat_map(|base| base.tags.iter().map(String::as_str))
        .collect();
    let mut spawns = HashMap::new();
    let listed = currencies
        .iter()
        .flat_map(|&(currency, list)| list.iter().map(move |&(id, tags)| (currency, id, tags)));
    for (currency, id, tags) in listed {
        let currency_name = currency.name_with_article();
        let modifier = modifiers.get(id).ok_or_else(|| {
            format!("{id}: a modifier of {currency_name} that RePoE doesn't have")
        })?;
        let error = if spawns.contains_key(id) {
            Some("written twice in the list".to_owned())
        } else if modifier.domain != EQUIPMENT {
            Some(format!("of the domain {}", modifier.domain))
        } else if !modifier.is_affix() {
            Some(format!(
                "neither a prefix nor a suffix ({})",
                modifier.generation_type
            ))
        } else if modifier.spawn_weights.iter().any(SpawnWeight::rolls) {
            Some(format!("rolls without {currency_name}"))
        } else {
            tags.iter()
                .find(|(tag, _)| !equipment_tags.contains(tag))
                .map(|(tag, _)| format!("a tag no base of the equipment has, {tag}"))
        };
        if let Some(error) = error {
            return Err(format!("{id}: a modifier of {currency_name}: {error}").into());
        }
        let spawn_weight = |(tag, weight): (&str, u32)| SpawnWeight {
            tag: tag.to_owned(),
            weight,
        };
        // Every base for a modifier of `default` already.
        let default = tags
            .iter()
            .all(|&(tag, _)| tag != "default")
            .then_some(("default", 0));
        let spawn = tags
            .iter()
            .copied()
            .chain(default)
            .map(spawn_weight)
            .collect();
        spawns.insert(id, CurrencySpawn { currency, spawn });
    }
    Ok(spawns)
}

/// An error when a line of the modifier `id` of `currency` has no template, or when it has no
/// line: left out, it would be read as no modifier, without a word.
pub(super) fn check_lines(id: &str, currency: Currency, lines: &[Line]) -> Result<()> {
    if let Some(line) = lines.iter().find(|line| !line.is_known) {
        return Err(format!(
            "{id}: a modifier of {} with a line without template, {}",
            currency.name_with_article(),
            line.template
        )
        .into());
    }
    if lines.is_empty() {
        return Err(format!(
            "{id}: a modifier of {} without line, its values all 0",
            currency.name_with_article()
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
