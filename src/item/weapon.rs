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

/// What the lines of a weapon add to the damage and the speed of its base: its increased
/// physical damage in percent, its added physical damage (the average of a hit), its
/// increased attack speed in percent.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeaponLines {
    pub increased: f64,
    pub added: f64,
    pub attack_speed: f64,
}

/// The quality a base usually reaches, which every weapon of it may have: its physical DPS
/// there is what its base alone reaches.
const USUAL_QUALITY: f64 = 20.0;

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

    /// The physical DPS its base alone reaches, without the `lines` of the weapon, at the
    /// usual quality or at its own `quality` (in percent) when higher: the properties count the
    /// quality and the increased physical damage together, the added damage before them, and
    /// the attack speed apart.
    #[must_use]
    pub fn base_physical_dps(&self, quality: f64, lines: WeaponLines) -> f64 {
        let hit: f64 = self
            .damage
            .iter()
            .filter(|range| range.kind == DamageKind::Physical)
            .map(|range| range.average())
            .sum();
        let base = hit / (1.0 + (lines.increased + quality) / 100.0) - lines.added;
        let speed = self.attacks_per_second / (1.0 + lines.attack_speed / 100.0);
        base.max(0.0) * (1.0 + quality.max(USUAL_QUALITY) / 100.0) * speed
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

    #[test]
    fn the_physical_dps_of_the_base_leaves_out_the_lines_and_takes_the_usual_quality() {
        // 10-20, a hit of 15, with 20 % increased physical damage and no quality: 12.5 from
        // the base.
        let weapon = Weapon {
            damage: vec![range(DamageKind::Physical, 10.0, 20.0)],
            attacks_per_second: 1.0,
        };
        let lines = WeaponLines {
            increased: 20.0,
            ..WeaponLines::default()
        };
        // At 20 % quality: 15, as much as the weapon.
        assert!((weapon.base_physical_dps(0.0, lines) - 15.0).abs() < 1e-9);
        // Its own quality, higher, counted in the properties: 15 / 1.5 = 10, at 30 %: 13.
        let weapon = Weapon {
            damage: vec![range(DamageKind::Physical, 10.0, 20.0)],
            attacks_per_second: 1.0,
        };
        assert!((weapon.base_physical_dps(30.0, lines) - 13.0).abs() < 1e-9);
        // Added damage and attack speed: (40 / 1.0 - 10) * 1.2 * (1.2 / 1.2) = 36.
        let weapon = Weapon {
            damage: vec![
                range(DamageKind::Physical, 30.0, 50.0),
                range(DamageKind::Elemental, 1.0, 59.0),
            ],
            attacks_per_second: 1.2,
        };
        let lines = WeaponLines {
            added: 10.0,
            attack_speed: 20.0,
            ..WeaponLines::default()
        };
        assert!((weapon.base_physical_dps(0.0, lines) - 36.0).abs() < 1e-9);
    }

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
