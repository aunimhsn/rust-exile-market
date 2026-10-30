//! The session, event by event: the copied item shown, the leagues and their bubble, the
//! prices downloaded or cached, the lines read again once the stats and the names of the trade
//! site arrive (with the tiers of an item copied from the in-game trade), and the offers.

use super::*;
use crate::item::{Affix, ModifierKind};
use crate::language::Language;
use crate::league::LeagueMode;
use crate::prices::Rates;

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

/// The base type of the item shown.
fn shown_base(session: &Session) -> Option<&str> {
    let item = session.item()?.parsed().ok()?;
    Some(item.base_type())
}

#[test]
fn shows_the_last_copied_item_read_by_the_parser() {
    let mut session = session();
    assert!(session.item().is_none());

    copy(&mut session, RING);
    copy(&mut session, AMULET);

    assert_eq!(shown_base(&session), Some("Jade Amulet"));
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
fn shows_an_item_copied_from_the_in_game_trade() {
    let mut session = session();

    copy(
        &mut session,
        include_str!("../item/test-items/en-owner-trade-rare-bow-mirrored.txt"),
    );

    assert_eq!(shown_base(&session), Some("Warmonger Bow"));
    let modifiers = session
        .item()
        .and_then(|item| item.parsed().ok())
        .map(|item| item.modifiers().len());
    assert_eq!(
        modifiers,
        Some(9),
        "3 runes and 7 explicit lines, the split 258% read with the accuracy of its hybrid"
    );
}

#[test]
fn keeps_the_item_while_the_clipboard_is_unreadable() {
    let mut session = session();
    copy(&mut session, RING);

    session.apply(ClipboardEvent::Unreadable);
    assert!(!session.clipboard_readable());
    assert_eq!(shown_base(&session), Some("Ruby Ring"));

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

fn prices(league: &str, updated: SystemTime) -> PriceList {
    PriceList {
        league: League::new(league),
        updated,
        rates: Rates {
            chaos: 75.0,
            divine: 750.0,
        },
        uniques: Vec::new(),
        currencies: Vec::new(),
    }
}

/// A time of 2026-10-25.
fn now() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_793_000_000)
}

fn ago(minutes: u64) -> SystemTime {
    now() - Duration::from_mins(minutes)
}

#[test]
fn the_prices_are_downloading_until_they_arrive() {
    let mut session = session();
    assert_eq!(
        session.prices_state(now()),
        PricesState::Downloading { progress: 0.0 }
    );

    session.apply_market(
        MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
        &[],
    );
    assert_eq!(session.prices_state(now()), PricesState::Unavailable);

    session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", ago(2))), &[]);
    assert_eq!(
        session.prices_state(now()),
        PricesState::Known {
            age: Duration::from_mins(2),
            offline: false
        }
    );
}

#[test]
fn cached_prices_are_offline_while_the_database_cant_be_reached() {
    let mut session = session();

    session.apply_market(MarketEvent::Unreachable(Source::Leagues), &[]);
    session.apply_market(
        MarketEvent::CachedPrices(prices("Forbidden Rites", ago(180))),
        &[],
    );

    assert_eq!(
        session.prices_state(now()),
        PricesState::Known {
            age: Duration::from_hours(3),
            offline: true
        }
    );
}

#[test]
fn the_prices_of_another_league_are_not_shown() {
    let mut session = session();
    session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", now())), &[]);

    session.choose_league(League::new("HC Forbidden Rites"));
    assert!(session.league_prices().is_none());
    // Downloaded before the player left that league.
    session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", now())), &[]);
    assert!(session.league_prices().is_none());

    session.choose_league(League::new("Forbidden Rites"));
    assert!(session.league_prices().is_some());
}

#[test]
fn partial_prices_are_shown_while_downloading() {
    let mut session = session();

    session.apply_market(
        MarketEvent::PartialPrices {
            prices: prices("Forbidden Rites", now()),
            progress: 0.25,
        },
        &[],
    );

    assert!(session.league_prices().is_some());
    assert_eq!(
        session.prices_state(now()),
        PricesState::Downloading { progress: 0.25 }
    );
}

#[test]
fn the_leagues_read_again_dont_hide_a_failing_download() {
    let mut session = session();
    session.apply_market(MarketEvent::Prices(prices("Forbidden Rites", now())), &[]);

    session.apply_market(
        MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
        &[],
    );
    session.apply_market(leagues(&["Forbidden Rites"]), &[]);

    assert!(matches!(
        session.prices_state(now()),
        PricesState::Known { offline: true, .. }
    ));
}

#[test]
fn a_failed_download_of_the_previous_league_says_nothing_of_the_new_one() {
    let mut session = session();

    session.choose_league(League::new("HC Forbidden Rites"));
    // Sent before the thread saw the new league.
    session.apply_market(
        MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
        &[],
    );

    assert_eq!(
        session.prices_state(now()),
        PricesState::Downloading { progress: 0.0 }
    );
}

#[test]
fn a_failed_download_of_an_ended_league_says_nothing_of_standard() {
    let mut session = Session::new(Some(League::new("Dawn of the Hunt")));
    session.apply_market(leagues(&["Forbidden Rites"]), &[]);

    session.apply_market(
        MarketEvent::Unreachable(Source::Prices(League::new("Dawn of the Hunt"))),
        &[],
    );

    assert_eq!(
        session.prices_state(now()),
        PricesState::Downloading { progress: 0.0 }
    );
}

#[test]
fn an_unreachable_database_changes_nothing() {
    let mut session = session();

    assert_eq!(
        session.apply_market(
            MarketEvent::Unreachable(Source::Prices(League::new("Forbidden Rites"))),
            &[]
        ),
        None
    );
    assert_eq!(session.leagues(), None);
}

#[test]
fn the_lines_of_the_item_shown_are_read_again_with_the_stats_of_the_trade_site() {
    use crate::filters::StatKey;
    use crate::item_names::{ItemNames, Names, StatTemplate};
    use crate::trade::stat_filters::LineStat;

    let mut session = session();
    // A modifier of a patch Exiled Exchange 2 doesn't know yet, copied before the names
    // of the trade site arrive.
    copy(
        &mut session,
        "Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring\n--------\n\
         Item Level: 80\n--------\n{ Unique Modifier }\n20% reduced Damage per Patch",
    );
    let line = StatKey {
        modifier: 0,
        line: 0,
    };
    let stat = |session: &Session| session.item().unwrap().stats().get(line).cloned();
    assert_eq!(stat(&session), Some(LineStat::Unsearchable));

    let names = |stats| Names {
        exchange: std::collections::HashMap::new(),
        base_types: Vec::new(),
        stats,
    };
    let template = StatTemplate {
        id: "explicit.stat_1000000001".to_owned(),
        text: "#% reduced Damage per Patch".to_owned(),
    };
    let reshaped = session.apply_item_names(ItemNames::new(
        SystemTime::UNIX_EPOCH,
        names(vec![template]),
        names(Vec::new()),
        Vec::new(),
    ));
    assert!(!reshaped, "a unique: its lines stay as they were");

    assert!(
        matches!(stat(&session), Some(LineStat::Searched(_))),
        "{:?}",
        stat(&session)
    );
}

#[test]
fn the_base_type_among_the_filters_of_a_magic_item_comes_with_the_names() {
    const MACE: &str = include_str!("../item/test-items/en-magic-mace.txt");
    let mut session = session();
    let base_type = |session: &Session| {
        let item = session.item()?.parsed().ok()?;
        session.filtered_base_type(item).map(str::to_owned)
    };

    copy(&mut session, MACE);
    assert_eq!(base_type(&session), None, "the names aren't known yet");
    assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
    assert_eq!(base_type(&session).as_deref(), Some("Temple Maul"));
    copy(&mut session, RING);
    assert_eq!(base_type(&session).as_deref(), Some("Ruby Ring"));
    // A unique is searched by its name, without a filter of its base type.
    copy(
        &mut session,
        "Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring",
    );
    assert_eq!(base_type(&session), None);
}

#[test]
fn a_magic_item_gets_the_rolls_of_every_tier_once_the_names_tell_its_base() {
    const MACE: &str = include_str!("../item/test-items/en-magic-mace.txt");
    let mut session = session();
    // The line of its strength suffix: its added lightning damage was copied before a
    // patch changed its ranges, and no tier rolls them any more.
    let strength = |session: &Session| {
        let key = crate::filters::StatKey {
            modifier: 1,
            line: 0,
        };
        session.item()?.tiers().get(key).cloned()
    };

    copy(&mut session, MACE);
    assert_eq!(strength(&session), None, "the names aren't known yet");
    assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
    assert!(strength(&session).is_some_and(|rolls| rolls.tiers.is_some()));
    // Copied once the names are known: at once.
    copy(&mut session, MACE);
    assert!(strength(&session).is_some_and(|rolls| rolls.tiers.is_some()));
}

#[test]
fn an_item_copied_from_the_in_game_trade_gets_the_tiers_found_from_its_values() {
    const TIARA: &str = include_str!("../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    use Affix::{Prefix, Suffix};
    let mut session = session();
    let explicit = |session: &Session| -> Vec<(usize, Option<Affix>, Option<u32>)> {
        let Some(Ok(item)) = session.item().map(ShownItem::parsed) else {
            return Vec::new();
        };
        item.modifiers()
            .iter()
            .filter_map(|modifier| match modifier.kind {
                ModifierKind::Explicit(affix) => Some((modifier.lines.len(), affix, modifier.tier)),
                _ => None,
            })
            .collect()
    };

    copy(&mut session, TIARA);

    assert_eq!(
        explicit(&session),
        [
            // The fractured hybrid of `41% increased Energy Shield` and `+34 to maximum
            // Mana`, its lines joined.
            (2, Some(Prefix), Some(1)),
            (1, Some(Prefix), Some(1)),
            (1, Some(Suffix), Some(1)),
            (1, Some(Suffix), Some(1)),
            (1, Some(Suffix), Some(2)),
            (1, Some(Prefix), Some(1)),
        ]
    );
    let hybrid = session.item().and_then(|item| {
        let key = crate::filters::StatKey {
            modifier: 5,
            line: 1,
        };
        item.tiers().get(key).cloned()
    });
    assert!(
        hybrid.is_some(),
        "the rolls over every tier of its second line, after the enchantment and the runes"
    );
}

#[test]
fn an_item_copied_from_the_in_game_trade_is_read_again_from_its_copy() {
    const TIARA: &str = include_str!("../item/test-items/en-owner-trade-rare-helmet-fractured.txt");
    let mut session = session();
    let modifiers = |session: &Session| {
        let item = session.item()?.parsed().ok()?;
        Some(item.modifiers().to_vec())
    };
    copy(&mut session, TIARA);
    let read = modifiers(&session);

    // Read again from the copy as the parser read it, not from its tiers found: the same.
    assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
    assert!(!session.apply_item_names(crate::item_names::tests::item_names()));

    assert!(read.as_ref().is_some_and(|read| read.len() == 11));
    assert_eq!(modifiers(&session), read);
}

#[test]
fn a_magic_hybrid_joined_once_the_names_arrive_reshapes_the_modifiers() {
    const MACE: &str = "Item Class: Two Hand Maces\nRarity: Magic\n\
        Crackling Temple Maul of the Brute\n--------\nItem Level: 32\n--------\n\
        30% increased Physical Damage\n+60 to Accuracy Rating\n+8 to Strength";
    let mut session = session();
    let shape = |session: &Session| {
        let item = session.item()?.parsed().ok()?;
        Some(super::shape(item.modifiers()))
    };
    copy(&mut session, MACE);
    assert_eq!(
        shape(&session),
        Some(vec![1, 1, 1]),
        "its base isn't known yet"
    );

    // The filters the player changed on its lines no longer fall on the same lines.
    assert!(session.apply_item_names(crate::item_names::tests::item_names()));

    assert_eq!(shape(&session), Some(vec![2, 1]));
    let hybrid = session
        .item()
        .and_then(|item| item.parsed().ok())
        .and_then(|item| {
            Some((
                item.modifiers().first()?.kind,
                item.modifiers().first()?.tier,
            ))
        });
    assert_eq!(
        hybrid,
        Some((ModifierKind::Explicit(Some(Affix::Prefix)), Some(6)))
    );

    // Names that don't know its base: read again from its copy, its lines apart again.
    let unknown = || crate::item_names::Names {
        exchange: std::collections::HashMap::new(),
        base_types: Vec::new(),
        stats: Vec::new(),
    };
    assert!(session.apply_item_names(ItemNames::new(
        SystemTime::UNIX_EPOCH,
        unknown(),
        unknown(),
        Vec::new(),
    )));
    assert_eq!(shape(&session), Some(vec![1, 1, 1]));
}

#[test]
fn a_magic_item_copied_from_the_in_game_trade_gets_its_tiers_once_the_names_tell_its_base() {
    const MACE: &str = "Item Class: Two Hand Maces\nRarity: Magic\n\
        Crackling Temple Maul of the Brute\n--------\nItem Level: 32\n--------\n\
        Adds 1 to 50 Lightning Damage\n+8 to Strength";
    let mut session = session();
    let strength = |session: &Session| {
        let item = session.item()?.parsed().ok()?;
        let modifier = item.modifiers().get(1)?;
        Some((modifier.kind, modifier.tier.is_some()))
    };

    copy(&mut session, MACE);
    assert_eq!(
        strength(&session),
        Some((ModifierKind::Explicit(None), false)),
        "the names aren't known yet"
    );
    assert!(!session.apply_item_names(crate::item_names::tests::item_names()));
    assert_eq!(
        strength(&session),
        Some((ModifierKind::Explicit(Some(Affix::Suffix)), true))
    );
    // Its added lightning damage was copied before a patch changed its ranges: no tier.
    let damage = session
        .item()
        .and_then(|item| item.parsed().ok())
        .and_then(|item| Some(item.modifiers().first()?.kind));
    assert_eq!(damage, Some(ModifierKind::Explicit(None)));
}

#[test]
fn the_offers_are_searched_in_the_league_shown_once_the_item_is_identified() {
    const UNIQUE: &str = "Item Class: Rings\nRarity: Unique\nAndvarius\nGold Ring";
    const UNIDENTIFIED: &str =
        "Item Class: Rings\nRarity: Unique\nGold Ring\n--------\nUnidentified";
    let parsed = |session: &Session| -> Item { session.item().unwrap().parsed().unwrap().clone() };
    let mut unknown_league = Session::new(None);
    copy(&mut unknown_league, UNIQUE);
    assert_eq!(
        unknown_league.trade_search(&parsed(&unknown_league), SearchFilters::default()),
        None,
        "the league isn't known yet"
    );
    let unknown = parsed(&unknown_league);
    assert_eq!(unknown_league.search_kind(&unknown), None);
    assert!(!unknown_league.tradable(&unknown));

    let mut session = session();
    copy(&mut session, UNIQUE);
    let unique = parsed(&session);
    let search = session
        .trade_search(&unique, SearchFilters::default())
        .expect("a search");
    assert_eq!(search.league, League::new("Forbidden Rites"));
    assert_eq!(search.language, Language::English);
    assert!(!session.must_identify(&unique));
    assert_eq!(session.search_kind(&unique), Some(SearchKind::Items));
    assert!(session.tradable(&unique));

    copy(&mut session, UNIDENTIFIED);
    let unidentified = parsed(&session);
    assert_eq!(
        session.trade_search(&unidentified, SearchFilters::default()),
        None
    );
    assert!(session.must_identify(&unidentified));
    assert_eq!(session.search_kind(&unidentified), None);
    // Its link searches its base type.
    assert!(session.tradable(&unidentified));
}
