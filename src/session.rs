//! What the window shows, built from what happened: the state of the application, without
//! the interface.

use crate::clipboard::ClipboardEvent;
use crate::copied_item::CopiedItem;
use crate::league::League;

#[derive(Debug)]
pub struct Session {
    league: League,
    /// The last item copied in the game, shown until another one is copied.
    item: Option<CopiedItem>,
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
            ClipboardEvent::Copied(item) => self.item = Some(item),
            ClipboardEvent::Unreadable => self.clipboard_readable = false,
            ClipboardEvent::Readable => self.clipboard_readable = true,
        }
    }

    #[must_use]
    pub fn league(&self) -> &League {
        &self.league
    }

    #[must_use]
    pub fn item(&self) -> Option<&CopiedItem> {
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

    fn session() -> Session {
        Session::new(League::new("Forbidden Rites", LeagueMode::Softcore))
    }

    fn item(text: &str) -> CopiedItem {
        CopiedItem::from_text(text).expect("an item")
    }

    #[test]
    fn shows_the_last_copied_item() {
        let mut session = session();
        assert_eq!(session.item(), None);

        session.apply(ClipboardEvent::Copied(item("Item Class: Rings")));
        session.apply(ClipboardEvent::Copied(item("Item Class: Amulets")));

        assert_eq!(session.item(), Some(&item("Item Class: Amulets")));
    }

    #[test]
    fn keeps_the_item_while_the_clipboard_is_unreadable() {
        let mut session = session();
        session.apply(ClipboardEvent::Copied(item("Item Class: Rings")));

        session.apply(ClipboardEvent::Unreadable);
        assert!(!session.clipboard_readable());
        assert_eq!(session.item(), Some(&item("Item Class: Rings")));

        session.apply(ClipboardEvent::Readable);
        assert!(session.clipboard_readable());
    }
}
