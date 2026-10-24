//! Path of Exile 2 leagues.

/// The mode of a league: softcore and hardcore leagues have separate economies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeagueMode {
    Softcore,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "the leagues are downloaded in step 7")
    )]
    Hardcore,
}

/// A league of Path of Exile 2, e.g. "Forbidden Rites" in softcore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct League {
    name: String,
    mode: LeagueMode,
}

impl League {
    #[must_use]
    pub fn new(name: impl Into<String>, mode: LeagueMode) -> Self {
        Self {
            name: name.into(),
            mode,
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn mode(&self) -> LeagueMode {
        self.mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_its_name_and_mode() {
        let league = League::new("Forbidden Rites", LeagueMode::Hardcore);

        assert_eq!(league.name(), "Forbidden Rites");
        assert_eq!(league.mode(), LeagueMode::Hardcore);
    }
}
