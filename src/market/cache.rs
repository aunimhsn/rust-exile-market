//! The last downloaded prices of each league, in the cache directory of the user: the prices
//! show at once at startup, and while the price database can't be reached.

use std::path::PathBuf;

use crate::files;
use crate::league::League;
use crate::logging;
use crate::prices::PriceList;

/// Changed with the shape of [`PriceList`]: a file of another version is downloaded again.
const VERSION: u32 = 3;

/// The files of the cache.
pub struct PriceCache {
    /// `None` without a cache directory: the prices are then downloaded at each launch.
    directory: Option<PathBuf>,
}

impl PriceCache {
    /// The cache in the cache directory of the user.
    #[must_use]
    pub fn in_cache_directory() -> Self {
        let directory = files::cache_directory();
        if directory.is_none() {
            log::warn!("No cache directory: the prices will be downloaded at each launch");
        }
        Self { directory }
    }

    #[cfg(test)]
    pub fn in_directory(directory: PathBuf) -> Self {
        Self {
            directory: Some(directory),
        }
    }

    /// The prices of `league` downloaded last, if the cache has them.
    #[must_use]
    pub fn load(&self, league: &League) -> Option<PriceList> {
        let path = self.directory.as_ref()?.join(file_name(league));
        match files::read_versioned_json::<PriceList>(&path, VERSION) {
            Ok(Some(prices)) if prices.league == *league => Some(prices),
            Ok(_) => None,
            Err(error) => {
                log::info!(
                    "{} can't be read, the prices will be downloaded again: {error}",
                    logging::without_home(&path).display()
                );
                None
            }
        }
    }

    /// Keeps `prices` for the next launches. A failure is logged: the prices still show in
    /// this session.
    pub fn save(&self, prices: &PriceList) {
        let Some(directory) = &self.directory else {
            return;
        };
        let name = file_name(&prices.league);
        if let Err(error) = files::write_versioned_json(directory, &name, VERSION, prices) {
            log::warn!(
                "The prices couldn't be saved in {}: {error}",
                logging::without_home(directory).display()
            );
        }
    }
}

/// The file of the prices of `league`: its id, with only letters, digits and `-` (e.g.
/// `prices-HC-Forbidden-Rites.json`), valid on every system.
fn file_name(league: &League) -> String {
    let id: String = league
        .id()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect();
    format!("prices-{id}.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::TestDirectory;
    use crate::prices::{CurrencyPrice, Price, Rates};
    use std::fs;
    use std::time::{Duration, SystemTime};

    fn prices(league: &str) -> PriceList {
        PriceList {
            league: League::new(league),
            updated: SystemTime::UNIX_EPOCH + Duration::from_secs(1_761_400_000),
            rates: Rates {
                chaos: 75.4,
                divine: 754.0,
            },
            uniques: Vec::new(),
            currencies: vec![CurrencyPrice {
                name: "Divine Orb".to_owned(),
                exchange_id: "divine".to_owned(),
                price: Price {
                    exalted: 754.0,
                    quantity: 3000,
                    history: vec![Some(750.0), None],
                },
            }],
        }
    }

    #[test]
    fn saved_prices_load_back_for_their_league_only() {
        let directory = TestDirectory::new("price-cache");
        let cache = PriceCache::in_directory(directory.0.clone());

        cache.save(&prices("HC Forbidden Rites"));

        assert_eq!(
            cache.load(&League::new("HC Forbidden Rites")),
            Some(prices("HC Forbidden Rites"))
        );
        assert_eq!(cache.load(&League::new("Forbidden Rites")), None);
    }

    #[test]
    fn a_file_of_another_version_or_unreadable_is_ignored() {
        let directory = TestDirectory::new("price-cache-version");
        let cache = PriceCache::in_directory(directory.0.clone());
        let league = League::new("Standard");
        fs::create_dir_all(&directory.0).unwrap();

        fs::write(directory.0.join(file_name(&league)), r#"{"version": 0}"#).unwrap();
        assert_eq!(cache.load(&league), None);

        fs::write(directory.0.join(file_name(&league)), "{").unwrap();
        assert_eq!(cache.load(&league), None);
    }

    #[test]
    fn the_file_name_keeps_only_letters_and_digits() {
        assert_eq!(
            file_name(&League::new("HC Forbidden Rites")),
            "prices-HC-Forbidden-Rites.json"
        );
    }
}
