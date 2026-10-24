//! Path of Exile 2 leagues: the current ones, given by the price database, and the two
//! permanent ones, where the game moves the characters at the end of a league.

/// The mode of a league: softcore and hardcore leagues have separate economies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeagueMode {
    Softcore,
    Hardcore,
}

/// The prefix of the hardcore leagues, e.g. "HC Forbidden Rites".
const HARDCORE_PREFIX: &str = "HC ";
/// The permanent leagues, by their id, and the name both show.
const STANDARD: &str = "Standard";
const HARDCORE: &str = "Hardcore";

/// A league of Path of Exile 2, known by its id on the trade site and the price database,
/// e.g. "HC Forbidden Rites".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct League {
    id: String,
}

impl League {
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    /// The permanent league of `mode`.
    #[must_use]
    pub fn permanent(mode: LeagueMode) -> Self {
        Self::new(match mode {
            LeagueMode::Softcore => STANDARD,
            LeagueMode::Hardcore => HARDCORE,
        })
    }

    /// Its id on the trade site and the price database.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Its name on screen: without the hardcore prefix, and "Standard" for both permanent
    /// leagues; the mode is told by the color.
    #[must_use]
    pub fn name(&self) -> &str {
        if self.is_permanent() {
            return STANDARD;
        }
        self.id.strip_prefix(HARDCORE_PREFIX).unwrap_or(&self.id)
    }

    #[must_use]
    pub fn mode(&self) -> LeagueMode {
        if self.id == HARDCORE || self.id.starts_with(HARDCORE_PREFIX) {
            LeagueMode::Hardcore
        } else {
            LeagueMode::Softcore
        }
    }

    #[must_use]
    pub fn is_permanent(&self) -> bool {
        self.id == STANDARD || self.id == HARDCORE
    }
}

/// A line of the list of the leagues: one league in its two modes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueRow {
    pub softcore: Option<League>,
    pub hardcore: Option<League>,
}

/// The league to show, after the list of the leagues arrived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub league: League,
    /// The league the player had chosen has ended: this is the permanent one of its mode.
    pub ended: Option<League>,
}

/// The leagues the player can choose: the current ones, in the order of the price database,
/// then the two permanent ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leagues {
    current: Vec<League>,
}

impl Leagues {
    /// The leagues, from the current ones of the price database; the permanent ones are
    /// added whatever it says.
    #[must_use]
    pub fn new(current: impl IntoIterator<Item = League>) -> Self {
        Self {
            current: current
                .into_iter()
                .filter(|league| !league.is_permanent())
                .collect(),
        }
    }

    /// The lines of the list: the current leagues by name, in their order, then the
    /// permanent ones.
    #[must_use]
    pub fn rows(&self) -> Vec<LeagueRow> {
        let mut rows: Vec<LeagueRow> = Vec::new();
        for league in &self.current {
            let index = rows
                .iter()
                .position(|row| row_name(row) == league.name())
                .unwrap_or_else(|| {
                    rows.push(LeagueRow {
                        softcore: None,
                        hardcore: None,
                    });
                    rows.len() - 1
                });
            let row = &mut rows[index];
            let slot = match league.mode() {
                LeagueMode::Softcore => &mut row.softcore,
                LeagueMode::Hardcore => &mut row.hardcore,
            };
            slot.get_or_insert_with(|| league.clone());
        }
        rows.push(LeagueRow {
            softcore: Some(League::permanent(LeagueMode::Softcore)),
            hardcore: Some(League::permanent(LeagueMode::Hardcore)),
        });
        rows
    }

    #[must_use]
    pub fn contains(&self, league: &League) -> bool {
        league.is_permanent() || self.current.contains(league)
    }

    /// The league by default: the first current softcore league, Standard when there is none.
    #[must_use]
    pub fn default_league(&self) -> League {
        self.first_current(LeagueMode::Softcore)
            .unwrap_or_else(|| League::permanent(LeagueMode::Softcore))
    }

    /// The league to show: the one the player chose while it lasts, the permanent league of
    /// its mode once it has ended, and the league by default when the player never chose one.
    #[must_use]
    pub fn choose(&self, chosen: Option<&League>) -> Choice {
        match chosen {
            None => Choice {
                league: self.default_league(),
                ended: None,
            },
            Some(league) if self.contains(league) => Choice {
                league: league.clone(),
                ended: None,
            },
            Some(league) => Choice {
                league: League::permanent(league.mode()),
                ended: Some(league.clone()),
            },
        }
    }

    /// The new league to offer a player on a permanent league: the first current league of
    /// its mode that the player didn't close the bubble of.
    #[must_use]
    pub fn new_league(&self, chosen: &League, closed: &[String]) -> Option<League> {
        if !chosen.is_permanent() {
            return None;
        }
        self.current
            .iter()
            .find(|league| league.mode() == chosen.mode() && !closed.contains(&league.id))
            .cloned()
    }

    fn first_current(&self, mode: LeagueMode) -> Option<League> {
        self.current
            .iter()
            .find(|league| league.mode() == mode)
            .cloned()
    }
}

fn row_name(row: &LeagueRow) -> &str {
    row.softcore
        .as_ref()
        .or(row.hardcore.as_ref())
        .map_or("", League::name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leagues() -> Leagues {
        Leagues::new(
            [
                "Forbidden Rites",
                "HC Forbidden Rites",
                "Runes of Aldur",
                "HC Runes of Aldur",
            ]
            .map(League::new),
        )
    }

    #[test]
    fn the_mode_and_the_name_come_from_the_id() {
        let hardcore = League::new("HC Forbidden Rites");

        assert_eq!(hardcore.mode(), LeagueMode::Hardcore);
        assert_eq!(hardcore.name(), "Forbidden Rites");
        assert_eq!(League::new("Forbidden Rites").mode(), LeagueMode::Softcore);
    }

    #[test]
    fn both_permanent_leagues_show_as_standard() {
        let softcore = League::permanent(LeagueMode::Softcore);
        let hardcore = League::permanent(LeagueMode::Hardcore);

        assert_eq!((softcore.id(), softcore.name()), ("Standard", "Standard"));
        assert_eq!((hardcore.id(), hardcore.name()), ("Hardcore", "Standard"));
        assert_eq!(hardcore.mode(), LeagueMode::Hardcore);
        assert!(softcore.is_permanent() && hardcore.is_permanent());
    }

    #[test]
    fn the_list_pairs_each_league_with_its_hardcore_trade_then_the_permanent_ones() {
        let rows = leagues().rows();
        let names: Vec<(Option<&str>, Option<&str>)> = rows
            .iter()
            .map(|row| {
                (
                    row.softcore.as_ref().map(League::id),
                    row.hardcore.as_ref().map(League::id),
                )
            })
            .collect();

        assert_eq!(
            names,
            [
                (Some("Forbidden Rites"), Some("HC Forbidden Rites")),
                (Some("Runes of Aldur"), Some("HC Runes of Aldur")),
                (Some("Standard"), Some("Hardcore")),
            ]
        );
    }

    #[test]
    fn a_league_without_hardcore_trade_has_half_a_line() {
        let rows = Leagues::new([League::new("Event")]).rows();

        assert_eq!(rows[0].softcore, Some(League::new("Event")));
        assert_eq!(rows[0].hardcore, None);
    }

    #[test]
    fn the_permanent_leagues_given_by_the_database_are_not_listed_twice() {
        let rows = Leagues::new(["Standard", "Hardcore", "Event"].map(League::new)).rows();

        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn the_league_by_default_is_the_first_current_softcore_one() {
        assert_eq!(leagues().default_league(), League::new("Forbidden Rites"));
        assert_eq!(
            Leagues::new([]).default_league(),
            League::permanent(LeagueMode::Softcore)
        );
    }

    #[test]
    fn a_chosen_league_stays_while_it_lasts() {
        let chosen = League::new("HC Runes of Aldur");

        let choice = leagues().choose(Some(&chosen));

        assert_eq!(choice.league, chosen);
        assert_eq!(choice.ended, None);
    }

    #[test]
    fn an_ended_league_gives_the_permanent_league_of_its_mode() {
        let ended = League::new("HC Dawn of the Hunt");

        let choice = leagues().choose(Some(&ended));

        assert_eq!(choice.league, League::permanent(LeagueMode::Hardcore));
        assert_eq!(choice.ended, Some(ended));
    }

    #[test]
    fn without_a_chosen_league_the_default_one_is_shown() {
        assert_eq!(
            leagues().choose(None).league,
            League::new("Forbidden Rites")
        );
    }

    #[test]
    fn a_player_on_a_permanent_league_is_offered_the_new_league_of_its_mode() {
        let hardcore = League::permanent(LeagueMode::Hardcore);

        assert_eq!(
            leagues().new_league(&hardcore, &[]),
            Some(League::new("HC Forbidden Rites"))
        );
        // Closed for that league: the next one of the mode, if any.
        assert_eq!(
            leagues().new_league(&hardcore, &["HC Forbidden Rites".to_owned()]),
            Some(League::new("HC Runes of Aldur"))
        );
        assert_eq!(
            leagues().new_league(&League::new("Forbidden Rites"), &[]),
            None
        );
    }
}
