//! The rolls of the database on the lines of uniques copied without ranges, English and
//! French, with the modifiers poe2scout writes for them (its answer of 2026-10-30).

use super::*;
use crate::item::{CopiedItem, Item};

fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

fn written(modifiers: &[&str]) -> Vec<String> {
    modifiers
        .iter()
        .map(|&modifier| modifier.to_owned())
        .collect()
}

/// Each line of `modifiers` with the range of each of its numbers.
type Ranges<'a> = Vec<(&'a str, Vec<Option<(f64, f64)>>)>;

fn ranges(modifiers: &[Modifier]) -> Ranges<'_> {
    modifiers
        .iter()
        .flat_map(|modifier| &modifier.lines)
        .map(|line| {
            let ranges = line
                .rolls
                .iter()
                .map(|roll| roll.range.map(|range| (range.min, range.max)))
                .collect();
            (line.text.as_str(), ranges)
        })
        .collect()
}

/// The modifiers of Soul Tether written by poe2scout.
fn soul_tether() -> (Vec<String>, Vec<String>) {
    (
        written(&[
            "(15-20)% increased Charm Effect Duration",
            "Has (1-3) Charm Slots",
        ]),
        written(&[
            "+(40-60) to maximum Energy Shield",
            "You lose (5-5)% of maximum Energy Shield per second",
            "Excess Life Recovery from Leech is applied to Energy Shield",
        ]),
    )
}

fn with_rolls(item: &Item, (implicit, explicit): &(Vec<String>, Vec<String>)) -> Vec<Modifier> {
    with_unique_rolls(item.language(), item.modifiers(), implicit, explicit, None)
}

#[test]
fn an_english_copy_gets_the_ranges_of_its_lines_but_the_vaal_modifier() {
    let item = parse(include_str!(
        "../../item/test-items/en-owner-trade-unique-belt-mutated.txt"
    ));

    let modifiers = with_rolls(&item, &soul_tether());

    assert_eq!(
        ranges(&modifiers),
        [
            ("15% increased Evasion Rating", vec![None]),
            (
                "16% increased Charm Effect Duration",
                vec![Some((15.0, 20.0))]
            ),
            // Matched by its stat: poe2scout writes the plural, as the copy.
            ("Has 3 Charm Slots", vec![Some((1.0, 3.0))]),
            ("14% increased Energy Shield Recovery rate", vec![None]),
            ("+51 to maximum Energy Shield", vec![Some((40.0, 60.0))]),
            (
                "You lose 5% of maximum Energy Shield per second",
                vec![Some((5.0, 5.0))]
            ),
            (
                "Excess Life Recovery from Leech is applied to Energy Shield",
                vec![]
            ),
        ]
    );
}

#[test]
fn a_french_copy_gets_them_by_its_stats_and_a_line_poe2scout_doesnt_hold_none() {
    let item = parse(include_str!(
        "../../item/test-items/fr-trade-unique-belt-mutated.txt"
    ));

    let modifiers = with_rolls(&item, &soul_tether());

    assert_eq!(
        ranges(&modifiers),
        [
            ("15% d'Augmentation de l'Armure", vec![None]),
            (
                "16% d'Augmentation de la Durée de l'Effet des Charmes",
                vec![Some((15.0, 20.0))]
            ),
            ("Possède 2 Emplacements de Charme", vec![Some((1.0, 3.0))]),
            (
                "+40 de Bouclier d'énergie maximal",
                vec![Some((40.0, 60.0))]
            ),
            ("+20 d'Intelligence", vec![None]),
            (
                "Vous perdez 5% de Bouclier d'énergie par seconde",
                vec![Some((5.0, 5.0))]
            ),
            (
                "14% d'Augmentation de la Récupération de Bouclier d'énergie",
                vec![None]
            ),
        ]
    );
}

#[test]
fn negative_and_reversed_ranges_are_read_without_their_sign() {
    // Coward's Legacy copied from the in-game trade, French.
    let item = parse(
        "Classe d'objet: Ceintures\nRareté: Unique\nHéritage du couard\nCeinture en mailles\n\
         --------\nNiveau de l'objet: 79\n--------\n\
         11% de Réduction du nombre de Charges de Flacon utilisées (implicit)\n\
         Possède 1 Emplacement de Charme (implicit)\n--------\n\
         36% d'Augmentation de la Récupération de Vie et de Mana des Flacons\n\
         -18 de Force\n\
         Votre Vie est désormais considérée comme Basse tant que vous avez 75% de votre Vie maximale ou moins\n\
         --------\nMourir est votre devoir le plus fondamental.\n",
    );
    let poe2scout = (
        written(&[
            "(15-10)% reduced Flask Charges used",
            "Has (1-3) Charm Slots",
        ]),
        written(&[
            "(-20--10) to Strength",
            "+(20-30) to Dexterity",
            "(30-40)% increased Life and Mana Recovery from Flasks",
            "You are considered on Low Life while at (75-75)% of maximum Life or below instead",
        ]),
    );

    let modifiers = with_rolls(&item, &poe2scout);

    assert_eq!(
        ranges(&modifiers),
        [
            (
                "11% de Réduction du nombre de Charges de Flacon utilisées",
                vec![Some((10.0, 15.0))]
            ),
            ("Possède 1 Emplacement de Charme", vec![Some((1.0, 3.0))]),
            (
                "36% d'Augmentation de la Récupération de Vie et de Mana des Flacons",
                vec![Some((30.0, 40.0))]
            ),
            ("-18 de Force", vec![Some((10.0, 20.0))]),
            (
                "Votre Vie est désormais considérée comme Basse tant que vous avez 75% de votre Vie maximale ou moins",
                vec![Some((75.0, 75.0))]
            ),
        ]
    );
}

/// The ranges `poe2scout` writes, its explicit modifiers, on the explicit `lines` of a ring
/// copied from the in-game trade.
fn ring_ranges(lines: &str, poe2scout: &[&str]) -> Vec<Vec<Option<(f64, f64)>>> {
    let item = parse(&format!(
        "Item Class: Rings\nRarity: Unique\nTest Ring\nGold Ring\n--------\n\
         Item Level: 80\n--------\n{lines}\n--------\nA flavour text.\n"
    ));
    let modifiers = with_rolls(&item, &(Vec::new(), written(poe2scout)));
    ranges(&modifiers)
        .into_iter()
        .map(|(_, ranges)| ranges)
        .collect()
}

#[test]
fn a_line_of_two_numbers_gets_no_range() {
    // Mist Whisper: `Ajoute 8(8-10) à 14(13-15)` in the advanced copy.
    assert_eq!(
        ring_ranges(
            "Adds 8 to 14 Cold Damage to Attacks",
            &["Adds (13-15) to 13 Cold Damage to Attacks"]
        ),
        [vec![None, None]]
    );
}

#[test]
fn two_lines_of_one_stat_take_the_modifiers_that_hold_their_values() {
    assert_eq!(
        ring_ranges(
            "+45 to maximum Life\n+30 to maximum Life",
            &["+(20-30) to maximum Life", "+(40-50) to maximum Life"]
        ),
        [vec![Some((40.0, 50.0))], vec![Some((20.0, 30.0))]]
    );
}

#[test]
fn a_range_of_one_value_is_fixed_only_at_that_value() {
    // Sunsplinter: poe2scout writes the levels of one copy, which roll.
    let poe2scout = ["+3 to Level of all Cold Skills"];

    assert_eq!(
        ring_ranges("+3 to Level of all Cold Skills", &poe2scout),
        [vec![Some((3.0, 3.0))]]
    );
    assert_eq!(
        ring_ranges("+2 to Level of all Cold Skills", &poe2scout),
        [vec![None]]
    );
}

#[test]
fn a_legacy_value_out_of_its_range_keeps_the_range() {
    // Widened to hold the value when shown.
    assert_eq!(
        ring_ranges("+65 to maximum Life", &["+(40-60) to maximum Life"]),
        [vec![Some((40.0, 60.0))]]
    );
}

#[test]
fn a_range_across_zero_gives_no_range() {
    // Ventor's Gamble: the game writes the sign of the roll.
    assert_eq!(
        ring_ranges(
            "+25% to Fire Resistance\n-15% to Cold Resistance",
            &[
                "+(-40-40)% to Fire Resistance",
                "+(-40-40)% to Cold Resistance"
            ]
        ),
        [vec![None], vec![None]]
    );
}

#[test]
fn an_implicit_is_only_matched_with_the_implicits_of_poe2scout() {
    let item = parse(include_str!(
        "../../item/test-items/en-owner-trade-unique-belt-mutated.txt"
    ));
    // The implicit written among the explicit modifiers.
    let poe2scout = (
        Vec::new(),
        written(&["(15-20)% increased Charm Effect Duration"]),
    );

    let modifiers = with_rolls(&item, &poe2scout);

    let charm = ranges(&modifiers)
        .into_iter()
        .find(|(text, _)| text.contains("Charm Effect Duration"))
        .expect("the implicit");
    assert_eq!(charm.1, [None]);
}

#[test]
fn a_value_goes_before_each_range_and_a_parenthesis_without_range_stays() {
    assert_eq!(
        with_values("+(40-60) to maximum Life"),
        "+40(40-60) to maximum Life"
    );
    assert_eq!(
        with_values("(-20--10) to Strength"),
        "-20(-20--10) to Strength"
    );
    assert_eq!(
        with_values("Adds (8-12) to (16-22) Fire Damage"),
        "Adds 8(8-12) to 16(16-22) Fire Damage"
    );
    assert_eq!(
        with_values("Gain (some) Rage (10-20) times"),
        "Gain (some) Rage 10(10-20) times"
    );
    assert_eq!(with_values("5(5-5)% more"), "5(5-5)% more");
    assert_eq!(
        with_values("(10) and (1-x) and (-5)"),
        "(10) and (1-x) and (-5)"
    );
    assert_eq!(with_values("unclosed (10-20"), "unclosed (10-20");
}

#[test]
fn the_decimals_written_after_a_decimal_range_are_left_out_and_a_negative_range_is_read() {
    assert_eq!(
        with_values("Life Flasks gain (0.17-0.25).18 charges per Second"),
        "Life Flasks gain 0.17(0.17-0.25) charges per Second"
    );
    let item = parse(
        "Item Class: Belts\nRarity: Unique\nTest Belt\nHeavy Belt\n--------\nItem Level: 80\n\
         --------\nLife Flasks gain 0.2 charges per Second\n25% reduced maximum Life\n\
         --------\nA flavour text.\n",
    );
    let poe2scout = (
        Vec::new(),
        written(&[
            "Life Flasks gain (0.17-0.25).18 charges per Second",
            "(-30--20)% reduced maximum Life",
        ]),
    );

    assert_eq!(
        ranges(&with_rolls(&item, &poe2scout)),
        [
            (
                "Life Flasks gain 0.2 charges per Second",
                vec![Some((0.17, 0.25))]
            ),
            ("25% reduced maximum Life", vec![Some((20.0, 30.0))]),
        ]
    );
}

/// Every range poe2scout gives to the lines of an advanced copy whose ranges are taken away is
/// the range the game writes: a number without range in the copy never varies.
#[test]
fn the_rolls_of_poe2scout_are_those_of_the_advanced_copies() {
    let empty_roar = (
        written(&["Strikes deal Splash Damage"]),
        written(&[
            "Adds (40-50) to 42 Physical Damage",
            "Leeches (10-10)% of Physical Damage as Life",
            "(10-20)% chance to cause Bleeding on Hit",
            "All Attacks count as Empowered Attacks",
            "Cannot use Warcries",
        ]),
    );
    let mist_whisper = (
        Vec::new(),
        written(&[
            "Adds (13-15) to 13 Cold Damage",
            "Gain (5-5) Mana per enemy killed",
            "(30-50)% increased Freeze Buildup",
            "(30-30)% increased Chill Duration on Enemies",
            "Attacks Chain (2-2) additional times",
        ]),
    );
    let eternal_spark = (
        Vec::new(),
        written(&[
            "(50-70)% increased Energy Shield",
            "+(5-5)% to Maximum Lightning Resistance",
            "+(20-30)% to Lightning Resistance",
            "(40-40)% increased Mana Regeneration Rate",
            "(40-40)% increased Mana Regeneration Rate while stationary",
        ]),
    );
    let cowards_legacy = (
        written(&[
            "(15-10)% reduced Flask Charges used",
            "Has (1-3) Charm Slots",
        ]),
        written(&[
            "(-20--10) to Strength",
            "+(20-30) to Dexterity",
            "(30-40)% increased Life and Mana Recovery from Flasks",
            "You are considered on Low Life while at (75-75)% of maximum Life or below instead",
        ]),
    );
    let kalandras_touch = (Vec::new(), written(&["Reflects opposite Ring"]));
    let copies = [
        (
            include_str!("../../item/test-items/en-owner-unique-mace-fixed-values.txt"),
            &empty_roar,
        ),
        (
            include_str!("../../item/test-items/fr-unique-mace-fixed-values.txt"),
            &empty_roar,
        ),
        (
            include_str!("../../item/test-items/fr-unique-crossbow.txt"),
            &mist_whisper,
        ),
        (
            include_str!("../../item/test-items/en-unique-focus.txt"),
            &eternal_spark,
        ),
        (
            include_str!("../../item/test-items/fr-unique-belt-vaal.txt"),
            &cowards_legacy,
        ),
        (
            include_str!("../../item/test-items/fr-unique-ring-mirrored.txt"),
            &kalandras_touch,
        ),
    ];
    let given: usize = copies
        .into_iter()
        .map(|(text, poe2scout)| rolls_as_the_game(text, poe2scout))
        .sum();
    // Every line of one number of these uniques: 2 of The Empty Roar in each language (the
    // French bleeding by its trade id, the French data having the global stat only), 4 of
    // Mist Whisper, 5 of The Eternal Spark, 5 of Coward's Legacy but its Vaal modifier. None
    // of `Adds # to #`, which poe2scout writes wrong.
    assert_eq!(given, 18);
}

/// The number of values of the advanced copy `text` that get a range of `poe2scout` once its
/// ranges are taken away, each checked to be the range the game writes.
fn rolls_as_the_game(text: &str, (implicit, explicit): &(Vec<String>, Vec<String>)) -> usize {
    let item = parse(text);
    let without_ranges: Vec<Modifier> = item
        .modifiers()
        .iter()
        .cloned()
        .map(|mut modifier| {
            for roll in modifier.lines.iter_mut().flat_map(|line| &mut line.rolls) {
                roll.range = None;
            }
            modifier
        })
        .collect();
    let read = with_unique_rolls(item.language(), &without_ranges, implicit, explicit, None);
    let rolls = |modifiers: &[Modifier]| -> Vec<(f64, Option<Range>)> {
        modifiers
            .iter()
            .flat_map(|modifier| &modifier.lines)
            .flat_map(|line| &line.rolls)
            .map(|roll| (roll.value, roll.range))
            .collect()
    };
    let mut given = 0;
    for ((value, game), (_, found)) in rolls(item.modifiers()).into_iter().zip(rolls(&read)) {
        let Some(found) = found else {
            continue;
        };
        given += 1;
        // A number without range in the advanced copy never varies.
        let game = game
            .unwrap_or(Range {
                min: value,
                max: value,
            })
            .magnitude();
        assert_eq!(Some(found), game, "{value} of {}", item.base_type());
    }
    given
}

/// The stats of the database (English) matched with the stat of a copied line of another stat
/// by an id of the trade site they share, in each language: the lines a language writes the
/// same for two stats. A new pair, after the game data is generated again, must be checked.
#[test]
fn the_stats_matched_by_a_trade_id_are_known() {
    use crate::game_data::stats::Stats;

    let english = Stats::of(Language::English);
    let mut pairs = Vec::new();
    for language in [Language::English, Language::French] {
        for group in [TradeGroup::Explicit, TradeGroup::Implicit] {
            for written in english.all() {
                for copied in Stats::of(language).all() {
                    if !same_stat(written, copied) && same_trade_id(written, copied, group) {
                        pairs.push(format!(
                            "{language:?} {group:?}: {} / {}",
                            written.reference, copied.reference
                        ));
                    }
                }
            }
        }
    }

    assert_eq!(
        pairs,
        [
            "French Explicit: #% chance to cause Bleeding on Hit / #% chance to inflict Bleeding on Hit",
            "French Explicit: #% increased Mana Recovery from Flasks / #% increased Flask Mana Recovery rate",
            "French Explicit: #% increased Rarity of Items found / #% increased Rarity of Items found in Map",
            "French Explicit: #% maximum Player Resistances / #% to all Maximum Resistances",
            "French Implicit: #% chance to cause Bleeding on Hit / #% chance to inflict Bleeding on Hit",
            "French Implicit: #% increased Mana Recovery from Flasks / #% increased Flask Mana Recovery rate",
            "French Implicit: Grants # Rage on Hit / Gain # Rage on Hit",
        ]
    );
}

#[test]
fn the_french_rarity_gets_its_range_by_its_trade_id() {
    // The French data writes it as the rarity of a map only.
    let item = parse(
        "Classe d'objet: Bagues\nRareté: Unique\nAndvarius\nAnneau d'or\n--------\n\
         Niveau de l'objet: 80\n--------\n\
         60% d'Augmentation de la Rareté des Objets trouvés\n--------\nUn texte d'ambiance.\n",
    );
    let poe2scout = (
        Vec::new(),
        written(&["(50-70)% increased Rarity of Items found"]),
    );

    assert_eq!(
        ranges(&with_rolls(&item, &poe2scout)),
        [(
            "60% d'Augmentation de la Rareté des Objets trouvés",
            vec![Some((50.0, 70.0))]
        )]
    );
}

#[test]
fn a_range_of_one_value_followed_by_decimals_holds_no_decimal_value() {
    // Death Articulated: the integer of the value of one copy replaced by its range.
    let poe2scout = ["Lose (5-5).3% of maximum Life per second"];

    assert_eq!(
        ring_ranges("Lose 5.3% of maximum Life per second", &poe2scout),
        [vec![None]]
    );
    assert_eq!(
        ring_ranges("Lose 5% of maximum Life per second", &poe2scout),
        [vec![Some((5.0, 5.0))]]
    );
}
