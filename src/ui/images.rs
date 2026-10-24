//! The images of `assets/`, embedded in the executable.

use egui::{ImageSource, include_image};

/// The emblem of the title bar and of the empty state, white, to tint.
#[must_use]
pub fn emblem() -> ImageSource<'static> {
    include_image!("../../assets/logo/emblem.svg")
}

/// The gear of the title bar, white, to tint.
#[must_use]
pub fn settings() -> ImageSource<'static> {
    include_image!("../../assets/icons/settings.svg")
}
