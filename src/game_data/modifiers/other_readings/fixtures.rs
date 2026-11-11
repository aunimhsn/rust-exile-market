//! What the tests of the other readings share: the lines, the placements and the ground they
//! are written with.

use crate::game_data::modifiers::best_reading::Room;
use crate::game_data::modifiers::copied::CopiedLine;
use crate::game_data::modifiers::other_readings::comparison::Ground;
use crate::game_data::modifiers::other_readings::placements::Placement;
use crate::game_data::modifiers::other_readings::{Line, MAX_STEPS, Shown};
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
        copied: CopiedLine {
            templates: vec![template],
            values: Vec::new(),
            written: None,
        },
        values,
        read_by_best: true,
    }
}

/// A placement of `kind` and `affix` on `lines`, adding `ranges` to the values.
pub(super) fn placement(
    kind: &'static str,
    affix: Affix,
    lines: &[usize],
    ranges: &[(usize, Range)],
    fractured: bool,
) -> Placement<'static> {
    Placement {
        kind,
        fractured,
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
        rising: vec![true; values.len()],
        falling: vec![false; values.len()],
        best: Vec::new(),
        best_candidates: Vec::new(),
        max_steps: MAX_STEPS,
    }
}
