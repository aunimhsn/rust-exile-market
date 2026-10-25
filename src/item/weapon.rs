//! The damage and the speed of a weapon, read from its properties, for its damage per second.
//!
//! The properties already count the quality and the modifiers of the weapon (`augmented`):
//! its DPS is the average of each damage range times its attacks per second.

/// The kind of damage of a range, told by the marker after it in every language, e.g.
/// `1-50 (lightning)`; a range without marker is physical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageKind {
    Physical,
    Elemental,
    Chaos,
}

impl DamageKind {
    /// The kind of the marker `marker`, without its parentheses.
    fn of_marker(marker: &str) -> Option<Self> {
        match marker {
            "fire" | "cold" | "lightning" => Some(Self::Elemental),
            "chaos" => Some(Self::Chaos),
            _ => None,
        }
    }
}

/// A damage range of a weapon, e.g. `74-231`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct DamageRange {
    pub kind: DamageKind,
    pub min: f64,
    pub max: f64,
}

impl DamageRange {
    fn average(self) -> f64 {
        f64::midpoint(self.min, self.max)
    }
}

/// The damage per second of a weapon, by kind.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dps {
    pub physical: f64,
    /// Fire, cold and lightning together.
    pub elemental: f64,
    pub chaos: f64,
}

/// The damage and the speed of a weapon.
#[derive(Debug, Clone, PartialEq)]
pub struct Weapon {
    pub(super) damage: Vec<DamageRange>,
    pub(super) attacks_per_second: f64,
}

impl Weapon {
    /// The damage per second of each kind.
    #[must_use]
    pub fn dps(&self) -> Dps {
        let mut dps = Dps::default();
        for range in &self.damage {
            let per_second = range.average() * self.attacks_per_second;
            match range.kind {
                DamageKind::Physical => dps.physical += per_second,
                DamageKind::Elemental => dps.elemental += per_second,
                DamageKind::Chaos => dps.chaos += per_second,
            }
        }
        dps
    }
}

/// The damage ranges of the value of a damage property, e.g. `74-231` or
/// `10-20 (fire), 1-30 (lightning)`; `None` when a part can't be read.
pub(super) fn damage_ranges(value: &str) -> Option<Vec<DamageRange>> {
    value
        .split(", ")
        .map(|part| {
            let (range, kind) = match part.strip_suffix(')').and_then(|p| p.rsplit_once(" (")) {
                Some((range, marker)) => (range, DamageKind::of_marker(marker)?),
                None => (part, DamageKind::Physical),
            };
            let (min, max) = range.split_once('-')?;
            Some(DamageRange {
                kind,
                min: min.trim().parse().ok()?,
                max: max.trim().parse().ok()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn range(kind: DamageKind, min: f64, max: f64) -> DamageRange {
        DamageRange { kind, min, max }
    }

    #[test]
    fn reads_the_ranges_and_their_kinds() {
        assert_eq!(
            damage_ranges("381-705"),
            Some(vec![range(DamageKind::Physical, 381.0, 705.0)])
        );
        assert_eq!(
            damage_ranges("10-20 (fire), 1-30 (lightning)"),
            Some(vec![
                range(DamageKind::Elemental, 10.0, 20.0),
                range(DamageKind::Elemental, 1.0, 30.0)
            ])
        );
        assert_eq!(
            damage_ranges("12-24 (chaos)"),
            Some(vec![range(DamageKind::Chaos, 12.0, 24.0)])
        );
        assert_eq!(damage_ranges("5 (unknown)"), None);
        assert_eq!(damage_ranges("high"), None);
    }

    #[test]
    fn the_dps_is_the_average_damage_times_the_attacks_per_second() {
        let weapon = Weapon {
            damage: vec![
                range(DamageKind::Physical, 74.0, 231.0),
                range(DamageKind::Elemental, 10.0, 273.0),
                range(DamageKind::Chaos, 10.0, 30.0),
            ],
            attacks_per_second: 2.0,
        };

        assert_eq!(
            weapon.dps(),
            Dps {
                physical: 305.0,
                elemental: 283.0,
                chaos: 40.0
            }
        );
    }
}
