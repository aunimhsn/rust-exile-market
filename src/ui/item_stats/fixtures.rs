//! What the tests of the stats share: the items they parse, the rows they decide, and the
//! texts these rows are compared with.

use super::Stats;
use super::model::{
    Group, Limit, ModifierTier, Row, Stat, StatFilter, StatRoll, StatText, TierRow, WordTip,
};
use super::rolls::ranges_text;
use crate::filters::{Filters, Half};
use crate::item::{Affix, CopiedItem, Item, PartTier, Range, Rarity, StatLine};
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::trade::stat_filters::LineStats;
use crate::{format, game_data};

pub(super) fn parse(text: &str) -> Item {
    Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
}

/// The lines of a stat written on several lines, a ` / ` between them.
fn lines_text(lines: &[StatLine]) -> String {
    let texts: Vec<_> = lines.iter().map(|line| line.text.as_str()).collect();
    texts.join(" / ")
}

/// The rows of `item`, its lines read in English.
pub(super) fn rows(item: &Item) -> Vec<Row<'_>> {
    rows_of(item, base_type(item))
}

/// The rows of `item` of the base type `base_type`, its lines read in English: the base type
/// of a magic item is found among the names of the trade site.
pub(super) fn rows_of<'a>(item: &'a Item, base_type: Option<&'a str>) -> Vec<Row<'a>> {
    let lines = CopiedLines::new(
        item.language(),
        item.modifiers(),
        &LineStats::default(),
        None,
    );
    let stats = LineStats::new(item, item.modifiers(), None);
    let tiers = TierRolls::new(
        item.language(),
        item.modifiers(),
        base_type,
        item.item_level(),
    );
    super::rows(item, &lines, &stats, &tiers, base_type)
}

/// The base type the search of `item` looks for, among its filters: none for a unique.
pub(super) fn base_type(item: &Item) -> Option<&str> {
    (item.rarity() != Rarity::Unique).then(|| item.base_type())
}

/// The rows of `item` as texts, as [`row_texts`] writes them.
pub(super) fn texts(item: &Item) -> Vec<String> {
    row_texts(rows(item))
}

/// The rows as texts: a title (`Prefixes 2/3`), a note, or a stat,
/// first `■ ` for a checked filter, `□ ` for an unchecked one or `⊘ ` for a line the trade
/// site can't search, then its minimum or its maximum and its roll (`■ +55 to maximum Life
/// ≥55 [40–60 0.75]`, `■ 18% reduced Movement Speed ≤18`, a line of a prefix or suffix
/// with its bar over every tier and, under its last line, its tier, its ranges and its
/// origin: `■ +86 to maximum Life ≥77 [0.37 T7 85–99]`, `[0.80 T2 71–79 Desecrated]`; the
/// tiers of a line that can be a prefix or a suffix, each row after a ` / `, its origin a
/// row of its own: `[T1 16–19 AffixPrefix / T1 15–18 AffixSuffix / Desecrated]`; the word
/// after the ranges followed by its tooltip, and two neighbouring tiers `T1/T2 155–179`); last,
/// the tag of the radius of a line of a Time-Lost jewel, `<Small>`.
pub(super) fn row_texts<'a>(rows: impl IntoIterator<Item = Row<'a>>) -> Vec<String> {
    // `2/3`, or the count alone: `6`.
    let fill = |filled: u32, max: Option<u32>| match max {
        Some(max) => format!("{filled}/{max}"),
        None => filled.to_string(),
    };
    rows.into_iter()
        .map(|row| match row {
            Row::Group(Group::Prefixes { filled, max }) => {
                format!("Prefixes {}", fill(filled, max))
            }
            Row::Group(Group::Suffixes { filled, max }) => {
                format!("Suffixes {}", fill(filled, max))
            }
            Row::Group(group) => format!("{group:?}"),
            Row::UnrevealedNote => "Unrevealed note".to_owned(),
            Row::TradeCopyNote => "Trade copy note".to_owned(),
            Row::UniqueRollsNote => "Unique rolls note".to_owned(),
            Row::Stat(Stat {
                text,
                filter,
                is_unsearchable,
                roll,
                ..
            }) => {
                let check = match filter {
                    _ if is_unsearchable => "⊘ ",
                    Some(StatFilter { checked: true, .. }) => "■ ",
                    Some(StatFilter { checked: false, .. }) => "□ ",
                    None => "",
                };
                let limit = filter
                    .and_then(|filter| filter.limit)
                    .map(|limit| {
                        let sign = match limit.half {
                            Half::Minimum => '≥',
                            Half::Maximum => '≤',
                        };
                        format!(" {sign}{}", format::number(limit.value))
                    })
                    .unwrap_or_default();
                let roll = match roll {
                    None => String::new(),
                    Some(StatRoll::Bar {
                        share,
                        ranges,
                        origin,
                    }) => {
                        let origin = origin
                            .map(|origin| format!(" {origin:?}"))
                            .unwrap_or_default();
                        format!(" [{} {share:.2}{origin}]", ranges_text(&ranges))
                    }
                    Some(StatRoll::Affix {
                        share,
                        tier,
                        origin,
                    }) => {
                        let share = share.map(|share| format!("{share:.2}"));
                        let tier = tier.as_deref().map(tier_row_text);
                        let origin = origin.map(|origin| format!("{origin:?}"));
                        let parts: Vec<String> =
                            [share, tier, origin].into_iter().flatten().collect();
                        format!(" [{}]", parts.join(" "))
                    }
                    Some(StatRoll::Tiers { rows, origin }) => {
                        // The origin on a line of its own, after the tiers.
                        let rows: Vec<String> = rows
                            .iter()
                            .map(tier_row_text)
                            .chain(origin.map(|origin| format!("{origin:?}")))
                            .collect();
                        format!(" [{}]", rows.join(" / "))
                    }
                    Some(StatRoll::WaystoneTier(tier)) => format!(" [Waystone T{tier}]"),
                };
                let radius = text
                    .radius_skills()
                    .map(|skills| format!(" <{skills:?}>"))
                    .unwrap_or_default();
                let text = match text {
                    StatText::Line(line) | StatText::RadiusLine { line, .. } => line.to_owned(),
                    StatText::Lines(lines) => lines_text(lines),
                    StatText::BaseType { base_type, .. } => format!("Base: {base_type}"),
                    StatText::Property(property, value) => {
                        format!("{property:?}: {}", format::number(value))
                    }
                    StatText::CatalystQuality(kind, value) => {
                        let kind = kind
                            .catalyst
                            .map_or_else(|| kind.words.clone(), |catalyst| format!("{catalyst:?}"));
                        format!("{kind} quality: {}", format::number(value))
                    }
                    StatText::Pseudo(pseudo, value) => {
                        format!("{pseudo:?}: {}", format::number(value))
                    }
                    StatText::Unrevealed(affix) => format!("Unrevealed {affix:?}"),
                };
                format!("{check}{text}{limit}{roll}{radius}")
            }
        })
        .collect()
}

/// A tier under a line as [`row_texts`] writes it: `T1/T2 155–179`, then its word and its
/// tooltip, `AffixSuffix AddedUpWithPrefix`, `Hybrid (with +195 to Accuracy Rating)`, `Hybrid
/// (with 75–79 of 258% increased Physical Damage)`; the lord before the word, `Amanamu
/// AffixPrefix`, and a tier written in the style of its word between braces, `{T2 AffixSuffix}`.
fn tier_row_text(row: &TierRow) -> String {
    let tier = tier_text(&row.tier);
    let tier = match row.better {
        Some(better) => format!("T{better}/{tier}"),
        None => tier,
    };
    let word = row.word.map(|word| match row.lord {
        Some(lord) => format!("{} {word:?}", lord.name()),
        None => format!("{word:?}"),
    });
    let tip = row.tip.as_ref().map(|tip| match tip {
        WordTip::Text(message) => format!("{message:?}"),
        WordTip::HybridLines(lines) => format!("(with {})", lines.join(" and ")),
        WordTip::HybridPart { ranges, line } => {
            format!("(with {} of {line})", ranges_text(ranges))
        }
    });
    let currency = row.currency.map(|currency| format!("{currency:?}"));
    let text = [Some(tier), word, tip, currency]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    // A tier written in the style of its word, between braces.
    if row.is_tier_in_word {
        format!("{{{text}}}")
    } else {
        text
    }
}

/// A tier as [`row_texts`] writes it: `T1 15–18`, `15–18` for a modifier of one tier only.
fn tier_text(tier: &ModifierTier) -> String {
    let number = tier.number.map(|number| format!("T{number}"));
    [number.unwrap_or_default(), ranges_text(&tier.ranges)]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `item`, copied from the in-game trade, read again with the tiers found from its values,
/// as the session does.
pub(super) fn read_again(mut item: Item) -> Item {
    let stats = LineStats::new(&item, item.modifiers(), None);
    let trade_item = crate::game_data::TradeItem {
        base: crate::game_data::ItemBase {
            language: item.language(),
            base_type: item.base_type(),
            item_level: item.item_level(),
        },
        limits: stats.affix_limits(&item),
        is_sanctified: item.has_state(crate::item::ItemState::Sanctified),
    };
    let modifiers = crate::game_data::with_trade_tiers(&trade_item, item.modifiers());
    item.set_modifiers(modifiers);
    item
}

/// `item`, a unique copied from the in-game trade, with the rolls poe2scout writes for Soul
/// Tether.
pub(super) fn with_soul_tether_rolls(mut item: Item) -> Item {
    let written = |modifiers: &[&str]| -> Vec<String> {
        modifiers
            .iter()
            .map(|&modifier| modifier.to_owned())
            .collect()
    };
    let modifiers = game_data::with_unique_rolls(
        item.language(),
        item.modifiers(),
        &written(&[
            "(15-20)% increased Charm Effect Duration",
            "Has (1-3) Charm Slots",
        ]),
        &written(&[
            "+(40-60) to maximum Energy Shield",
            "You lose (5-5)% of maximum Energy Shield per second",
            "Excess Life Recovery from Leech is applied to Energy Shield",
        ]),
        None,
    );
    item.set_modifiers(modifiers);
    item
}

/// The limit of the filter of `row`.
pub(super) fn limit(row: &Row<'_>) -> Limit {
    match row {
        Row::Stat(Stat {
            filter: Some(StatFilter {
                limit: Some(limit), ..
            }),
            ..
        }) => *limit,
        row => panic!("{row:?}"),
    }
}

/// The rows of `item` from its first group `title` on.
pub(super) fn texts_from(item: &Item, title: &str) -> Vec<String> {
    let texts = texts(item);
    let start = texts
        .iter()
        .position(|text| text.starts_with(title))
        .unwrap_or_else(|| panic!("{title}: {texts:?}"));
    texts[start..].to_vec()
}

/// A part of a split line of `affix` in tier 1, over 1–2.
pub(super) fn part(affix: Affix) -> PartTier {
    PartTier {
        affix,
        tiers: vec![1],
        hides_number: false,
        ranges: vec![Range { min: 1.0, max: 2.0 }],
        currency: None,
    }
}

/// The rows of `texts` from the first group of prefixes, suffixes or explicit modifiers.
pub(super) fn explicit_rows(texts: Vec<String>) -> Vec<String> {
    texts
        .into_iter()
        .skip_while(|text| {
            !["Prefixes", "Suffixes", "Explicit"]
                .iter()
                .any(|group| text.starts_with(group))
        })
        .collect()
}

/// The checks of the filters of `item` by default, as the item view decides them.
pub(super) fn default_checks(item: &Item) -> crate::ui::StatChecks {
    let base = base_type(item);
    let lines = CopiedLines::new(
        item.language(),
        item.modifiers(),
        &LineStats::default(),
        None,
    );
    let line_stats = LineStats::new(item, item.modifiers(), None);
    let tiers = TierRolls::new(item.language(), item.modifiers(), base, item.item_level());
    let stats = Stats::new(item, &lines, &line_stats, &tiers, base);
    let all = stats.filters(&Filters::default());
    crate::ui::StatChecks::of(&line_stats.search_filters(&all), &line_stats, &all)
}

/// A rare item of `base`, of the item `class`, as the in-game trade copies it, without
/// header nor range: `top`, its properties, then its modifier `lines`.
pub(super) fn rare_item(base: &str, class: &str, top: &str, lines: &str) -> Item {
    parse(&format!(
        "Item Class: {class}\nRarity: Rare\nGale Coil\n{base}\n--------\n{top}\
         Item Level: 80\n--------\n{lines}"
    ))
}
