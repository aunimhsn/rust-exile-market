//! A skill gem: its level, read from its properties (`Level: 20 (Max)`), and the limits of its
//! level, quality and sockets, checked on the trade site on 2026-11-01 (league Runes of Aldur).

/// The highest level of a gem without corruption, which raises it to [`MAX_GEM_LEVEL`].
pub const TOP_GEM_LEVEL: u32 = 20;
/// The highest quality of a gem without corruption, which raises it to [`MAX_GEM_QUALITY`],
/// in percent.
pub const TOP_GEM_QUALITY: u32 = 20;
/// The highest level of a gem: 20 from the gem, 1 from the corruption; none above.
pub const MAX_GEM_LEVEL: u32 = 21;
/// The highest quality of a gem, corrupted, in percent.
pub const MAX_GEM_QUALITY: u32 = 23;
/// The most sockets a gem has, for its support gems, also corrupted; every gem has 2.
pub const MAX_GEM_SOCKETS: u32 = 5;

/// The level of a gem, its number only: `20 (Max)` gives 20 (French `20 (Maxi)`); the trade
/// site writes the number alone, `21`.
#[must_use]
pub fn read_gem_level(value: &str) -> Option<u32> {
    value.split_whitespace().next()?.parse().ok()
}
