//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use crate::clipboard::ClipboardEvent;
use crate::item::{CopiedItem, Item, ParseError};
use crate::league::{League, Leagues};
use crate::market::MarketEvent;

/// The last item copied in the game, and what the parser read from it.
#[derive(Debug)]
pub struct ShownItem {
    copied: CopiedItem,
    parsed: Result<Item, ParseError>,
}

impl ShownItem {
    fn new(copied: CopiedItem) -> Self {
        let parsed = Item::parse(&copied);
        match &parsed {
            Ok(item) => {
                log::debug!("{item:#?}");
                for warning in item.warnings() {
                    log::warn!("A part of a copied item couldn't be read: {warning}");
                }
            }
            Err(error) => log::warn!("A copied item couldn't be read: {error}"),
        }
        Self { copied, parsed }
    }

    #[must_use]
    pub fn copied(&self) -> &CopiedItem {
        &self.copied
    }

    pub fn parsed(&self) -> Result<&Item, ParseError> {
        self.parsed.as_ref().map_err(|&error| error)
    }
}

#[derive(Debug)]
pub struct Session {
    /// The leagues of the price database, once they arrived.
    leagues: Option<Leagues>,
    /// The league shown: the one of the last session until the leagues arrive.
    league: Option<League>,
    /// The new league offered to a player on a permanent league.
    new_league: Option<League>,
    /// Shown until another item is copied.
    item: Option<ShownItem>,
    clipboard_readable: bool,
}

/// What changed in the leagues, for the settings to keep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeagueChange {
    /// The league shown is another one: chosen by default, or because the last one ended.
    Shown(League),
}

impl Session {
    /// A session showing `league`, the one of the last session, until the leagues arrive.
    #[must_use]
    pub fn new(league: Option<League>) -> Self {
        Self {
            leagues: None,
            league,
            new_league: None,
            item: None,
            clipboard_readable: true,
        }
    }

    /// Takes in what the price database sent; `closed` are the new leagues whose bubble the
    /// player closed.
    pub fn apply_market(&mut self, event: MarketEvent, closed: &[League]) -> Option<LeagueChange> {
        let MarketEvent::Leagues(leagues) = event else {
            return None;
        };
        let choice = leagues.choose(self.league.as_ref());
        if let Some(ended) = &choice.ended {
            log::info!(
                "{} has ended: switched to {}",
                ended.id(),
                choice.league.id()
            );
        }
        let change = (self.league.as_ref() != Some(&choice.league))
            .then(|| LeagueChange::Shown(choice.league.clone()));
        self.new_league = leagues.new_league(&choice.league, closed);
        self.league = Some(choice.league);
        self.leagues = Some(leagues);
        change
    }

    /// Shows `league`, chosen by the player.
    pub fn choose_league(&mut self, league: League) {
        self.new_league = None;
        self.league = Some(league);
    }

    /// Closes the bubble of the new league; the league it offered.
    pub fn close_new_league(&mut self) -> Option<League> {
        self.new_league.take()
    }

    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied(copied) => self.item = Some(ShownItem::new(copied)),
            ClipboardEvent::Unreadable => self.clipboard_readable = false,
            ClipboardEvent::Readable => self.clipboard_readable = true,
        }
    }

    /// The league shown; none before the leagues arrive at the first session.
    #[must_use]
    pub fn league(&self) -> Option<&League> {
        self.league.as_ref()
    }

    #[must_use]
    pub fn leagues(&self) -> Option<&Leagues> {
        self.leagues.as_ref()
    }

    #[must_use]
    pub fn new_league(&self) -> Option<&League> {
        self.new_league.as_ref()
    }

    #[must_use]
    pub fn item(&self) -> Option<&ShownItem> {
        self.item.as_ref()
    }

    #[must_use]
    pub fn clipboard_readable(&self) -> bool {
        self.clipboard_readable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::league::LeagueMode;

    fn leagues(ids: &[&str]) -> MarketEvent {
        MarketEvent::Leagues(Leagues::new(ids.iter().map(|&id| League::new(id))))
    }

    const RING: &str = "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring";
    const AMULET: &str = "Item Class: Amulets\nRarity: Magic\nJade Amulet";

    fn session() -> Session {
        Session::new(Some(League::new("Forbidden Rites")))
    }

    fn copy(session: &mut Session, text: &str) {
        let copied = CopiedItem::from_text(text).expect("an item");
        session.apply(ClipboardEvent::Copied(copied));
    }

    fn shown_text(session: &Session) -> Option<&str> {
        session.item().map(|item| item.copied().text())
    }

    #[test]
    fn shows_the_last_copied_item_read_by_the_parser() {
        let mut session = session();
        assert!(session.item().is_none());

        copy(&mut session, RING);
        copy(&mut session, AMULET);

        assert_eq!(shown_text(&session), Some(AMULET));
        let parsed = session.item().map(ShownItem::parsed);
        assert!(
            parsed.is_some_and(|item| item.is_ok_and(|item| item.base_type() == "Jade Amulet"))
        );
    }

    #[test]
    fn keeps_an_item_that_can_t_be_read_with_its_error() {
        let mut session = session();

        copy(&mut session, "Item Class: Rings\nWoe Loop");

        assert_eq!(
            session.item().map(|item| item.parsed().err()),
            Some(Some(ParseError::MissingRarity))
        );
    }

    #[test]
    fn keeps_the_item_while_the_clipboard_is_unreadable() {
        let mut session = session();
        copy(&mut session, RING);

        session.apply(ClipboardEvent::Unreadable);
        assert!(!session.clipboard_readable());
        assert_eq!(shown_text(&session), Some(RING));

        session.apply(ClipboardEvent::Readable);
        assert!(session.clipboard_readable());
    }

    #[test]
    fn the_first_session_shows_the_league_by_default_and_keeps_it() {
        let mut session = Session::new(None);
        assert_eq!(session.league(), None);

        let change = session.apply_market(leagues(&["Forbidden Rites", "Runes of Aldur"]), &[]);

        let league = League::new("Forbidden Rites");
        assert_eq!(session.league(), Some(&league));
        assert_eq!(change, Some(LeagueChange::Shown(league)));
    }

    #[test]
    fn a_league_that_lasts_changes_nothing() {
        let mut session = session();

        assert_eq!(
            session.apply_market(leagues(&["Forbidden Rites"]), &[]),
            None
        );
        assert_eq!(session.new_league(), None);
    }

    #[test]
    fn an_ended_league_moves_to_standard_and_offers_the_new_one() {
        let mut session = Session::new(Some(League::new("HC Dawn of the Hunt")));

        let change = session.apply_market(leagues(&["Forbidden Rites", "HC Forbidden Rites"]), &[]);

        let hardcore = League::permanent(LeagueMode::Hardcore);
        assert_eq!(change, Some(LeagueChange::Shown(hardcore.clone())));
        assert_eq!(session.league(), Some(&hardcore));
        assert_eq!(
            session.new_league(),
            Some(&League::new("HC Forbidden Rites"))
        );
    }

    #[test]
    fn a_closed_bubble_doesnt_come_back() {
        let mut session = Session::new(Some(League::permanent(LeagueMode::Softcore)));
        session.apply_market(leagues(&["Forbidden Rites"]), &[]);

        assert_eq!(
            session.close_new_league(),
            Some(League::new("Forbidden Rites"))
        );
        session.apply_market(
            leagues(&["Forbidden Rites"]),
            &[League::new("Forbidden Rites")],
        );

        assert_eq!(session.new_league(), None);
    }

    #[test]
    fn choosing_a_league_closes_the_bubble() {
        let mut session = Session::new(Some(League::permanent(LeagueMode::Softcore)));
        session.apply_market(leagues(&["Forbidden Rites"]), &[]);

        session.choose_league(League::new("Forbidden Rites"));

        assert_eq!(session.new_league(), None);
        assert_eq!(session.league(), Some(&League::new("Forbidden Rites")));
    }

    #[test]
    fn an_unreachable_database_changes_nothing() {
        let mut session = session();

        assert_eq!(session.apply_market(MarketEvent::Unreachable, &[]), None);
        assert_eq!(session.leagues(), None);
    }
}
