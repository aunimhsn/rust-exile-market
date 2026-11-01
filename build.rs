//! Embeds the icons of the currency exchange of `assets/exchange/` (taken by
//! `tools/exchange-icons`): writes `exchange_icons.rs`, the table of their ids and their
//! bytes, sorted by id, which `src/currencies.rs` includes. The icons change with each run of
//! the tool, so the table is written from the folder rather than by hand.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const ICONS: &str = "assets/exchange";

fn main() {
    println!("cargo::rerun-if-changed={ICONS}");
    let manifest = env::var("CARGO_MANIFEST_DIR").expect("set by Cargo");
    let folder = Path::new(&manifest).join(ICONS);
    let mut ids: Vec<String> = fs::read_dir(&folder)
        .expect("assets/exchange/ holds the icons of the currency exchange")
        .map(|entry| entry.expect("a file of assets/exchange/").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "png"))
        .map(|path| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .expect("an icon named by its id")
                .to_owned()
        })
        .collect();
    ids.sort();
    let mut table = String::from(
        "/// The icons of the currency exchange, by id, sorted by id.\n\
         static ICONS: &[(&str, &[u8])] = &[\n",
    );
    for id in &ids {
        let path = folder.join(format!("{id}.png"));
        writeln!(
            table,
            "    ({id:?}, include_bytes!({:?})),",
            path.display().to_string()
        )
        .expect("writing to a string");
    }
    table.push_str("];\n");
    let out = env::var("OUT_DIR").expect("set by Cargo");
    fs::write(Path::new(&out).join("exchange_icons.rs"), table)
        .expect("writing the table of the icons");
}
