//! The images of `assets/`, embedded in the executable.

use egui::load::Bytes;
use egui::{ImageSource, include_image};

use crate::currencies::ExchangeCurrency;

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

/// The icon of an item of the currency exchange, the currency of a price.
pub fn exchange_icon(currency: &'static ExchangeCurrency) -> egui::Image<'static> {
    egui::Image::new(ImageSource::Bytes {
        uri: format!("bytes://exchange/{}.png", currency.id).into(),
        bytes: Bytes::Static(currency.icon),
    })
}

/// The gear of the title bar, white, to tint.
#[must_use]
pub fn settings() -> ImageSource<'static> {
    include_image!("../../assets/icons/settings.svg")
}
