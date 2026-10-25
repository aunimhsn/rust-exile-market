//! The log of the application: what happened, to follow it and to find the cause of a bug.
//!
//! The log of the current session is `exile-market.log`, in the state directory of the
//! application on Linux, `~/Library/Logs/exile-market` on macOS, the local data directory on
//! Windows; the log of the previous session stays next to it, as `exile-market.previous.log`.
//! Debug builds also write to the error output. The texts copied outside the game are never
//! logged.

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use log::LevelFilter;
use simplelog::{CombinedLogger, ConfigBuilder, SharedLogger, WriteLogger};

const FILE_NAME: &str = "exile-market.log";
const PREVIOUS_FILE_NAME: &str = "exile-market.previous.log";

/// The log file of this session.
struct LogFile {
    path: PathBuf,
    file: File,
    /// Why the log of the previous session couldn't be kept, if it couldn't.
    rotation_error: Option<io::Error>,
}

/// Starts the log, and logs the panics. Without a log file (a folder that can't be written),
/// the application works the same.
pub fn init() {
    let config = {
        let mut builder = ConfigBuilder::new();
        builder.set_time_format_rfc3339();
        // Local time, or UTC when the offset can't be read.
        let _ = builder.set_time_offset_to_local();
        // arboard logs every reading of the X11 clipboard that times out: the clipboard
        // watching logs the lasting failures itself.
        builder.add_filter_ignore_str("arboard");
        builder.build()
    };

    let mut loggers: Vec<Box<dyn SharedLogger>> = Vec::new();
    #[cfg(debug_assertions)]
    loggers.push(simplelog::TermLogger::new(
        LevelFilter::Debug,
        config.clone(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    ));
    let log_file = open_file().map(|log_file| {
        loggers.push(WriteLogger::new(LevelFilter::Info, config, log_file.file));
        (log_file.path, log_file.rotation_error)
    });
    if CombinedLogger::init(loggers).is_err() {
        // A logger is already installed: keep it.
        return;
    }
    match log_file {
        Ok((path, rotation_error)) => {
            log::info!("Log file: {}", without_home(&path).display());
            if let Some(error) = rotation_error {
                log::warn!("The log of the previous session couldn't be kept: {error}");
            }
        }
        Err(error) => log::warn!("No log file: {error}"),
    }

    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let thread = std::thread::current();
        log::error!(
            "Panic in the {} thread: {panic}",
            thread.name().unwrap_or("unnamed")
        );
        default_hook(panic);
    }));
}

/// Opens the log file of this session, after keeping the one of the previous session when
/// it can.
fn open_file() -> io::Result<LogFile> {
    let directory = log_directory().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no home directory for the log file",
        )
    })?;
    fs::create_dir_all(&directory)?;
    let path = directory.join(FILE_NAME);
    let rotation_error = match fs::rename(&path, directory.join(PREVIOUS_FILE_NAME)) {
        Ok(()) => None,
        // The first session: no previous log.
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => Some(error),
    };
    let file = File::create(&path)?;
    Ok(LogFile {
        path,
        file,
        rotation_error,
    })
}

fn log_directory() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let home = BaseDirs::new()?.home_dir().to_path_buf();
        return Some(home.join("Library/Logs").join(crate::FOLDER_NAME));
    }
    let directories = crate::files::project_directories()?;
    Some(
        directories
            .state_dir()
            .unwrap_or_else(|| directories.data_local_dir())
            .to_path_buf(),
    )
}

/// The path with `~` for the home directory, so that the log doesn't show the account name.
pub fn without_home(path: &Path) -> PathBuf {
    BaseDirs::new()
        .and_then(|directories| {
            path.strip_prefix(directories.home_dir())
                .ok()
                .map(|rest| Path::new("~").join(rest))
        })
        .unwrap_or_else(|| path.to_path_buf())
}
