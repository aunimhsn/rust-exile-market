//! The wait the trade site asked for by refusing a request, shared by the two threads that
//! call it: a search refused makes the download of the item names wait, and the other way
//! round, since both reach the same site from the same address.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

/// The time before which no request is sent to the trade site, if it refused one. Its clones
/// share the same time.
#[derive(Debug, Clone, Default)]
pub struct Refusal(Arc<Mutex<Option<Instant>>>);

impl Refusal {
    /// The end of the wait, while it is after `now`.
    #[must_use]
    pub fn until(&self, now: Instant) -> Option<Instant> {
        // A thread that panicked wrote a whole time or none: the wait still holds.
        let until = *self.0.lock().unwrap_or_else(PoisonError::into_inner);
        until.filter(|&until| until > now)
    }

    /// Waits until `until` at least: a wait already longer stays.
    pub fn extend(&self, until: Instant) {
        let mut known = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *known = Some(known.map_or(until, |known| known.max(until)));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn the_clones_share_the_longest_wait() {
        let now = Instant::now();
        let refusal = Refusal::default();
        let other = refusal.clone();
        assert_eq!(other.until(now), None);

        refusal.extend(now + Duration::from_mins(30));
        other.extend(now + Duration::from_mins(5));

        assert_eq!(other.until(now), Some(now + Duration::from_mins(30)));
        assert_eq!(refusal.until(now + Duration::from_mins(30)), None);
    }
}
