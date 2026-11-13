use super::*;
use crate::game_data::trade_stats;
use crate::item::{
    Affix, CopiedItem, Item, ModifierKind, ModifierOrigin, Roll, StatLine, test_items,
};

fn modifier(kind: ModifierKind, origin: Option<ModifierOrigin>, lines: &[&str]) -> Modifier {
    Modifier {
        kind,
        origin,
        name: None,
        tier: None,
        tags: Vec::new(),
        is_unrevealed: false,
        lines: lines.iter().map(|line| StatLine::parse(line)).collect(),
        doubt: None,
    }
}

fn explicit(lines: &[&str]) -> Modifier {
    modifier(ModifierKind::Explicit(None), None, lines)
}

/// The stat of a modifier of one line.
fn stat(language: Language, modifier: &Modifier) -> StatMatch<'static> {
    stat_with(language, modifier, None)
}

/// The stat of a modifier of one line, with the stats of the trade site `trade`.
fn stat_with<'a>(
    language: Language,
    modifier: &Modifier,
    trade: Option<&'a TradeStats>,
) -> StatMatch<'a> {
    match modifier_stats(language, modifier, trade).as_slice() {
        [
            LineMatch {
                stat: Some(stat), ..
            },
        ] => stat.clone(),
        other => panic!("{modifier:?}: {other:?}"),
    }
}

#[test]
fn the_numbers_and_their_signs_are_read() {
    let numbers = numbers("+15% to -2.5 X-10 ricochets 2 times.");

    let read: Vec<f64> = numbers.iter().map(Number::value).collect();
    assert_eq!(read, [15.0, -2.5, 10.0, 2.0]);
    assert_eq!((numbers[0].start, numbers[0].digits), (0, 1));
    assert_eq!(numbers[2].start, numbers[2].digits);
}

#[test]
fn a_line_finds_its_stat_and_its_values() {
    let found = stat(
        Language::English,
        &explicit(&["Adds 10(1-16) to 273(239-300) Lightning Damage"]),
    );

    assert_eq!(found.stat.reference, "Adds # to # Lightning Damage");
    assert_eq!(found.values, [10.0, 273.0]);
    assert_eq!(found.matcher_value, None);
}

#[test]
fn the_sign_goes_into_the_value() {
    let found = stat(Language::English, &explicit(&["+42 to maximum Life"]));
    assert_eq!(
        (found.stat.reference.as_str(), found.values.as_slice()),
        ("# to maximum Life", [42.0].as_slice())
    );

    let found = stat(Language::English, &explicit(&["-10% to Cold Resistance"]));
    assert_eq!(found.values, [-10.0]);

    let found = stat(
        Language::English,
        &explicit(&["-5% to all Elemental Resistances per Socket filled"]),
    );
    assert_eq!(found.values, [-5.0]);
}

#[test]
fn a_reduced_line_is_the_opposite_of_its_stat() {
    let found = stat(
        Language::English,
        &explicit(&["20% reduced Charm Charges used"]),
    );
    assert_eq!(found.stat.reference, "#% reduced Charm Charges used");
    assert_eq!(found.values, [-20.0]);

    let found = stat(
        Language::French,
        &explicit(&["Réduit la durée des saignements vous affectant de 41(45-41) %."]),
    );
    assert_eq!(found.values, [-41.0]);
}

#[test]
fn a_number_written_in_the_string_stays_written() {
    let found = stat(
        Language::English,
        &explicit(&["25% chance for Skills to retain 40% of Glory on use"]),
    );
    assert!(found.stat.reference.contains("40% of Glory"));
    assert_eq!(found.values, [25.0]);

    let found = stat(
        Language::English,
        &explicit(&["Skills to retain 40% of Glory on use"]),
    );
    assert_eq!(found.values, Vec::<f64>::new());
    assert_eq!(found.matcher_value, Some(100.0));
}

#[test]
fn the_numbers_a_stat_counts_are_those_of_its_hashes() {
    // The 25 is a `#` of the stat, the 40 a number its string writes.
    let glory = explicit(&["25% chance for Skills to retain 40% of Glory on use"]);
    let found = stat(Language::English, &glory);
    let counted = |numbers: &[&'static str]| -> Vec<&'static str> {
        found.counted(numbers.iter().copied()).collect()
    };

    assert_eq!(counted(&["25", "40"]), ["25"]);
    // Numbers that don't line up with those the stat read stop at the fewer of both.
    assert_eq!(counted(&["25"]), ["25"]);
    assert_eq!(counted(&["25", "40", "7"]), ["25"]);
    assert_eq!(counted(&[]), [""; 0]);
}

#[test]
fn the_counted_numbers_of_lines_that_dont_line_up_with_their_stat_are_all_of_them() {
    let glory = explicit(&["25% chance for Skills to retain 40% of Glory on use"]);
    let matches = modifier_stats(Language::English, &glory, None);
    let found = matches.first().expect("its line");

    // The 25 only.
    assert_eq!(found.counted_numbers(&glory), [vec![0]]);
    // The line read with one number more than the stat read: nothing tells which the stat
    // counts, so every number is counted.
    let mut longer = glory.clone();
    longer.lines[0].rolls.push(Roll {
        value: 7.0,
        range: None,
    });
    assert_eq!(found.counted_numbers(&longer), [vec![0, 1, 2]]);
    // A line found nowhere has no stat to tell: every number too.
    let unknown = explicit(&["12 of something no stat writes 3"]);
    let matches = modifier_stats(Language::English, &unknown, None);
    let found = matches.first().expect("its line");
    assert_eq!(found.stat, None);
    assert_eq!(found.counted_numbers(&unknown), [vec![0, 1]]);
}

#[test]
fn a_stat_with_an_id_in_the_group_is_preferred_to_a_string_with_more_hashes() {
    for (language, line) in [
        (
            Language::English,
            "100% of Armour also applies to Lightning Damage",
        ),
        (
            Language::French,
            "100% de l'Armure s'applique également aux Dégâts de Foudre",
        ),
    ] {
        let found = stat(language, &explicit(&[line]));

        assert_eq!(
            found.stat.trade_ids(TradeGroup::Explicit),
            ["explicit.stat_2134207902"],
            "{language:?}"
        );
    }
}

#[test]
fn beyond_six_numbers_only_the_string_with_every_hash_is_looked_up() {
    let text = "1 2 3 4 5 6 7";
    let numbers = numbers(text);

    let strings: Vec<String> = templates(text, &numbers)
        .map(|template| template.string)
        .collect();

    assert_eq!(strings, ["# # # # # # #"]);
    assert_eq!(templates("1 2", &super::numbers("1 2")).count(), 4);
}

#[test]
fn a_string_without_number_gives_its_value() {
    let found = stat(Language::English, &explicit(&["Blind Enemies on Hit"]));

    assert_eq!(found.values, Vec::<f64>::new());
    assert_eq!(found.matcher_value, Some(100.0));
}

#[test]
fn an_option_stat_gives_its_option() {
    let found = stat(
        Language::English,
        &explicit(&["+3 to Level of all Spark Skills"]),
    );

    assert!(found.stat.option);
    assert_eq!(found.values, [3.0]);
    assert_eq!(found.matcher_value, Some(154.0));
}

#[test]
fn a_stat_on_several_lines_takes_them_together() {
    let modifier = explicit(&[
        "+30 to maximum Life",
        "Spells fire 2 additional Projectiles",
        "Spells fire Projectiles in a circle",
    ]);

    let matches = modifier_stats(Language::English, &modifier, None);

    let lines: Vec<_> = matches.iter().map(|found| found.lines.clone()).collect();
    assert_eq!(lines, [0..1, 1..3]);
    let projectiles = matches[1].stat.as_ref().unwrap();
    assert!(projectiles.stat.reference.contains("in a circle"));
    assert_eq!(projectiles.values, [2.0]);
}

#[test]
fn a_line_found_nowhere_has_no_stat() {
    let matches = modifier_stats(
        Language::English,
        &explicit(&["Grants a strange power", "+30 to maximum Life"]),
        None,
    );

    assert_eq!(matches[0].stat, None);
    assert!(matches[1].stat.is_some());
}

#[test]
fn the_group_of_the_line_picks_its_trade_id() {
    let fractured = modifier(
        ModifierKind::Explicit(None),
        Some(ModifierOrigin::Fractured),
        &["+42 to maximum Life"],
    );
    let found = stat(Language::English, &fractured);
    assert_eq!(
        found.stat.trade_ids(TradeGroup::of(&fractured)),
        ["explicit.stat_3299347043"],
        "a fractured line is searched by its explicit id: it finds the plain lines too"
    );
    let desecrated = modifier(
        ModifierKind::Explicit(Some(Affix::Suffix)),
        Some(ModifierOrigin::Desecrated),
        &["+22% to Chaos Resistance"],
    );
    assert_eq!(TradeGroup::of(&desecrated), TradeGroup::Explicit);
    let crafted = modifier(
        ModifierKind::Explicit(Some(Affix::Prefix)),
        Some(ModifierOrigin::Crafted),
        &["+42 to maximum Life"],
    );
    assert_eq!(TradeGroup::of(&crafted), TradeGroup::Crafted);

    let rune = modifier(ModifierKind::Rune, None, &["+12% to Fire Resistance"]);
    assert_eq!(TradeGroup::of(&rune), TradeGroup::Rune);
    assert_eq!(
        stat(Language::English, &rune)
            .stat
            .trade_ids(TradeGroup::Rune)
            .len(),
        1
    );
    let corruption = modifier(ModifierKind::Corruption, None, &[]);
    assert_eq!(TradeGroup::of(&corruption), TradeGroup::Enchant);
}

#[test]
fn a_local_and_a_global_stat_keep_both_ids() {
    let found = stat(Language::English, &explicit(&["+120 to Accuracy Rating"]));

    assert_eq!(found.stat.trade_ids(TradeGroup::Explicit).len(), 2);
}

/// The English stats of Exiled Exchange 2 hold the rarity of the items found with that of
/// a map (`explicit.stat_2306002879`, "#% increased Rarity of Items found in Map" on the
/// trade site, checked on 2026-10-28), the French ones apart: an equipment searches its line
/// with both ids in English (either of them), one in French, and finds the same offers,
/// since no equipment rolls the rarity of a map.
#[test]
fn the_rarity_of_the_items_found_has_the_id_of_a_map_too_in_english() {
    let ids = |language, line| {
        stat(language, &explicit(&[line]))
            .stat
            .trade_ids(TradeGroup::Explicit)
            .to_vec()
    };

    assert_eq!(
        ids(Language::English, "16% increased Rarity of Items found"),
        ["explicit.stat_3917489142", "explicit.stat_2306002879"]
    );
    assert_eq!(
        ids(
            Language::French,
            "16% d'Augmentation de la Rareté des Objets cédés"
        ),
        ["explicit.stat_3917489142"]
    );
    assert_eq!(
        ids(
            Language::French,
            "30% d'Augmentation de la Rareté des Objets trouvés dans la Carte"
        ),
        ["explicit.stat_2306002879"]
    );
}

#[test]
fn a_line_the_game_data_doesnt_know_is_found_on_the_trade_site() {
    let trade = trade_stats();
    let line = "Sacrifice up to 10 Chaos Orbs to receive double on Trial completion";

    let without = modifier_stats(Language::English, &explicit(&[line]), None);
    assert_eq!(without[0].stat, None, "not in the game data");

    let found = stat_with(Language::English, &explicit(&[line]), Some(&trade));
    assert_eq!(found.source, StatSource::TradeSite);
    assert_eq!(
        found.stat.trade_ids(TradeGroup::Explicit),
        ["explicit.stat_3076483222"]
    );
    assert_eq!(found.values, Vec::<f64>::new());
    assert_eq!(found.matcher_value, Some(49977.0), "its option");
}

#[test]
fn a_line_without_id_in_its_group_takes_the_one_of_the_trade_site() {
    let trade = trade_stats();
    let rune = modifier(
        ModifierKind::Rune,
        None,
        &["+2 to Level of all Mark Skills"],
    );

    let known = stat(Language::English, &rune);
    assert_eq!(known.source, StatSource::GameData);
    assert_eq!(known.stat.trade_ids(TradeGroup::Rune), [] as [String; 0]);

    let found = stat_with(Language::English, &rune, Some(&trade));
    assert_eq!(found.source, StatSource::TradeSite);
    assert_eq!(
        found.stat.trade_ids(TradeGroup::Rune),
        ["rune.stat_1992191903"]
    );
    assert_eq!(found.values, [2.0]);
}

#[test]
fn a_line_without_id_in_its_group_in_both_keeps_the_stat_of_the_game_data() {
    let trade = trade_stats();
    // The trade site has the Mark skills as a rune only.
    let implicit = modifier(
        ModifierKind::Implicit,
        None,
        &["+2 to Level of all Mark Skills"],
    );

    let found = stat_with(Language::English, &implicit, Some(&trade));

    assert_eq!(found.source, StatSource::GameData);
    assert_eq!(
        found.stat.trade_ids(TradeGroup::Implicit),
        [] as [String; 0]
    );
}

#[test]
fn a_french_line_of_the_game_data_keeps_its_english_string() {
    let trade = trade_stats();

    assert_eq!(
        english_string(
            Language::French,
            "10% de Réduction de l'Armure",
            TradeGroup::Explicit,
            Some(&trade)
        ),
        Some("#% reduced Armour")
    );
}

#[test]
fn a_line_of_the_game_data_is_taken_from_it_first() {
    let trade = trade_stats();

    let found = stat_with(
        Language::English,
        &explicit(&["+42 to maximum Life"]),
        Some(&trade),
    );

    assert_eq!(found.source, StatSource::GameData);
    assert_eq!(found.stat.id.as_deref(), Some("base_maximum_life"));
}

#[test]
fn a_stat_of_the_trade_site_on_several_lines_takes_them_together() {
    let trade = trade_stats();
    let modifier = explicit(&["+30 to maximum Life", "Gain 3 Patches", "while it Patches"]);

    let matches = modifier_stats(Language::English, &modifier, Some(&trade));

    let lines: Vec<_> = matches.iter().map(|found| found.lines.clone()).collect();
    assert_eq!(lines, [0..1, 1..3]);
    let patches = matches[1].stat.as_ref().unwrap();
    assert_eq!(patches.source, StatSource::TradeSite);
    assert_eq!(patches.values, [3.0]);
}

#[test]
fn a_french_line_of_the_trade_site_has_the_english_template_of_its_id() {
    let trade = trade_stats();
    let english = |text| english_string(Language::French, text, TradeGroup::Explicit, Some(&trade));

    assert_eq!(
        english("12% de Réduction des Dégâts par patch"),
        Some("#% reduced Damage per Patch")
    );
    assert_eq!(
        english("12% de Réduction de quelque chose"),
        Some("#% de Réduction de quelque chose"),
        "no English template: its own"
    );
    assert_eq!(
        english_string(
            Language::French,
            "12% de Réduction des Dégâts par patch",
            TradeGroup::Explicit,
            None
        ),
        None
    );
}

/// The parser reads the numbers of a line into its rolls (`StatLine::parse`), and
/// [`numbers`] reads them again in its text for the strings and the values the stats
/// count: the ranges of a tier are put on the rolls by their place, so both must find
/// the same numbers in the same order. Both take a point into a number only between two
/// digits: the parser used to take the point that ends a sentence too, and read no
/// value in `de 2.5.`, which a French string of the stats can write.
#[test]
fn the_parser_and_the_strings_read_the_same_numbers() {
    // The lines of an offer are read the same, by `line_template`.
    let same = |line: &str| {
        let parsed = StatLine::parse(line);
        let rolls: Vec<f64> = parsed.rolls.iter().map(|roll| roll.value).collect();
        let read: Vec<f64> = numbers(&parsed.text).iter().map(Number::value).collect();
        assert_eq!(read, rolls, "{line}");
    };
    let mut lines = 0;
    for (_, text) in test_items::ALL {
        let item = Item::parse(&CopiedItem::from_text(text).unwrap()).unwrap();
        for line in item.modifiers().iter().flat_map(|modifier| &modifier.lines) {
            let rolls: Vec<f64> = line.rolls.iter().map(|roll| roll.value).collect();
            let read: Vec<f64> = numbers(&line.text).iter().map(Number::value).collect();
            assert_eq!(read, rolls, "{}", line.text);
            lines += 1;
        }
    }
    assert!(lines >= 330, "{lines} lines");
    // Every string of the stats, a number in place of each `#`: a whole one, a decimal
    // one, and one the advanced copy writes with its range.
    let mut strings = 0;
    for language in [Language::English, Language::French] {
        for stat in Stats::of(language).all() {
            for matcher in &stat.matchers {
                for number in ["7", "2.5", "7(5-10)", "2.5(1.5-3.5)"] {
                    matcher.string.replace('#', number).lines().for_each(same);
                }
                strings += 1;
            }
        }
    }
    assert!(strings >= 9_000, "{strings} strings");
}

/// The lines the game data doesn't know yet: the French data of Exiled Exchange 2 writes
/// the notable with its English name, `Attribue Paragon` (step 21).
const UNKNOWN_LINES: [&str; 1] = ["Attribue Parangon"];

/// Every modifier line of the copied items (`src/item/test-items/`) finds its stat, and an
/// id in its group, but the [`UNKNOWN_LINES`].
#[test]
fn every_line_of_the_copied_items_finds_its_stat() {
    let mut missing = Vec::new();
    let mut found = 0;
    for (name, text) in test_items::ALL {
        let item = Item::parse(&CopiedItem::from_text(text).unwrap()).unwrap();
        for modifier in item.modifiers() {
            let group = TradeGroup::of(modifier);
            for line in modifier_stats(item.language(), modifier, None) {
                let texts: Vec<&str> = modifier.lines[line.lines.clone()]
                    .iter()
                    .map(|line| line.text.as_str())
                    .collect();
                match line.stat {
                    Some(stat) if !stat.stat.trade_ids(group).is_empty() => found += 1,
                    Some(_) => missing.push(format!("{name}: no {group:?} id: {texts:?}")),
                    None if texts.iter().all(|text| UNKNOWN_LINES.contains(text)) => {}
                    None => missing.push(format!("{name}: {texts:?}")),
                }
            }
        }
    }

    assert!(missing.is_empty(), "{missing:#?}");
    assert!(found >= 325, "{found} lines");
}
