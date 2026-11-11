//! Takes the icons and the names of the items of the currency exchange of the trade site,
//! embedded in the application, into `assets/exchange/`: the currencies an offer may be
//! priced in. Run it again at a new league or after a patch of the game that adds items:
//!
//! - `cargo run -p exchange-icons`.
//!
//! It reads `/api/trade2/data/static` in English and in French, downloads the image of each
//! item from the CDN of Grinding Gear Games, shrinks it to [`ICON_PIXELS`] pixels and writes it as
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

/// The result of a step of the tool, its error a message that stops it.
type Result<T> = std::result::Result<T, Box<dyn Error>>;

/// The static data of the trade site, in English.
const ENGLISH_STATIC_DATA: &str = "https://www.pathofexile.com/api/trade2/data/static";
/// The static data of the trade site, in French.
const FRENCH_STATIC_DATA: &str = "https://fr.pathofexile.com/api/trade2/data/static";
/// The CDN of the images, before their path in the static data.
const CDN: &str = "https://web.poecdn.com";
/// How the tool names itself to the servers it reads.
const USER_AGENT: &str =
    "rust-exile-market exchange icons tool (+https://github.com/aunimhsn/rust-exile-market)";
/// The size of the icons written, in pixels: twice the 18 points of an offer row, so that
/// they stay sharp on a screen at twice the scale.
const ICON_PIXELS: u32 = 36;
/// The time between two requests to the trade site, as the application waits.
const TRADE_SITE_PAUSE: Duration = Duration::from_secs(1);
/// The time between two images of the CDN, built to serve many.
const CDN_PAUSE: Duration = Duration::from_millis(200);
/// Where the icons and their index are written, from the root of the repository.
const OUTPUT: &str = "assets/exchange";
/// The index of the icons in [`OUTPUT`]: the names and the group of each item.
const INDEX: &str = "currencies.ndjson";
/// The most an answer read may hold, a bound against one that doesn't end.
const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

/// The answer of `/data/static`: the items of the currency exchange, by group.
#[derive(Deserialize)]
struct StaticData {
    result: Vec<Group>,
}

/// A group of the currency exchange, e.g. `Currency`, and its items.
#[derive(Deserialize)]
struct Group {
    id: String,
    entries: Vec<Entry>,
}

/// An item of a group, by its id and its name.
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
    let [english, french] = static_data(&agent)?;
    // Every icon is taken before any file is replaced, so that a failure halfway leaves the
    // icons of the last run as they were.
    let icons = download_icons(&agent, &english, &french)?;
    icons.replace_those_of_the_last_run()?;
    println!("{} icons written to {OUTPUT}/", icons.files.len());
    Ok(())
}

/// The static data of the trade site in English and in French, a pause between the two.
fn static_data(agent: &ureq::Agent) -> Result<[StaticData; 2]> {
    let english = read_static_data(agent, "en", ENGLISH_STATIC_DATA)?;
    thread::sleep(TRADE_SITE_PAUSE);
    let french = read_static_data(agent, "fr", FRENCH_STATIC_DATA)?;
    Ok([english, french])
}

/// The static data at `url`, in `language`, its count of groups printed.
fn read_static_data(agent: &ureq::Agent, language: &str, url: &str) -> Result<StaticData> {
    let data: StaticData = serde_json::from_slice(&get(agent, url)?)?;
    println!("{language}: {} groups", data.result.len());
    Ok(data)
}

/// The icons of the items of the currency exchange and their index.
struct Icons {
    /// Each icon, by the name of its file.
    files: Vec<(String, Vec<u8>)>,
    /// `currencies.ndjson`.
    index: String,
}

/// The icon of each item of `english` with an image, downloaded from the CDN, a pause between
/// two, and the index of their names in English and in `french` ([`wanted_icon`]).
fn download_icons(agent: &ureq::Agent, english: &StaticData, french: &StaticData) -> Result<Icons> {
    let french_names: BTreeMap<&str, &str> = french
        .result
        .iter()
        .flat_map(|group| &group.entries)
        .map(|entry| (entry.id.as_str(), entry.text.trim()))
        .collect();
    let mut icons = Icons {
        files: Vec::new(),
        index: String::new(),
    };
    for group in &english.result {
        for entry in &group.entries {
            let Some(wanted) = wanted_icon(group, entry, &french_names)? else {
                continue;
            };
            if !icons.files.is_empty() {
                thread::sleep(CDN_PAUSE);
            }
            let png = shrunk(&get(agent, &format!("{CDN}{}", wanted.image))?)
                .map_err(|error| format!("{}: {error}", entry.id))?;
            icons.files.push((wanted.file_name, png));
            icons.index.push_str(&wanted.index_line);
            icons.index.push('\n');
        }
    }
    Ok(icons)
}

/// An icon to download: its path on the CDN, the name of its file, and its line of the index.
#[derive(Debug, PartialEq)]
struct WantedIcon<'a> {
    image: &'a str,
    file_name: String,
    index_line: String,
}

/// The icon `entry` of `group` asks for, none for an item without image. An error for an id that
/// can't name a file, or an item without a name among `french_names`.
fn wanted_icon<'a>(
    group: &Group,
    entry: &'a Entry,
    french_names: &BTreeMap<&str, &str>,
) -> Result<Option<WantedIcon<'a>>> {
    let Some(image) = &entry.image else {
        return Ok(None);
    };
    if !is_file_name(&entry.id) {
        return Err(format!("{:?}: not a file name", entry.id).into());
    }
    let french = french_names
        .get(entry.id.as_str())
        .ok_or_else(|| format!("{}: no French name", entry.id))?;
    let index_line = serde_json::to_string(&Currency {
        id: &entry.id,
        group: &group.id,
        english: entry.text.trim(),
        french,
    })?;
    Ok(Some(WantedIcon {
        image,
        file_name: format!("{}.png", entry.id),
        index_line,
    }))
}

impl Icons {
    /// Writes the icons and their index in [`OUTPUT`], in place of the icons of the last run.
    fn replace_those_of_the_last_run(&self) -> Result<()> {
        let output = Path::new(OUTPUT);
        fs::create_dir_all(output)?;
        for file in fs::read_dir(output)? {
            let path = file?.path();
            if path.extension().is_some_and(|extension| extension == "png") {
                fs::remove_file(path)?;
            }
        }
        for (name, png) in &self.files {
            fs::write(output.join(name), png)?;
        }
        fs::write(output.join(INDEX), &self.index)?;
        Ok(())
    }
}

/// Whether the id of an item can name its file on every system: no separator of a path, no
/// character Windows refuses.
fn is_file_name(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('.')
        && !id.contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        && !id.chars().any(char::is_control)
}

/// The image `png`, fit into [`ICON_PIXELS`] pixels, centered on a transparent square of that size
/// (a few items, the Starlit Ores, are taller than wide), and encoded again, compressed.
fn shrunk(png: &[u8]) -> Result<Vec<u8>> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png)?;
    let small = image
        .resize(ICON_PIXELS, ICON_PIXELS, Resize::Lanczos3)
        .into_rgba8();
    let mut square = RgbaImage::new(ICON_PIXELS, ICON_PIXELS);
    imageops::replace(
        &mut square,
        &small,
        i64::from((ICON_PIXELS - small.width()) / 2),
        i64::from((ICON_PIXELS - small.height()) / 2),
    );
    let mut bytes = Vec::new();
    square.write_with_encoder(PngEncoder::new_with_quality(
        &mut bytes,
        CompressionType::Best,
        FilterType::Adaptive,
    ))?;
    Ok(bytes)
}

/// The body of `url`, its errors naming it.
fn get(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
    let read = || -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .call()?
            .into_body()
            .into_reader()
            .take(MAX_BODY_BYTES)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    read().map_err(|error| format!("{url}: {error}").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use image::Rgba;

    /// `image` encoded as a PNG.
    fn png(image: &RgbaImage) -> Vec<u8> {
        let mut bytes = Vec::new();
        image
            .write_with_encoder(PngEncoder::new(&mut bytes))
            .expect("encoded");
        bytes
    }

    #[test]
    fn an_image_wider_than_high_is_centered_on_a_transparent_square() {
        let wide = RgbaImage::from_pixel(64, 32, Rgba([200, 100, 50, 255]));

        let square = image::load_from_memory(&shrunk(&png(&wide)).expect("shrunk"))
            .expect("read")
            .into_rgba8();

        assert_eq!(square.dimensions(), (ICON_PIXELS, ICON_PIXELS));
        assert_eq!(square.get_pixel(0, 0)[3], 0, "transparent above");
        assert_eq!(
            square.get_pixel(0, ICON_PIXELS - 1)[3],
            0,
            "transparent below"
        );
        assert_eq!(
            *square.get_pixel(ICON_PIXELS / 2, ICON_PIXELS / 2),
            Rgba([200, 100, 50, 255])
        );
    }

    /// The group `Currency` with `entries`, written as the static data write them.
    fn currency_group(entries: &str) -> Group {
        serde_json::from_str(&format!(r#"{{"id": "Currency", "entries": [{entries}]}}"#))
            .expect("a group")
    }

    /// The French names of two items, one of an id that can't name a file.
    fn french_names() -> BTreeMap<&'static str, &'static str> {
        BTreeMap::from([("exalted", "Orbe exalté"), ("a/b", "Une barre")])
    }

    #[test]
    fn an_item_with_an_image_asks_for_its_icon_and_its_line_of_the_index() {
        let group = currency_group(
            r#"{"id": "exalted", "text": " Exalted Orb ", "image": "/gen/image/exalted.png"}"#,
        );

        let wanted = wanted_icon(&group, &group.entries[0], &french_names()).expect("wanted");

        assert_eq!(
            wanted,
            Some(WantedIcon {
                image: "/gen/image/exalted.png",
                file_name: "exalted.png".to_owned(),
                index_line: r#"{"id":"exalted","group":"Currency","english":"Exalted Orb","french":"Orbe exalté"}"#
                    .to_owned(),
            })
        );
    }

    #[test]
    fn an_item_without_image_asks_for_no_icon() {
        // Neither its id nor its French name is checked.
        let group = currency_group(r#"{"id": "a/b", "text": "Gold"}"#);

        let wanted = wanted_icon(&group, &group.entries[0], &BTreeMap::new()).expect("wanted");

        assert_eq!(wanted, None);
    }

    #[test]
    fn an_id_that_cant_name_a_file_stops_the_tool() {
        let group = currency_group(r#"{"id": "a/b", "text": "A bar", "image": "/bar.png"}"#);

        let error = wanted_icon(&group, &group.entries[0], &french_names()).expect_err("an error");

        assert_eq!(error.to_string(), r#""a/b": not a file name"#);
    }

    #[test]
    fn an_item_without_french_name_stops_the_tool() {
        let group =
            currency_group(r#"{"id": "divine", "text": "Divine Orb", "image": "/divine.png"}"#);

        let error = wanted_icon(&group, &group.entries[0], &french_names()).expect_err("an error");

        assert_eq!(error.to_string(), "divine: no French name");
    }

    #[test]
    fn an_id_names_a_file_only_without_a_path_or_a_forbidden_character() {
        assert!(is_file_name("vaal"));
        assert!(is_file_name("legacy-of-mjölner"));
        for id in ["", "a/b", "a\\b", "c:", "..", "what?"] {
            assert!(!is_file_name(id), "{id:?}");
        }
    }
}
