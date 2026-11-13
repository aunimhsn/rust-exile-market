//! What the tests of the other readings share: the lines, the placements and the ground of
//! the walk they are written with.

use crate::game_data::modifiers::best_reading::Room;
use crate::game_data::modifiers::doubts::comparison::BestShown;
use crate::game_data::modifiers::doubts::placements::{Directions, Placement};
use crate::game_data::modifiers::doubts::walk::Ground;
use crate::game_data::modifiers::doubts::{Line, MAX_STEPS, Shown};
use crate::game_data::modifiers::fixtures::copied_line;
use crate::item::{Affix, ModifierOrigin, Range};

/// A line of the modifier `modifier` of the item, of `template`, its values at `values` among
/// those of every line.
pub(super) fn line(
    modifier: usize,
    template: &'static str,
    values: std::ops::Range<usize>,
    origin: Option<ModifierOrigin>,
) -> Line<'static> {
    Line {
        modifier,
        origin,
        copied: copied_line(template, &[]),
        values,
        read_by_best: true,
    }
}

/// A placement of `kind` and `affix` on `lines`, adding `ranges` to the values, of a modifier
/// that isn't fractured.
pub(super) fn placement(
    kind: &'static str,
    affix: Affix,
    lines: &[usize],
    ranges: &[(usize, Range)],
) -> Placement<'static> {
    Placement {
        kind,
        fractured: false,
        shown: Shown {
            affix,
            tier: 1,
            modifiers: lines.to_vec(),
            ranges: ranges.iter().map(|&(_, range)| range).collect(),
        },
        lines: lines.to_vec(),
        ranges: ranges.to_vec(),
    }
}

/// `placement`, of a fractured modifier.
pub(super) fn fractured(placement: Placement<'static>) -> Placement<'static> {
    Placement {
        fractured: true,
        ..placement
    }
}

/// What the search reads `lines` and `placements` from, with `values`: room for three
/// prefixes and suffixes, no best reading.
pub(super) fn ground<'s>(
    lines: &'s [Line<'static>],
    values: &'s [f64],
    placements: &'s [Placement<'static>],
) -> Ground<'s, 'static> {
    Ground {
        lines,
        values,
        placements,
        starting: vec![Vec::new(); lines.len()],
        room: Room {
            prefixes: 3,
            suffixes: 3,
        },
        fractured: 1,
        directions: Directions {
            rising: vec![true; values.len()],
            falling: vec![false; values.len()],
        },
        best: BestShown {
            lines: Vec::new(),
            candidates: Vec::new(),
        },
        max_steps: MAX_STEPS,
    }
}
