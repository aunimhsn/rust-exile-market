//! The tiers under a line that adds up two modifiers, the prefix first, and under the other
//! lines of a hybrid whose part such a line adds up.

use super::model::{TierRow, WordTip, affix_word};
use super::rolls::{copied_ranges, magnitudes};
use crate::i18n::Message;
use crate::item::{Affix, Modifier, PartTier, Range, StatLine};

/// The tiers of the parts `whole` and `added` of a line that adds up two modifiers, one under
/// the other: the prefix first, then, of two parts of one affix, the pure one (`added`) before
/// the hybrid; "Prefix" and "Suffix" after the ranges of a prefix and a suffix, the tooltip of
/// "Suffix" saying they are added up, which a line that can be either doesn't have. A hybrid
/// `whole`, of the `others` lines, says "Hybrid" instead ([`hybrid_word`]), its tooltip naming
/// them.
pub(super) fn split_tier_rows<'a>(
    whole: &PartTier,
    added: &PartTier,
    others: &[&'a str],
) -> Vec<TierRow<'a>> {
    let mut parts = [(added, false), (whole, true)];
    parts.sort_by_key(|&(part, is_whole)| (part.affix == Affix::Suffix, is_whole));
    let both = whole.affix != added.affix;
    parts
        .iter()
        .filter_map(|&(part, is_whole)| {
            let (word, tip) = if is_whole && !others.is_empty() {
                (
                    Some(hybrid_word(whole, added)),
                    Some(WordTip::HybridLines(others.to_vec())),
                )
            } else {
                (
                    both.then(|| affix_word(part.affix)),
                    (both && part.affix == Affix::Suffix)
                        .then_some(WordTip::Text(Message::AddedUpWithPrefix)),
                )
            };
            TierRow::part(part, word, tip)
        })
        .collect()
}

/// The word after the ranges of the hybrid `whole` whose part a line adds up with `added`,
/// on that line and under its other lines: "Hybrid", with its affix when `added` is of the
/// other affix ("Hybrid prefix"), the line then in Explicit.
fn hybrid_word(whole: &PartTier, added: &PartTier) -> Message {
    match whole.affix {
        _ if whole.affix == added.affix => Message::Hybrid,
        Affix::Prefix => Message::HybridPrefix,
        Affix::Suffix => Message::HybridSuffix,
    }
}

/// The lines of `modifier` other than its line `summed`, with their indexes: the other lines
/// of the hybrid whose part that line adds up.
pub(super) fn other_lines(
    modifier: &Modifier,
    summed: usize,
) -> impl DoubleEndedIterator<Item = (usize, &StatLine)> {
    modifier
        .lines
        .iter()
        .enumerate()
        .filter(move |&(index, _)| index != summed)
}

/// The tier of the hybrid `whole` of `modifier`, whose line `summed` adds up its part with
/// `added`, under the last of its other lines: the ranges of these lines over its tiers,
/// "Hybrid" after them ([`hybrid_word`]), its tooltip naming that part and the line.
pub(super) fn hybrid_tier_row<'a>(
    modifier: &'a Modifier,
    summed: usize,
    whole: &PartTier,
    added: &PartTier,
) -> Option<TierRow<'a>> {
    // As under any modifier, none for fixed values.
    let ranges = Range::unless_fixed(
        other_lines(modifier, summed).flat_map(|(_, line)| magnitudes(line, &copied_ranges(line))),
    );
    let tip = WordTip::HybridPart {
        ranges: whole.ranges.clone(),
        line: &modifier.lines.get(summed)?.text,
    };
    TierRow::tiers(whole, ranges, Some(hybrid_word(whole, added)), Some(tip))
}

#[cfg(test)]
mod tests;
