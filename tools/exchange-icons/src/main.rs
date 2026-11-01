//! Takes the icons and the names of the items of the currency exchange of the trade site,
//! embedded in the application, into `assets/exchange/`: the currencies an offer may be
//! priced in. Run it again at a new league or after a patch of the game that adds items:
//!
//! - `cargo run -p exchange-icons`.
//!
//! It reads `/api/trade2/data/static` in English and in French, downloads the image of each
//! item from the CDN of Grinding Gear Games, shrinks it to [`SIZE`] pixels and writes it as
//! `<id>.png`, then writes their names and groups to `currencies.ndjson`, one item per line,
//! so that the changes of a league read well in a diff. The images and the names replace
//! those of the last run.

use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::thread;
use std::time::Duration;

use image::RgbaImage;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use image::imageops::{self, FilterType as Resize};
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const STATIC_DATA: [(&str, &str); 2] = [
    ("en", "https://www.pathofexile.com/api/trade2/data/static"),
    ("fr", "https://fr.pathofexile.com/api/trade2/data/static"),
];
/// The CDN of the images, before their path in the static data.
const CDN: &str = "https://web.poecdn.com";
const USER_AGENT: &str =
    "rust-exile-market exchange icons tool (+https://github.com/aunimhsn/rust-exile-market)";
/// The size of the icons written, in pixels: twice the 18 points of an offer row, so that
/// they stay sharp on a screen at twice the scale.
const SIZE: u32 = 36;
/// The time between two requests to the trade site, as the application waits.
const TRADE_SITE_PAUSE: Duration = Duration::from_secs(1);
/// The time between two images of the CDN, built to serve many.
const CDN_PAUSE: Duration = Duration::from_millis(200);
const OUTPUT: &str = "assets/exchange";
const INDEX: &str = "currencies.ndjson";

/// The answer of `/data/static`: the items of the currency exchange, by group.
#[derive(Deserialize)]
struct StaticData {
    result: Vec<Group>,
}

#[derive(Deserialize)]
struct Group {
    id: String,
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct Entry {
    id: String,
    text: String,
    /// Its path on the CDN; `None` for the items without image.
    image: Option<String>,
}

/// An item of `currencies.ndjson`.
#[derive(Serialize)]
struct Currency<'a> {
    id: &'a str,
    /// Its group on the trade site, e.g. `Currency`, `Waystones`.
    group: &'a str,
    english: &'a str,
    french: &'a str,
}

fn main() -> Result<()> {
    let agent = ureq::Agent::new_with_defaults();
    let mut languages = Vec::new();
    for (index, (language, url)) in STATIC_DATA.iter().enumerate() {
        if index > 0 {
            thread::sleep(TRADE_SITE_PAUSE);
        }
        let data: StaticData = serde_json::from_slice(&get(&agent, url)?)?;
        println!("{language}: {} groups", data.result.len());
        languages.push(data);
    }
    let [english, french] =
        <[StaticData; 2]>::try_from(languages).map_err(|_| "the static data of two languages")?;
    let french_names: BTreeMap<&str, &str> = french
        .result
        .iter()
        .flat_map(|group| &group.entries)
        .map(|entry| (entry.id.as_str(), entry.text.trim()))
        .collect();

    let output = Path::new(OUTPUT);
    fs::create_dir_all(output)?;
    for file in fs::read_dir(output)? {
        let path = file?.path();
        if path.extension().is_some_and(|extension| extension == "png") {
            fs::remove_file(path)?;
        }
    }

    let mut index = String::new();
    let mut count = 0;
    for group in &english.result {
        for entry in &group.entries {
            let Some(image) = &entry.image else { continue };
            let french = french_names
                .get(entry.id.as_str())
                .ok_or_else(|| format!("{}: no French name", entry.id))?;
            if count > 0 {
                thread::sleep(CDN_PAUSE);
            }
            let png = shrunk(&get(&agent, &format!("{CDN}{image}"))?)
                .map_err(|error| format!("{}: {error}", entry.id))?;
            fs::write(output.join(format!("{}.png", entry.id)), png)?;
            index.push_str(&serde_json::to_string(&Currency {
                id: &entry.id,
                group: &group.id,
                english: entry.text.trim(),
                french,
            })?);
            index.push('\n');
            count += 1;
        }
    }
    fs::write(output.join(INDEX), index)?;
    println!("{count} icons written to {OUTPUT}/");
    Ok(())
}

/// The image `png`, fit into [`SIZE`] pixels, centered on a transparent square of that size
/// (a few items, the Starlit Ores, are taller than wide), and encoded again, compressed.
fn shrunk(png: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)?;
    let small = image.resize(SIZE, SIZE, Resize::Lanczos3).into_rgba8();
    let mut square = RgbaImage::new(SIZE, SIZE);
    imageops::replace(
        &mut square,
        &small,
        i64::from((SIZE - small.width()) / 2),
        i64::from((SIZE - small.height()) / 2),
    );
    let mut bytes = Vec::new();
    square.write_with_encoder(PngEncoder::new_with_quality(
        &mut bytes,
        CompressionType::Best,
        FilterType::Adaptive,
    ))?;
    Ok(bytes)
}

fn get(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
    read(agent, url).map_err(|error| format!("{url}: {error}").into())
}

fn read(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()?
        .into_body()
        .into_reader()
        .take(16 * 1024 * 1024)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}
