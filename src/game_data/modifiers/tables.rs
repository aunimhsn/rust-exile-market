//! The modifier tables as the generator writes them, read once from the embedded file: what a
//! table rolls on, what it may hold, and which tables are tiers of one modifier.

use std::sync::LazyLock;

use serde::Deserialize;

use crate::game_data::bases::{BaseSpawn, Domain};
use crate::item::{Affix, Currency, Lord, ModifierOrigin};

/// The tiers of a type of modifier that roll on the same bases, as the generator writes them.
/// A field the application doesn't know makes the line unreadable, here and in the lines and
/// tiers: the generator and the application describe the file each on their side, and a field
/// renamed on one side only would otherwise read as its default (a desecrated table as an
/// ordinary one), without a failing test.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Table {
    /// The modifiers of one type can't be on the same item: its tiers.
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) affix: TableAffix,
    /// The domain of the bases it rolls on: `default`, a tag of every base, rolls on those of
    /// its domain only (the charges of a flask); a desecrated table rolls on the equipment and
    /// the jewels.
    #[serde(default)]
    pub(super) domain: Domain,
    pub(super) lines: Vec<TableLine>,
    /// Each tag and its weight: the first tag of the list a base has tells whether the
    /// modifier rolls on it (above zero) or not.
    pub(super) spawn: Vec<(String, u32)>,
    /// Where the modifier comes from when it isn't an ordinary one.
    #[serde(default)]
    pub(super) source: Option<TableSource>,
    /// The lord of the Abyss whose modifier it is.
    #[serde(default)]
    pub(super) lord: Option<TableLord>,
    pub(super) tiers: Vec<TableTier>,
}

/// Where a modifier that isn't an ordinary one comes from, as the generator writes it: a
/// modifier of its own, whose tiers the game doesn't write.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(super) enum TableSource {
    /// The Genesis tree of the Breach (`Reinforcing`, `Minions have 21(19-22)% increased
    /// maximum Life`).
    Genesis,
    /// An essence (`of the Essence`, `20(20-25)% chance to gain Onslaught on Killing Hits with
    /// this Weapon`).
    Essence,
    /// An alloy of Verisium (`of the Stars`, `11(10-15)% increased Attack Speed while missing
    /// Runic Ward`, crafted).
    Alloy,
}

/// A lord of the Abyss, as the generator writes him.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TableLord {
    Amanamu,
    Kurgal,
    Ulaman,
}

impl From<TableLord> for Lord {
    fn from(lord: TableLord) -> Self {
        match lord {
            TableLord::Amanamu => Self::Amanamu,
            TableLord::Kurgal => Self::Kurgal,
            TableLord::Ulaman => Self::Ulaman,
        }
    }
}

/// The prefix first, as the two pure modifiers of a split line are read.
#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub(super) enum TableAffix {
    Prefix,
    Suffix,
}

#[derive(Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct TableLine {
    /// The English template of the line, as Exiled Exchange 2 writes it.
    pub(super) template: String,
    /// What the value of the stat is divided by to be shown, for a line of one value that can
    /// be written without number: its value is then that of the stat (1000 for `Inherent Rage
    /// loss starts 1 second later`, shown in seconds; no such line rolls on a base on
    /// 2026-10-28, the generator writes the divisor when one does).
    #[serde(default = "one")]
    pub(super) divisor: f64,
}

fn one() -> f64 {
    1.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TableTier {
    pub(super) level: u32,
    /// For each line, the range of each of its values, with the sign of the stat.
    pub(super) ranges: Vec<Vec<[f64; 2]>>,
}

impl From<TableAffix> for Affix {
    fn from(affix: TableAffix) -> Self {
        match affix {
            TableAffix::Prefix => Self::Prefix,
            TableAffix::Suffix => Self::Suffix,
        }
    }
}

impl Table {
    /// Whether the modifier rolls on `base`: a base of its domain, whose first tag of the list
    /// says so.
    pub(super) fn rolls_on(&self, base: &BaseSpawn) -> bool {
        self.domain.rolls_on(base.domain)
            && self
                .spawn
                .iter()
                .find(|(tag, _)| base.tags.contains(tag))
                .is_some_and(|&(_, weight)| weight > 0)
    }

    /// Whether the table may hold a modifier of `origin`, as the game marks it: a desecrated
    /// table only a desecrated modifier, which is never one of a currency (the `20–30` of an
    /// essence on an amulet, before the `15–25` of Amanamu in the file); the table of an alloy
    /// only a marked one, the game marking it crafted (`(crafted)` from the in-game trade,
    /// checked by the owner on 2026-10-30), or fractured since: an unmarked line read in a
    /// hybrid of an alloy (`+1 to Level of all Spell Skills` and the mana of a wand) would put
    /// the ordinary modifiers of every such item in doubt.
    pub(super) fn may_hold(&self, origin: Option<ModifierOrigin>) -> bool {
        match origin {
            Some(ModifierOrigin::Desecrated) => self.currency().is_none(),
            Some(_) => self.domain != Domain::Desecrated,
            None => self.domain != Domain::Desecrated && self.source != Some(TableSource::Alloy),
        }
    }

    /// Whether the game writes the modifier without tier: one of the Genesis tree, of an
    /// essence or of an alloy.
    pub(super) fn tier_unwritten(&self) -> bool {
        match self.source {
            Some(TableSource::Genesis | TableSource::Essence | TableSource::Alloy) => true,
            None => false,
        }
    }

    /// The currency that adds the modifier, when only one does.
    pub(super) fn currency(&self) -> Option<Currency> {
        match self.source {
            Some(TableSource::Essence) => Some(Currency::Essence),
            Some(TableSource::Alloy) => Some(Currency::Alloy),
            Some(TableSource::Genesis) | None => None,
        }
    }

    /// Whether the table holds tiers of the same modifier as `other`, maybe on other bases. A
    /// desecrated table is a modifier of its own, its one tier "the only one": two of them
    /// may roll the same lines on one base, a lord's own (the flask charges of Ulaman,
    /// `10–18`, and of Kurgal, `10–15`, on the belts), or the same range written twice (the
    /// cost efficiency of attacks of Kurgal on the maces), which merged would read as a T1 and
    /// a T2. Nothing tells two such tables apart but their place: `self` and `other` are tables
    /// of one slice (`TABLES`, or the tables of a test), and a desecrated table is the same
    /// modifier as itself only. A minion modifier of the Genesis tree merges with no ordinary
    /// one: `of the Despot` of the tree (`MinionGemLevelBelt2`) would make the `+2 to Level of
    /// all Minion Skills` of an amulet, its T2 of 3, a T3 of 5; nor does a modifier of an
    /// essence or of an alloy, of one tier: the tables of an alloy merge only with those of an
    /// alloy of its type (the cast speed and cold damage of the Transcendent Alloy on a wand,
    /// the values of the staves and the one-handed ones, two tiers).
    pub(super) fn same_modifier(&self, other: &Self) -> bool {
        if self.domain == Domain::Desecrated || other.domain == Domain::Desecrated {
            return std::ptr::eq(self, other);
        }
        self.source == other.source
            && self.kind == other.kind
            && self.affix == other.affix
            && self.lines == other.lines
    }
}

pub(super) static TABLES: LazyLock<Vec<Table>> =
    LazyLock::new(|| read(include_str!("../../../assets/game-data/modifiers.ndjson")));

/// Reads one table per line; a line that can't be read is logged and left out (the tests read
/// every line of the embedded file).
pub(super) fn read(text: &str) -> Vec<Table> {
    let mut tables = Vec::new();
    for (index, line) in text.lines().enumerate() {
        match serde_json::from_str::<Table>(line) {
            Ok(table) => tables.push(table),
            Err(error) => log::error!(
                "Game data, modifier table {} unreadable: {error}",
                index + 1
            ),
        }
    }
    tables
}

/// Whether no table of `tables` before the one at `index` holds its modifier: each modifier
/// is looked at once, at its first table.
pub(super) fn first_of_its_modifier(tables: &[Table], index: usize) -> bool {
    let Some((table, before)) = tables.get(..=index).and_then(<[Table]>::split_last) else {
        return false;
    };
    !before.iter().any(|other| other.same_modifier(table))
}

#[cfg(test)]
mod game_tests;
