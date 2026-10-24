//! The images of `assets/`, embedded in the executable.

use egui::{ImageSource, Vec2, include_image, vec2};

/// The size of the full logo, in pixels: the window keeps its proportions.
pub const LOGO_SIZE: Vec2 = vec2(407.0, 372.0);

/// The emblem of the header, white, to tint.
#[must_use]
pub fn emblem() -> ImageSource<'static> {
    include_image!("../../assets/logo/emblem.svg")
}

/// The full logo, shown before the first item.
#[must_use]
pub fn logo() -> ImageSource<'static> {
    include_image!("../../assets/logo/exile-market.png")
}

/// The gear of the settings block, white, to tint.
#[must_use]
pub fn settings() -> ImageSource<'static> {
    include_image!("../../assets/icons/settings.svg")
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::load::Bytes;

    #[test]
    fn the_logo_size_is_the_size_of_its_file() {
        let ImageSource::Bytes {
            bytes: Bytes::Static(bytes),
            ..
        } = logo()
        else {
            panic!("the logo is embedded");
        };
        let image = image::load_from_memory(bytes).expect("the logo decodes");

        #[expect(clippy::cast_precision_loss, reason = "small image sizes")]
        let size = vec2(image.width() as f32, image.height() as f32);
        assert_eq!(size, LOGO_SIZE);
    }
}
