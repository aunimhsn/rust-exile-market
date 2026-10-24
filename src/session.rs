//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use crate::clipboard::ClipboardEvent;
use crate::item::{CopiedItem, Item, ParseError};
use crate::league::League;

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
    league: League,
    /// Shown until another item is copied.
    item: Option<ShownItem>,
    clipboard_readable: bool,
}

impl Session {
    #[must_use]
    pub fn new(league: League) -> Self {
        Self {
            league,
            item: None,
            clipboard_readable: true,
        }
    }

    pub fn apply(&mut self, event: ClipboardEvent) {
        match event {
            ClipboardEvent::Copied(copied) => self.item = Some(ShownItem::new(copied)),
            ClipboardEvent::Unreadable => self.clipboard_readable = false,
            ClipboardEvent::Readable => self.clipboard_readable = true,
        }
    }

    #[must_use]
    pub fn league(&self) -> &League {
        &self.league
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

    const RING: &str = "Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring";
    const AMULET: &str = "Item Class: Amulets\nRarity: Magic\nJade Amulet";

    fn session() -> Session {
        Session::new(League::new("Forbidden Rites", LeagueMode::Softcore))
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
}
