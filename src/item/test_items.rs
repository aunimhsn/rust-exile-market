//! Every item of `test-items/` (see its `README.md`), by the name of its file: one list for
//! the tests that read them all, so that an item copied later is checked by each of them.

macro_rules! test_items {
    ($($file:literal,)*) => {
        [$(($file, include_str!(concat!("test-items/", $file))),)*]
    };
}

/// The name of the file of each copied item, and its text.
pub(crate) const ALL: [(&str, &str); 70] = test_items![
    "en-currency-uncut-gem.txt",
    "en-magic-mace.txt",
    "en-normal-helmet.txt",
    "en-owner-currency-uncut-gem.txt",
    "en-owner-magic-tablet.txt",
    "en-owner-normal-quarterstaff-exceptional.txt",
    "en-owner-normal-quarterstaff-rune.txt",
    "en-owner-normal-ring-implicit.txt",
    "en-owner-rare-amulet-catalyst.txt",
    "en-owner-rare-focus-kurgal.txt",
    "en-owner-rare-focus-unrevealed.txt",
    "en-owner-rare-helmet-amanamu.txt",
    "en-owner-rare-helmet-runeforged.txt",
    "en-owner-rare-helmet-ulaman.txt",
    "en-owner-rare-jewel-time-lost.txt",
    "en-owner-rare-ring-breach.txt",
    "en-owner-rare-ring-catalyst.txt",
    "en-owner-rare-sceptre-sanctified.txt",
    "en-owner-rare-two-hand-mace-fractured.txt",
    "en-owner-rare-waystone.txt",
    "en-owner-skill-gem.txt",
    "en-owner-trade-rare-bow-mirrored.txt",
    "en-owner-trade-rare-helmet-fractured.txt",
    "en-owner-trade-unique-belt-mutated.txt",
    "en-owner-unique-charm.txt",
    "en-owner-unique-mace-fixed-values.txt",
    "en-plain-currency-large-stack.txt",
    "en-plain-magic-jewel-unidentified.txt",
    "en-plain-rare-ring-unidentified.txt",
    "en-plain-rare-ring.txt",
    "en-plain-unique-jewel.txt",
    "en-rare-bow-fractured.txt",
    "en-rare-crossbow-all-modifiers.txt",
    "en-rare-wand-unidentified.txt",
    "en-unique-focus.txt",
    "fr-currency-stack.txt",
    "fr-currency-uncut-gem.txt",
    "fr-magic-mana-flask.txt",
    "fr-magic-tablet.txt",
    "fr-normal-exceptional-shield.txt",
    "fr-normal-quarterstaff-exceptional.txt",
    "fr-normal-quarterstaff-rune.txt",
    "fr-rare-amulet-catalyst.txt",
    "fr-rare-body-armour-two-hybrids.txt",
    "fr-rare-body-armour.txt",
    "fr-rare-focus-kurgal.txt",
    "fr-rare-focus-unrevealed.txt",
    "fr-rare-gloves-alloy.txt",
    "fr-rare-helmet-amanamu.txt",
    "fr-rare-helmet-runeforged.txt",
    "fr-rare-helmet-ulaman.txt",
    "fr-rare-jewel-time-lost-large.txt",
    "fr-rare-jewel-time-lost.txt",
    "fr-rare-ring-breach.txt",
    "fr-rare-ring-catalyst.txt",
    "fr-rare-ring-implicit.txt",
    "fr-rare-sceptre-sanctified.txt",
    "fr-rare-staff.txt",
    "fr-rare-two-hand-mace-fractured.txt",
    "fr-rare-waystone.txt",
    "fr-skill-gem.txt",
    "fr-trade-rare-body-armour.txt",
    "fr-trade-rare-bow-mirrored.txt",
    "fr-trade-rare-helmet-fractured.txt",
    "fr-trade-unique-belt-mutated.txt",
    "fr-unique-belt-vaal.txt",
    "fr-unique-charm.txt",
    "fr-unique-crossbow.txt",
    "fr-unique-mace-fixed-values.txt",
    "fr-unique-ring-mirrored.txt",
];

/// The texts of the copied items.
pub(crate) fn texts() -> impl Iterator<Item = &'static str> {
    ALL.into_iter().map(|(_, text)| text)
}

#[test]
fn every_file_of_the_folder_is_in_the_list() {
    let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/src/item/test-items");
    let mut files: Vec<String> = std::fs::read_dir(folder)
        .expect("the folder of the test items")
        .map(|entry| entry.expect("a file").file_name())
        .filter_map(|name| name.into_string().ok())
        .filter(|name| name != "README.md")
        .collect();
    files.sort();

    let listed: Vec<&str> = ALL.iter().map(|&(name, _)| name).collect();
    assert_eq!(listed, files);
}
