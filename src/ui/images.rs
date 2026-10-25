//! The images of `assets/`, embedded in the executable.

use egui::{ImageSource, include_image};

/// The emblem of the title bar and of the empty state, white, to tint.
#[must_use]
pub fn emblem() -> ImageSource<'static> {
    include_image!("../../assets/logo/emblem.svg")
}

/// The orbs of the three currencies of the prices, in their colors.
#[must_use]
pub fn divine_orb() -> ImageSource<'static> {
    include_image!("../../assets/currencies/divine-orb.png")
}

#[must_use]
pub fn chaos_orb() -> ImageSource<'static> {
    include_image!("../../assets/currencies/chaos-orb.png")
}

#[must_use]
pub fn exalted_orb() -> ImageSource<'static> {
    include_image!("../../assets/currencies/exalted-orb.png")
}

/// The gear of the title bar, white, to tint.
#[must_use]
pub fn settings() -> ImageSource<'static> {
    include_image!("../../assets/icons/settings.svg")
}
