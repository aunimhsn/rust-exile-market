//! The lines of a copied modifier on the stats of the game: the English templates of a French
//! line, which the tables are searched with, to look at when the game data are generated again.

use super::*;
use crate::game_data::lines::modifier_stats;
use crate::game_data::modifiers::fixtures::modifier;
use crate::language::Language;

/// The mana a flask recovers, in French: the line of two English stats, of the belts and of the
/// jewels.
const FLASK_MANA: &str = "25% d'Augmentation de la Récupération de Mana des Flacons";
/// The chance to inflict a bleeding, in French: the line of two English stats too.
const BLEEDING: &str = "15% de chances d'infliger le Saignement au Toucher";
/// The rarity of the items found, in French: the line of a stat of the tablets, whose English
/// template is the one of the equipment.
const RARITY: &str = "20% d'Augmentation de la Rareté des Objets trouvés";

/// The English templates of the one line `text`, written in French, which the game marks of
/// `origin`.
fn english_templates(text: &str, origin: Option<ModifierOrigin>) -> Option<Vec<&'static str>> {
    let line = Modifier {
        origin,
        ..modifier(None, &[text])
    };
    let matches = modifier_stats(Language::French, &line, None);
    let copied = Copied::of(&line, &matches)?;
    Some(
        copied
            .lines
            .into_iter()
            .flat_map(|line| line.templates)
            .collect(),
    )
}

#[test]
fn a_french_line_has_the_english_templates_of_every_stat_that_writes_it() {
    assert_eq!(
        english_templates(FLASK_MANA, None),
        Some(vec![
            "#% increased Flask Mana Recovery rate",
            "#% increased Mana Recovery from Flasks"
        ])
    );
    assert_eq!(
        english_templates(BLEEDING, None),
        Some(vec![
            "#% chance to inflict Bleeding on Hit",
            "#% chance to cause Bleeding on Hit"
        ])
    );
    assert_eq!(
        english_templates(RARITY, None),
        Some(vec!["#% increased Rarity of Items found"])
    );
}

#[test]
fn a_french_line_has_the_same_english_templates_whatever_the_game_marks_it() {
    // None of these stats has an id in the crafted group of the trade site: read in the group
    // of its mark, a crafted line lost the template of its table.
    for text in [FLASK_MANA, BLEEDING, RARITY] {
        let unmarked = english_templates(text, None);
        for origin in [
            ModifierOrigin::Crafted,
            ModifierOrigin::Fractured,
            ModifierOrigin::Desecrated,
        ] {
            assert_eq!(
                english_templates(text, Some(origin)),
                unmarked,
                "{text}, {origin:?}"
            );
        }
    }
}
