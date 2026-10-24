//! The fonts, embedded in the executable: Manrope for the text, `JetBrains Mono` for the numbers.
//!
//! egui gives one weight per font family, so each weight of Manrope is its own family.

use std::sync::Arc;

use egui::{Context, FontData, FontDefinitions, FontFamily, FontId};

const MANROPE_REGULAR: &str = "manrope-regular";
const MANROPE_SEMIBOLD: &str = "manrope-semibold";
const MANROPE_EXTRABOLD: &str = "manrope-extrabold";
const JETBRAINS_MONO_SEMIBOLD: &str = "jetbrains-mono-semibold";

/// A weight of the text font.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Weight {
    Regular,
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "used by the prices and the pills from step 8")
    )]
    SemiBold,
    ExtraBold,
}

impl Weight {
    fn family(self) -> FontFamily {
        match self {
            Self::Regular => FontFamily::Proportional,
            Self::SemiBold => FontFamily::Name(MANROPE_SEMIBOLD.into()),
            Self::ExtraBold => FontFamily::Name(MANROPE_EXTRABOLD.into()),
        }
    }
}

/// The text font, at a size and a weight.
#[must_use]
pub fn text(size: f32, weight: Weight) -> FontId {
    FontId::new(size, weight.family())
}

/// The font of the numbers that line up (prices, offers): `JetBrains Mono`, semibold.
#[must_use]
pub fn numbers(size: f32) -> FontId {
    FontId::monospace(size)
}

/// Replaces the default fonts of egui by the embedded ones.
pub fn install(ctx: &Context) {
    ctx.set_fonts(definitions());
}

fn definitions() -> FontDefinitions {
    let mut definitions = FontDefinitions::empty();
    for (name, bytes) in [
        (
            MANROPE_REGULAR,
            include_bytes!("../../assets/fonts/manrope-regular.ttf").as_slice(),
        ),
        (
            MANROPE_SEMIBOLD,
            include_bytes!("../../assets/fonts/manrope-semibold.ttf").as_slice(),
        ),
        (
            MANROPE_EXTRABOLD,
            include_bytes!("../../assets/fonts/manrope-extrabold.ttf").as_slice(),
        ),
        (
            JETBRAINS_MONO_SEMIBOLD,
            include_bytes!("../../assets/fonts/jetbrains-mono-semibold.ttf").as_slice(),
        ),
    ] {
        definitions
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }

    // Each family falls back to Manrope Regular for the characters it lacks.
    let family = |names: &[&str]| names.iter().map(|&name| name.to_owned()).collect();
    definitions
        .families
        .insert(FontFamily::Proportional, family(&[MANROPE_REGULAR]));
    definitions.families.insert(
        FontFamily::Name(MANROPE_SEMIBOLD.into()),
        family(&[MANROPE_SEMIBOLD, MANROPE_REGULAR]),
    );
    definitions.families.insert(
        FontFamily::Name(MANROPE_EXTRABOLD.into()),
        family(&[MANROPE_EXTRABOLD, MANROPE_REGULAR]),
    );
    definitions.families.insert(
        FontFamily::Monospace,
        family(&[JETBRAINS_MONO_SEMIBOLD, MANROPE_REGULAR]),
    );
    definitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_family_of_the_interface_has_fonts() {
        let definitions = definitions();
        for family in [
            Weight::Regular.family(),
            Weight::SemiBold.family(),
            Weight::ExtraBold.family(),
            FontFamily::Monospace,
        ] {
            let fonts = definitions.families.get(&family);
            assert!(fonts.is_some_and(|fonts| !fonts.is_empty()), "{family:?}");
            for font in fonts.into_iter().flatten() {
                assert!(definitions.font_data.contains_key(font), "{font}");
            }
        }
    }
}
