//! How the search of the other readings ends, on readings written by hand.

use super::*;

#[test]
fn the_search_ends_as_the_readings_it_met_and_the_room_tell() {
    // Stopped at its bound, whatever the room: it can't tell what the others would have been.
    for best_fits_room in [true, false] {
        assert_eq!(Outcome::of(None, best_fits_room), Outcome::Cut);
    }
    // A best reading that fits the room is among the readings of the search, or isn't one.
    assert_eq!(Outcome::of(Some(Met::Best), true), Outcome::BestMet);
    assert_eq!(Outcome::of(Some(Met::Others), true), Outcome::BestNotMet);
    assert_eq!(Outcome::of(Some(Met::Nothing), true), Outcome::BestNotMet);
    // One that doesn't fit it isn't a reading of the item: whether another one is.
    assert_eq!(
        Outcome::of(Some(Met::Others), false),
        Outcome::OnlyAnotherFits
    );
    assert_eq!(
        Outcome::of(Some(Met::Best), false),
        Outcome::OnlyAnotherFits
    );
    assert_eq!(Outcome::of(Some(Met::Nothing), false), Outcome::NoneFits);
}
