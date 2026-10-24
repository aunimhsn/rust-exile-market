//! Exile Market: the price of a Path of Exile 2 item as soon as it is copied in the game.

// No console window next to the application on Windows, except in debug builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod clipboard;
mod i18n;
mod item;
mod language;
mod league;
mod logging;
mod session;
mod settings;
mod ui;
mod worker;

/// The name of the application, in every language.
const APP_NAME: &str = "Exile Market";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The author of the application, and their site, shown while the settings are open.
const AUTHOR: &str = "Sylent";
const AUTHOR_SITE: &str = "https://sylent.art";
/// The site as shown, without the scheme.
const AUTHOR_SITE_LABEL: &str = "sylent.art";
/// The name of the folders of the application in the user's directories (log, then settings
/// and cache).
const FOLDER_NAME: &str = "exile-market";

/// The narrow layout of version 1.
const WINDOW_SIZE: [f32; 2] = [420.0, 720.0];
const MIN_WINDOW_SIZE: [f32; 2] = [360.0, 420.0];

fn main() -> eframe::Result {
    logging::init();
    log::info!("{APP_NAME} {VERSION} started");
    // Read before the window opens, which starts always on top when it is set.
    let settings_file = settings::SettingsFile::in_config_directory();
    let settings = settings_file.load();
    let result = eframe::run_native(
        APP_NAME,
        native_options(&settings, &settings_file),
        Box::new(move |creation| {
            Ok(Box::new(app::ExileMarketApp::new(
                creation,
                settings,
                settings_file,
            )))
        }),
    );
    match &result {
        Ok(()) => log::info!("{APP_NAME} closed"),
        Err(error) => log::error!("The window failed: {error}"),
    }
    result
}

fn native_options(
    settings: &settings::Settings,
    settings_file: &settings::SettingsFile,
) -> eframe::NativeOptions {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id("rust-exile-market")
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
    };
    // On Linux, the window always goes through X11 (XWayland under Wayland), like the game run
    // by Proton, so that the clipboard can be read without the focus: see roadmap.md (Linux
    // display). Without it, winit would pick Wayland, which the OpenGL renderer isn't built for.
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_x11();
        }));
    }
    options
}
