//! Embeds the icons of the currency exchange of `assets/exchange/` (taken by
//! `tools/exchange-icons`): writes `exchange_icons.rs`, the table of their ids and their
//! bytes, sorted by id, which `src/currencies.rs` includes. The ids come from the index of the
//! icons, `currencies.ndjson`, and the build stops when an icon is missing or left over, so
//! that the index and the icons agree on every system that builds the application.

use std::collections::BTreeSet;
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// The folder of the icons, from the root of the package.
const FOLDER: &str = "assets/exchange";
const INDEX: &str = "currencies.ndjson";

fn main() {
    println!("cargo::rerun-if-changed={FOLDER}");
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("set by Cargo");
    let folder = Path::new(&manifest).join(FOLDER);
    let index = fs::read_to_string(folder.join(INDEX)).expect("the index of the icons");
    let ids: BTreeSet<String> = index
        .lines()
        .map(|line| {
            let entry: serde_json::Value = serde_json::from_str(line).expect("a line of JSON");
            entry["id"]
                .as_str()
                .expect("an item with its id")
                .to_owned()
        })
        .collect();
    let icons: BTreeSet<String> = fs::read_dir(&folder)
        .expect("the folder of the icons")
        .map(|entry| entry.expect("a file of the icons").file_name())
        .filter_map(|name| Some(name.to_str()?.strip_suffix(".png")?.to_owned()))
        .collect();
    let missing: Vec<&String> = ids.difference(&icons).collect();
    let left_over: Vec<&String> = icons.difference(&ids).collect();
    assert!(
        missing.is_empty() && left_over.is_empty(),
        "{FOLDER}: icons missing {missing:?}, icons without item {left_over:?}: \
         run `cargo run -p exchange-icons` again"
    );

    let mut table = String::from(
        "/// The icons of the currency exchange, by id, sorted by id.\n\
         static ICONS: &[(&str, &[u8])] = &[\n",
    );
    for id in &ids {
        writeln!(
            table,
            "    ({id:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{FOLDER}/\", \
             {id:?}, \".png\"))),"
        )
        .expect("writing to a string");
    }
    table.push_str("];\n");
    let out = env::var("OUT_DIR").expect("set by Cargo");
    fs::write(Path::new(&out).join("exchange_icons.rs"), table)
        .expect("writing the table of the icons");
}
