//! The damage and the speed of a weapon, read from its properties, for its damage per second.
//!
//! The properties already count the quality and the modifiers of the weapon (`augmented`):
//! its DPS is the average of each damage range times its attacks per second.

use super::USUAL_QUALITY;

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

/// What lines of a weapon add to the damage and the speed of its base: their increased
/// physical damage in percent, their added physical and elemental damage (the average of a
/// hit), their increased attack speed in percent.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct WeaponLines {
    pub increased_physical: f64,
    pub added_physical: f64,
    pub added_elemental: f64,
    pub attack_speed: f64,
}

/// The damage per second the base of a weapon reaches, of the kinds a filter searches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BaseDps {
    pub physical: f64,
    /// Fire, cold and lightning together.
    pub elemental: f64,
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
        let per_second = |kind| self.hit(kind) * self.attacks_per_second;
        Dps {
            physical: per_second(DamageKind::Physical),
            elemental: per_second(DamageKind::Elemental),
            chaos: per_second(DamageKind::Chaos),
        }
    }

    /// The average damage of a hit of `kind`.
    fn hit(&self, kind: DamageKind) -> f64 {
        self.damage
            .iter()
            .filter(|range| range.kind == kind)
            .map(|range| range.average())
            .sum()
    }

    /// The DPS its base reaches with the lines it `keeps` (its runes, which any weapon of the
    /// base may have), from the weapon at `quality` (in percent) whose `lines` (its modifiers
    /// and runes) the properties count; the physical one at the usual quality, or at its own
    /// when higher. The properties count the added damage first, then the increased physical
    /// damage of the lines, then the quality apart, which multiplies the physical damage, and
    /// the attack speed apart: the only reading that gives the two copies of one base of the
    /// test items, with other lines, the same whole damage (`docs/game-data.md`).
    #[must_use]
    pub fn base_dps(&self, quality: f64, lines: WeaponLines, keeps: WeaponLines) -> BaseDps {
        let percent = |value: f64| 1.0 + value / 100.0;
        let physical = self.hit(DamageKind::Physical)
            / (percent(lines.increased_physical) * percent(quality))
            - lines.added_physical;
        let elemental = self.hit(DamageKind::Elemental) - lines.added_elemental;
        let speed =
            self.attacks_per_second / percent(lines.attack_speed) * percent(keeps.attack_speed);
        let usual = f64::from(USUAL_QUALITY);
        BaseDps {
            physical: (physical + keeps.added_physical).max(0.0)
                * percent(keeps.increased_physical)
                * percent(quality.max(usual))
                * speed,
            elemental: (elemental + keeps.added_elemental).max(0.0) * speed,
        }
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
    fn the_dps_of_the_base_leaves_out_the_lines_and_takes_the_usual_quality() {
        let none = WeaponLines::default();
        let close = |found: f64, expected: f64| (found - expected).abs() < 1e-9;
        // 10-20, a hit of 15, with 20 % increased physical damage and no quality: 12.5 from
        // the base, 15 at 20 % quality.
        let weapon = Weapon {
            damage: vec![range(DamageKind::Physical, 10.0, 20.0)],
            attacks_per_second: 1.0,
        };
        let increased = WeaponLines {
            increased_physical: 20.0,
            ..none
        };
        assert!(close(weapon.base_dps(0.0, increased, none).physical, 15.0));
        // Its own quality, higher, multiplying apart: 15 / (1.2 × 1.3) × 1.3 = 12.5.
        assert!(close(weapon.base_dps(30.0, increased, none).physical, 12.5));
        // Added damage and attack speed: (40 - 10) × 1.2 × (1.2 / 1.2) = 36; no elemental
        // damage of the base under the 30 a modifier adds.
        let weapon = Weapon {
            damage: vec![
                range(DamageKind::Physical, 30.0, 50.0),
                range(DamageKind::Elemental, 1.0, 59.0),
            ],
            attacks_per_second: 1.2,
        };
        let lines = WeaponLines {
            added_physical: 10.0,
            added_elemental: 30.0,
            attack_speed: 20.0,
            ..none
        };
        let base = weapon.base_dps(0.0, lines, none);
        assert!(close(base.physical, 36.0), "{base:?}");
        assert!(close(base.elemental, 0.0), "{base:?}");
        // Runes kept, which any weapon of the base may have: their 10 added and their 20 %
        // of attack speed stay, (30 + 10) × 1.2 × 1.2 = 57.6, and their 30 of elemental
        // damage, 30 × 1.2 = 36; the quality raises the physical damage only.
        let base = weapon.base_dps(0.0, lines, lines);
        assert!(close(base.physical, 57.6), "{base:?}");
        assert!(close(base.elemental, 36.0), "{base:?}");
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
