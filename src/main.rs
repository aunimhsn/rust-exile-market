// Copyright (C) 2026 Aunim HASSAN. GPL-3.0-or-later, with the additional terms of its
// section 7 in assets/NOTICE.md.

//! Exile Market: the price of a Path of Exile 2 item as soon as it is copied in the game.

// No console window next to the application on Windows, except in debug builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod bug_report;
mod clipboard;
mod exchange_currencies;
mod files;
mod filters;
mod format;
mod game_data;
mod hotkey;
mod http;
mod i18n;
mod item;
mod item_names;
mod language;
mod league;
mod logging;
mod market;
mod offers_state;
mod overlay;
mod price;
mod recent_items;
mod roll_difference;
mod session;
mod settings;
mod tier_rolls;
mod trade;
mod ui;
mod update;
mod window_error;
mod worker;
#[cfg(target_os = "linux")]
mod x11;

use std::cell::Cell;
use std::process::ExitCode;

/// The name of the application, in every language.
const APP_NAME: &str = "Exile Market";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The author of the application, and their site, shown while the settings are open.
const AUTHOR: &str = "Sylent";
const AUTHOR_SITE: &str = "https://sylent.art";
/// The site as shown, without the scheme.
const AUTHOR_SITE_LABEL: &str = "sylent.art";
/// The name of the executable (`[[bin]]` in `Cargo.toml`), also the application id of the
/// window, which the desktop groups its windows by.
const APP_ID: &str = env!("CARGO_BIN_NAME");
/// The name of the folders of the application in the user's directories (log, then settings
/// and cache).
const FOLDER_NAME: &str = APP_ID;
/// The application icon (`assets/logo/icon.svg`, rendered by `scripts/app-icon.py`); the
/// Windows executable has its own, linked by `build.rs`.
const ICON: &[u8] = include_bytes!("../assets/logo/icon.png");

/// The narrow layout of version 1.
const WINDOW_SIZE: [f32; 2] = [420.0, 720.0];
const MIN_WINDOW_SIZE: [f32; 2] = [360.0, 420.0];

fn main() -> ExitCode {
    logging::init();
    log::info!("{APP_NAME} {VERSION} started");
    // Read before the window opens, which starts always on top when it is set.
    let settings_file = settings::SettingsFile::in_config_directory();
    let settings = settings_file.load();
    // The language of the message of a window that can't open.
    let settings_language = settings.language().language();
    // Set once the window is open, so that a later error isn't told as a window that can't open.
    let opened = Cell::new(false);
    let opened_flag = &opened;
    let result = eframe::run_native(
        APP_NAME,
        native_options(&settings, &settings_file),
        Box::new(move |creation| {
            opened_flag.set(true);
            Ok(Box::new(app::ExileMarketApp::new(
                creation,
                settings,
                settings_file,
            )))
        }),
    );
    match result {
        Ok(()) => {
            log::info!("{}", logging::closed_message());
            ExitCode::SUCCESS
        }
        Err(error) if opened.get() => {
            log::error!("The window failed: {error}");
            ExitCode::FAILURE
        }
        Err(error) => {
            let cause = window_error::Cause::of(&error);
            log::error!("The window failed to open ({cause:?}): {error}");
            let translator = i18n::Translator::new(settings_language);
            window_error::show(&window_error::message(
                &translator,
                cause,
                &error.to_string(),
                logging::path(),
            ));
            // The message already told the error: no `Error: …` written after it, as returning
            // the error would.
            ExitCode::FAILURE
        }
    }
}

fn native_options(
    settings: &settings::Settings,
    settings_file: &settings::SettingsFile,
) -> eframe::NativeOptions {
    on_x11(eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_ID)
            .with_icon(icon())
            .with_inner_size(WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE)
            // The title bar of the application replaces the one of the system.
            .with_decorations(false)
            .with_window_level(settings.window_level()),
        // eframe restores the size and the position of the window of the last session, on a
        // screen that is still there.
        persist_window: true,
        persistence_path: settings_file.window_file(),
        ..Default::default()
    })
}

/// The application icon, shown by the system in its taskbar and its window switcher.
fn icon() -> std::sync::Arc<egui::IconData> {
    std::sync::Arc::new(
        eframe::icon_data::from_png_bytes(ICON)
            .expect("the embedded icon is a PNG, checked by a test"),
    )
}

/// On Linux, the window always goes through X11 (XWayland under Wayland), like the game run by
/// Proton, so that the clipboard can be read without the focus: see docs/technical-choices.md (Linux display).
/// Without it, winit would pick Wayland, which the OpenGL renderer isn't built for.
#[cfg(target_os = "linux")]
fn on_x11(options: eframe::NativeOptions) -> eframe::NativeOptions {
    use winit::platform::x11::EventLoopBuilderExtX11;
    eframe::NativeOptions {
        event_loop_builder: Some(Box::new(|builder| {
            builder.with_x11();
        })),
        ..options
    }
}

/// The other systems have one display server only.
#[cfg(not(target_os = "linux"))]
fn on_x11(options: eframe::NativeOptions) -> eframe::NativeOptions {
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_executable_is_named_without_the_prefix_of_the_repository() {
        assert_eq!(APP_ID, "exile-market");
        assert_eq!(FOLDER_NAME, APP_ID);
    }

    #[test]
    fn the_icon_is_a_square_png_large_enough_for_every_screen() {
        let icon = icon();
        assert_eq!((icon.width, icon.height), (256, 256));
    }
}
