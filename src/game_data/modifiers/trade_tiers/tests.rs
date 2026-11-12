//! The reading of an item copied from the in-game trade on tables written by hand: what it
//! does whatever the patch of the game. The tests of `game_tests` read the tables and the
//! items of the game, and are the ones to look at when the game data are generated again.

use super::*;
use crate::game_data::modifiers::best_reading::MAX_READINGS;
use crate::game_data::modifiers::fixtures::{
    Added, AffixTiers, ENERGY_SHIELD, LIFE, MANA, RARITY, added, equipment, modifier, read_tables,
    summary, table_json,
};
use crate::game_data::modifiers::tables::Table;

/// What [`read_in`] gives: for each modifier read, how many lines it has and the tier number
/// of each affix it can be; and the text of each line split with the affix and tier number of
/// its pure modifier.
type ReadIn = (Vec<(usize, AffixTiers)>, Added);

/// The tiers of the lines `texts` of an item of `item_level` copied from the in-game trade,
/// found among `tables` on a helmet with at most `limits` prefixes and suffixes. The tables
/// are written by hand, but the lines find their stat in the embedded stats of the game: the
/// templates of these tests ([`LIFE`], [`ENERGY_SHIELD`], [`RARITY`]) must stay among them.
fn read_in(tables: &[Table], item_level: u32, texts: &[&str], limits: (u32, u32)) -> ReadIn {
    let modifiers: Vec<Modifier> = texts.iter().map(|text| modifier(None, &[text])).collect();
    read_modifiers(tables, item_level, &modifiers, limits).0
}

/// [`read_in`] for `modifiers`, with what the search took and how it ended.
fn read_modifiers(
    tables: &[Table],
    item_level: u32,
    modifiers: &[Modifier],
    (prefixes, suffixes): (u32, u32),
) -> (ReadIn, Effort) {
    let tables = ItemTables {
        all: tables,
        base: &equipment(&["helmet"]),
        item_level: Some(item_level),
    };
    let limits = AffixLimits::new(prefixes, suffixes);
    let (found, effort) = trade_tiers_in(&tables, Language::English, modifiers, limits);
    (
        (summary(modifiers, &found), added(modifiers, &found)),
        effort,
    )
}

#[test]
fn a_line_copied_from_the_in_game_trade_takes_the_worst_tier_its_item_level_allows() {
    use Affix::Prefix;
    let tables = read_tables(&[table_json(
        "Life",
        "prefix",
        &[LIFE],
        &[(1, &[(1, 10)]), (50, &[(5, 15)])],
    )]);
    let read = |item_level, text| read_in(&tables, item_level, &[text], (3, 3)).0;

    // In both tiers: the worst.
    assert_eq!(read(60, "+7 to maximum Life"), [(1, vec![(Prefix, 2)])]);
    assert_eq!(read(60, "+12 to maximum Life"), [(1, vec![(Prefix, 1)])]);
    // A tier above the level of the item can't have rolled.
    assert_eq!(read(40, "+12 to maximum Life"), []);
    // No tier holds the value.
    assert_eq!(read(60, "+16 to maximum Life"), []);
}

#[test]
fn lines_pure_modifiers_and_their_hybrid_hold_have_no_tier_while_the_item_has_room() {
    use Affix::Prefix;
    let tables = pure_and_hybrid_tables();
    let lines = ["25% increased Energy Shield", "+25 to maximum Life"];

    // Two pure prefixes or their hybrid, the item having room for either: nothing tells which
    // rolled.
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, []);
    assert_eq!(split, []);
    // One prefix only: the hybrid, in its tier 1.
    let (read, split) = read_in(&tables, 60, &lines, (1, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(split, []);
    // In the other order of its lines too.
    let (read, _) = read_in(&tables, 60, &[lines[1], lines[0]], (1, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    // Values no tier of the hybrid holds: two pure prefixes.
    let pure = ["30% increased Energy Shield", "+30 to maximum Life"];
    let (read, _) = read_in(&tables, 60, &pure, (2, 0));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Prefix, 1)])]);
    // The tier of the hybrid that holds them above the level of the item: two pure prefixes.
    let (read, _) = read_in(&tables, 15, &lines, (2, 0));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Prefix, 1)])]);
    // A life above its pure tier adds up the hybrid and the pure modifier, after the other
    // line of the hybrid.
    let lines = ["25% increased Energy Shield", "+45 to maximum Life"];
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(split, [("+45 to maximum Life".to_owned(), Prefix, 1)]);
}

/// The tables of [`pure_and_hybrid_tables`], with a rarity that rolls as a suffix.
fn pure_hybrid_and_rarity_tables() -> Vec<Table> {
    let mut tables = pure_and_hybrid_tables();
    tables.extend(read_tables(&[table_json(
        "Rarity",
        "suffix",
        &[RARITY],
        &[(1, &[(10, 20)])],
    )]));
    tables
}

#[test]
fn the_lines_of_a_hybrid_apart_are_in_doubt_too() {
    use Affix::Suffix;
    let tables = pure_hybrid_and_rarity_tables();
    // The in-game trade sorts its lines by stat: the rarity between the lines of the hybrid.
    let lines = [
        "25% increased Energy Shield",
        "15% increased Rarity of Items found",
        "+25 to maximum Life",
    ];

    let modifiers = lines.map(|text| modifier(None, &[text]));

    // Two pure prefixes or their hybrid; the rarity, which no other reading touches, keeps
    // its tier.
    let ((read, split), effort) = read_modifiers(&tables, 60, &modifiers, (2, 1));
    assert_eq!(read, [(1, vec![(Suffix, 1)])]);
    assert_eq!(split, []);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestMet));
    // One prefix only: the hybrid is the only reading that fits, which the lines apart
    // don't let read, a modifier being read on consecutive lines; no tier rather than two
    // pure prefixes the item has no room for.
    let ((read, _), effort) = read_modifiers(&tables, 60, &modifiers, (1, 1));
    assert_eq!(read, [(1, vec![(Suffix, 1)])]);
    assert_eq!(
        effort.ending,
        Ending::OtherReadings(Outcome::OnlyAnotherFits)
    );
}

#[test]
fn a_line_read_alone_is_in_doubt_when_its_hybrid_reads_a_line_left_without_tier() {
    use Affix::{Prefix, Suffix};
    // No pure life: its line has no tier alone.
    let tables = read_tables(&[
        table_json(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(21, 30)])],
        ),
        table_json(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(20, 29), (10, 19)])],
        ),
        table_json("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
    ]);
    let (shield, rarity, life) = (
        "25% increased Energy Shield",
        "15% increased Rarity of Items found",
        "+15 to maximum Life",
    );

    // The lines of the hybrid following each other: the reading that reads the most lines.
    let (read, _) = read_in(&tables, 60, &[shield, life, rarity], (3, 3));
    assert_eq!(read, [(2, vec![(Prefix, 1)]), (1, vec![(Suffix, 1)])]);
    // Apart, the life is left without tier and the energy shield read alone, a pure prefix:
    // in doubt, since the hybrid reads both.
    let (read, _) = read_in(&tables, 60, &[shield, rarity, life], (3, 3));
    assert_eq!(read, [(1, vec![(Suffix, 1)])]);
    // A life the hybrid doesn't hold: the energy shield is the pure prefix.
    let (read, _) = read_in(
        &tables,
        60,
        &[shield, rarity, "+25 to maximum Life"],
        (3, 3),
    );
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Suffix, 1)])]);
}

#[test]
fn a_line_read_alone_is_in_doubt_when_its_hybrid_adds_up_on_a_line_left_without_tier() {
    use Affix::{Prefix, Suffix};
    let tables = pure_hybrid_and_rarity_tables();
    // The life is above its pure tier, and apart from the energy shield: no candidate splits
    // it with the hybrid, and it is left without tier.
    let lines = [
        "25% increased Energy Shield",
        "15% increased Rarity of Items found",
        "+45 to maximum Life",
    ];

    // The hybrid and the pure life add up to 45: the energy shield may be the hybrid's.
    let (read, split) = read_in(&tables, 60, &lines, (2, 1));
    assert_eq!(read, [(1, vec![(Suffix, 1)])]);
    assert_eq!(split, []);
    // No room for the hybrid and the pure life with a pure energy shield in doubt: with one
    // prefix only, nothing adds up to the life, and the energy shield is the pure prefix.
    let (read, _) = read_in(&tables, 60, &lines, (1, 1));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Suffix, 1)])]);
}

#[test]
fn a_hybrid_in_doubt_leaves_its_room_to_a_line_that_can_be_a_prefix_or_a_suffix() {
    use Affix::{Prefix, Suffix};
    let mut tables = pure_hybrid_and_rarity_tables();
    tables.extend(read_tables(&[table_json(
        "RarityPrefix",
        "prefix",
        &[RARITY],
        &[(1, &[(10, 20)])],
    )]));
    let lines = [
        "25% increased Energy Shield",
        "+25 to maximum Life",
        "15% increased Rarity of Items found",
    ];

    // Two pure prefixes would leave the rarity the suffix only, their hybrid a prefix too.
    let (read, _) = read_in(&tables, 60, &lines, (2, 1));
    assert_eq!(read, [(1, vec![(Prefix, 1), (Suffix, 1)])]);
    // Values only the pure modifiers hold: the prefixes are full, the rarity is the suffix.
    let pure = [
        "30% increased Energy Shield",
        "+30 to maximum Life",
        lines[2],
    ];
    let (read, _) = read_in(&tables, 60, &pure, (2, 1));
    assert_eq!(
        read,
        [
            (1, vec![(Prefix, 1)]),
            (1, vec![(Prefix, 1)]),
            (1, vec![(Suffix, 1)])
        ]
    );
}

#[test]
fn a_line_no_tier_holds_alone_adds_up_a_pure_modifier_and_its_hybrid() {
    use Affix::Prefix;
    let tables = read_tables(&[
        table_json(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(10, 20)]), (30, &[(21, 30)])],
        ),
        table_json(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(10, 19), (20, 29)])],
        ),
    ]);
    let lines = ["45% increased Energy Shield", "+25 to maximum Life"];

    // 45% is above the pure tier 1: the 26 to 30 of that tier, with the 15 to 19 of the hybrid.
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(
        split,
        [("45% increased Energy Shield".to_owned(), Prefix, 1)]
    );
    // The two modifiers of a split take two places: without them, no tier.
    let (read, split) = read_in(&tables, 60, &lines, (1, 0));
    assert_eq!(read, []);
    assert_eq!(split, []);
    // The pure tier 1 above the level of the item: no way to add up 45%.
    let (read, _) = read_in(&tables, 25, &lines, (2, 0));
    assert_eq!(read, []);
}

#[test]
fn a_line_a_prefix_and_a_suffix_hold_keeps_the_affix_the_room_leaves() {
    use Affix::{Prefix, Suffix};
    let tables = read_tables(&[
        table_json("Life", "prefix", &[LIFE], &[(1, &[(1, 30)])]),
        table_json("RarityPrefix", "prefix", &[RARITY], &[(1, &[(10, 20)])]),
        table_json(
            "RaritySuffix",
            "suffix",
            &[RARITY],
            &[(1, &[(6, 12)]), (40, &[(13, 18)])],
        ),
    ]);
    let rarity = "15% increased Rarity of Items found";

    // Room for either.
    let (read, _) = read_in(&tables, 60, &[rarity], (1, 1));
    assert_eq!(read, [(1, vec![(Prefix, 1), (Suffix, 1)])]);
    // The life takes the only prefix.
    let (read, _) = read_in(&tables, 60, &["+25 to maximum Life", rarity], (1, 1));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Suffix, 1)])]);
    // A value no tier holds alone adds up the prefix and the suffix, the prefix first.
    let (read, split) = read_in(
        &tables,
        60,
        &["38% increased Rarity of Items found"],
        (1, 1),
    );
    assert_eq!(read, [(1, vec![(Prefix, 1)])]);
    assert_eq!(
        split,
        [("38% increased Rarity of Items found".to_owned(), Suffix, 1)]
    );
}

/// Two pure prefixes of one tier, `21–30`, and their hybrid of two tiers, `10–19` and
/// `20–29` on each line.
fn pure_and_hybrid_tables() -> Vec<Table> {
    read_tables(&[
        table_json("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
        table_json(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(21, 30)])],
        ),
        table_json(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(10, 19), (10, 19)]), (20, &[(20, 29), (20, 29)])],
        ),
    ])
}

#[test]
fn two_fractured_lines_are_one_hybrid_whatever_the_room() {
    use Affix::Prefix;
    let tables = pure_and_hybrid_tables();
    let fractured = |text: &str| Modifier {
        origin: Some(ModifierOrigin::Fractured),
        ..modifier(None, &[text])
    };
    let lines = [
        fractured("25% increased Energy Shield"),
        fractured("+25 to maximum Life"),
    ];

    // An item has one fractured modifier: not two pure ones, even with room for them.
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (3, 3));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestMet));
    // A fractured line and another aren't the lines of one modifier, even when the item has
    // room for one prefix only: no reading fits, and both stay pure.
    let lines = [
        fractured("25% increased Energy Shield"),
        modifier(None, &["+25 to maximum Life"]),
    ];
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (1, 3));
    assert_eq!(read, [(1, vec![(Prefix, 1)]), (1, vec![(Prefix, 1)])]);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::NoneFits));
}

#[test]
fn the_lines_another_reading_adds_up_in_as_many_modifiers_have_no_tier() {
    use Affix::{Prefix, Suffix};
    // Two pure prefixes from 1: with the life of the hybrid, `10–19`, the pure life adds up
    // to 25 as well, two modifiers as the two pure ones; the hybrid alone doesn't hold 25.
    let tables = read_tables(&[
        table_json("Life", "prefix", &[LIFE], &[(1, &[(1, 30)])]),
        table_json(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(1, 30)])],
        ),
        table_json(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(20, 29), (10, 19)])],
        ),
        table_json("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
    ]);
    let rarity = "15% increased Rarity of Items found";

    // Both lines in doubt; the rarity, which no other reading touches, keeps its tier.
    let lines = ["25% increased Energy Shield", "+25 to maximum Life", rarity];
    let (read, split) = read_in(&tables, 60, &lines, (2, 1));
    assert_eq!(read, [(1, vec![(Suffix, 1)])]);
    assert_eq!(split, []);
    // At 30, the hybrid adds up with a pure energy shield and a pure life only, three
    // modifiers for the two pure ones: a modifier holding a line alone is the likelier.
    let lines = ["30% increased Energy Shield", "+30 to maximum Life", rarity];
    let (read, _) = read_in(&tables, 60, &lines, (3, 1));
    assert_eq!(
        read,
        [
            (1, vec![(Prefix, 1)]),
            (1, vec![(Prefix, 1)]),
            (1, vec![(Suffix, 1)])
        ]
    );
}

#[test]
fn another_reading_of_a_split_line_leaves_it_without_tier_whatever_it_makes() {
    use Affix::Prefix;
    let tables = read_tables(&[
        table_json(
            "EnergyShield",
            "prefix",
            &[ENERGY_SHIELD],
            &[(1, &[(10, 20)]), (30, &[(21, 30)])],
        ),
        table_json(
            "Hybrid",
            "prefix",
            &[ENERGY_SHIELD, LIFE],
            &[(1, &[(10, 19), (1, 9)]), (30, &[(15, 24), (20, 29)])],
        ),
        table_json("Life", "prefix", &[LIFE], &[(1, &[(10, 24)])]),
    ]);
    let lines = ["45% increased Energy Shield", "+25 to maximum Life"];

    // Two prefixes: the hybrid T1 and the pure energy shield T1 add up to 45, the only
    // reading.
    let (read, split) = read_in(&tables, 60, &lines, (2, 0));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    assert_eq!(
        split,
        [("45% increased Energy Shield".to_owned(), Prefix, 1)]
    );
    // A third prefix: the hybrid T2 with the same pure energy shield and a pure life, one
    // modifier more, which a line read alone would rule out and a split line doesn't.
    let (read, split) = read_in(&tables, 60, &lines, (3, 0));
    assert_eq!(read, []);
    assert_eq!(split, []);
}

#[test]
fn a_doubt_stays_in_the_part_of_the_item_it_is_about() {
    use Affix::Prefix;
    let mut tables = pure_and_hybrid_tables();
    tables.extend(read_tables(&[table_json(
        "Mana",
        "prefix",
        &[MANA],
        &[(1, &[(10, 25)]), (30, &[(20, 40)])],
    )]));
    let lines = [
        "25% increased Energy Shield",
        "+25 to maximum Life",
        "+22 to maximum Mana",
    ];

    // The energy shield and the life, two pure prefixes or their hybrid, are in doubt; the
    // mana, in two tiers of its modifier whichever reads them, keeps the worst.
    let (read, _) = read_in(&tables, 60, &lines, (3, 0));
    assert_eq!(read, [(1, vec![(Prefix, 2)])]);
}

#[test]
fn two_hybrids_that_share_a_line_leave_it_without_tier_when_both_follow_it() {
    use Affix::{Prefix, Suffix};
    let hybrid = |kind, affix, other| {
        table_json(
            kind,
            affix,
            &[ENERGY_SHIELD, other],
            &[(1, &[(20, 29), (10, 19)])],
        )
    };
    // No pure modifier: the energy shield is a hybrid with the life or with the mana, the
    // other line then without tier.
    let tables = read_tables(&[
        hybrid("HybridLife", "prefix", LIFE),
        hybrid("HybridMana", "prefix", MANA),
    ]);
    let (life, shield, mana) = (
        "+15 to maximum Life",
        "25% increased Energy Shield",
        "+15 to maximum Mana",
    );

    // Two readings as good: the order of the lines doesn't choose one.
    let (read, _) = read_in(&tables, 60, &[life, shield, mana], (3, 3));
    assert_eq!(read, []);
    let (read, _) = read_in(&tables, 60, &[mana, shield, life], (3, 3));
    assert_eq!(read, []);
    // The energy shield first, the lines of one hybrid are apart, which no candidate reads:
    // the other one is shown as sure (an entry of the backlog).
    let (read, _) = read_in(&tables, 60, &[shield, life, mana], (3, 3));
    assert_eq!(read, [(2, vec![(Prefix, 1)])]);
    // The hybrid with the mana a suffix: a rarity that rolls as both keeps both affixes,
    // each reading leaving it another one.
    let tables = read_tables(&[
        hybrid("HybridLife", "prefix", LIFE),
        hybrid("HybridMana", "suffix", MANA),
        table_json("RarityPrefix", "prefix", &[RARITY], &[(1, &[(10, 20)])]),
        table_json("RaritySuffix", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
    ]);
    let rarity = "15% increased Rarity of Items found";
    let both = [(1, vec![(Prefix, 1), (Suffix, 1)])];
    let (read, _) = read_in(&tables, 60, &[life, shield, mana, rarity], (1, 1));
    assert_eq!(read, both);
    let (read, _) = read_in(&tables, 60, &[mana, shield, life, rarity], (1, 1));
    assert_eq!(read, both);
}

#[test]
fn a_best_reading_out_of_the_room_keeps_only_the_tiers_every_reading_in_the_room_gives() {
    use Affix::Prefix;
    let prefixes = [
        table_json("LifePrefix", "prefix", &[LIFE], &[(1, &[(40, 60)])]),
        table_json("ManaPrefix", "prefix", &[MANA], &[(1, &[(25, 35)])]),
        table_json("RarityPrefix", "prefix", &[RARITY], &[(1, &[(10, 20)])]),
    ];
    let with = |suffixes: &[String]| read_tables(&[prefixes.as_slice(), suffixes].concat());
    let (life, mana, rarity) = (
        "+50 to maximum Life",
        "+30 to maximum Mana",
        "15% increased Rarity of Items found",
    );
    let pure = (1, vec![(Prefix, 1)]);

    // A hybrid suffix and a pure suffix of each stat add up to the life and to the mana.
    let adding_up = [
        table_json(
            "HybridSuffix",
            "suffix",
            &[LIFE, MANA],
            &[(1, &[(20, 29), (10, 19)])],
        ),
        table_json("LifeSuffix", "suffix", &[LIFE], &[(1, &[(20, 29)])]),
        table_json("ManaSuffix", "suffix", &[MANA], &[(1, &[(10, 19)])]),
    ];
    let tables = with(&adding_up);
    let lines = [life, mana].map(|text| modifier(None, &[text]));
    // Room for both prefixes: three suffixes for two prefixes, which a modifier holding its
    // line alone rules out.
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (2, 3));
    assert_eq!(read, [pure.clone(), pure.clone()]);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestMet));
    // Room for one prefix only: the two prefixes can't be, and the three suffixes, the only
    // reading in the room, read neither line as they do.
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (1, 3));
    assert_eq!(read, []);
    assert_eq!(
        effort.ending,
        Ending::OtherReadings(Outcome::OnlyAnotherFits)
    );
    // No room for three suffixes either: no reading fits, and the lines keep their tiers.
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (1, 2));
    assert_eq!(read, [pure.clone(), pure.clone()]);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::NoneFits));
    // A rarity every reading in the room reads as the prefix keeps its tier.
    let lines = [life, mana, rarity].map(|text| modifier(None, &[text]));
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (2, 3));
    assert_eq!(read, std::slice::from_ref(&pure));
    assert_eq!(
        effort.ending,
        Ending::OtherReadings(Outcome::OnlyAnotherFits)
    );
    // A rarity that two tiers of its prefix hold takes the worst one while the best reading
    // fits the room, a reading a tier apart being no doubt.
    let mut two_tiers = prefixes.clone();
    two_tiers[2] = table_json(
        "RarityPrefix",
        "prefix",
        &[RARITY],
        &[(1, &[(10, 16)]), (1, &[(14, 20)])],
    );
    let tables = read_tables(&[two_tiers.as_slice(), &adding_up].concat());
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (3, 3));
    assert_eq!(read, [pure.clone(), pure.clone(), (1, vec![(Prefix, 2)])]);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestMet));
    // Out of the room, it has none: of the readings in the room, one reads it in the worst
    // tier, as the best reading does, and one in the other, so that they don't all give it.
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (2, 3));
    assert_eq!(read, []);
    assert_eq!(
        effort.ending,
        Ending::OtherReadings(Outcome::OnlyAnotherFits)
    );

    // Three pure suffixes of each stat: two readings in the room, each with one of the
    // prefixes and the suffixes of the other stat. Each prefix has a reading that gives
    // it, none gives both.
    let suffix =
        |kind: &str, template, range| table_json(kind, "suffix", &[template], &[(1, &[range])]);
    let tables = with(&[
        suffix("LifeA", LIFE, (15, 18)),
        suffix("LifeB", LIFE, (15, 18)),
        suffix("LifeC", LIFE, (15, 18)),
        suffix("ManaA", MANA, (9, 11)),
        suffix("ManaB", MANA, (9, 11)),
        suffix("ManaC", MANA, (9, 11)),
    ]);
    let lines = [life, mana].map(|text| modifier(None, &[text]));
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines, (1, 3));
    assert_eq!(read, []);
    assert_eq!(
        effort.ending,
        Ending::OtherReadings(Outcome::OnlyAnotherFits)
    );
}

#[test]
fn a_best_reading_the_other_readings_dont_meet_leaves_every_line_without_tier() {
    let tables = read_tables(&[
        table_json("Life", "prefix", &[LIFE], &[(1, &[(21, 30)])]),
        table_json("Rarity", "suffix", &[RARITY], &[(1, &[(10, 20)])]),
    ]);
    // Two lines of one stat written apart, one fractured: the best reading takes each for
    // the pure life, two modifiers of one type, which the game can't roll and the other
    // readings don't make.
    let lines = [
        Modifier {
            origin: Some(ModifierOrigin::Fractured),
            ..modifier(None, &["+25 to maximum Life"])
        },
        modifier(None, &["+26 to maximum Life"]),
        modifier(None, &["15% increased Rarity of Items found"]),
    ];

    // The rarity too, which has nothing to do with it (an entry of the backlog).
    let ((read, split), effort) = read_modifiers(&tables, 60, &lines, (3, 3));
    assert_eq!(read, []);
    assert_eq!(split, []);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestNotMet));

    // Other readings met, a hybrid with either life, none of them the best one: the same.
    let lines = [
        "25% increased Energy Shield",
        "+25 to maximum Life",
        "+26 to maximum Life",
    ]
    .map(|text| modifier(None, &[text]));
    let ((read, _), effort) = read_modifiers(&pure_and_hybrid_tables(), 60, &lines, (3, 3));
    assert_eq!(read, []);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestNotMet));
}

#[test]
fn a_search_whose_readings_stop_at_their_bound_reads_no_line() {
    let tables = pure_and_hybrid_tables();
    let lines = |count: usize| -> Vec<Modifier> {
        ["25% increased Energy Shield", "+25 to maximum Life"]
            .iter()
            .cycle()
            .take(count)
            .map(|text| modifier(None, &[text]))
            .collect()
    };

    // Two lines, alone or their hybrid, the only reading that fits one prefix.
    let ((read, _), effort) = read_modifiers(&tables, 60, &lines(2), (1, 0));
    assert_eq!(effort.readings, 2);
    assert_eq!(effort.ending, Ending::OtherReadings(Outcome::BestMet));
    assert_eq!(read, [(2, vec![(Affix::Prefix, 1)])]);
    // Each line a hybrid with the one before or after: more readings than the bound, and no
    // line read, the best of those looked at being maybe not the best of all (twelve hybrids
    // fit twelve prefixes).
    let ((read, split), effort) = read_modifiers(&tables, 60, &lines(24), (12, 0));
    assert_eq!(effort.readings, MAX_READINGS);
    assert_eq!(effort.ending, Ending::ReadingsCut);
    assert_eq!(read, []);
    assert_eq!(split, []);
}
