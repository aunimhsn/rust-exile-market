//! The folders of the application, and the files it writes: never half a file, even after a
//! crash.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

/// The folders of the application in the user's directories (configuration, cache, log);
/// none when the system gives no home directory.
#[must_use]
pub fn project_directories() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", crate::FOLDER_NAME)
}

/// The cache directory of the application; none when the system gives no home directory.
#[must_use]
pub fn cache_directory() -> Option<PathBuf> {
    project_directories().map(|directories| directories.cache_dir().to_path_buf())
}

/// A JSON file with the version of the shape of its value.
#[derive(Serialize, Deserialize)]
struct Versioned<T> {
    version: u32,
    value: T,
}

/// Only the version of a JSON file, read when the whole can't be: a file of another version.
#[derive(Deserialize)]
struct VersionOnly {
    version: u32,
}

/// The value of the JSON file at `path`, written with `version`; `None` when there is no
/// file, or a file of another version: the shape of the value changed, and the file is
/// downloaded again rather than migrated. The file is read whole once (the cache of the prices
/// weighs several megabytes); its version alone only when its value doesn't read.
pub fn read_versioned_json<T: DeserializeOwned>(
    path: &Path,
    version: u32,
) -> io::Result<Option<T>> {
    let json = match fs::read(path) {
        Ok(json) => json,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let found = match serde_json::from_slice::<Versioned<T>>(&json) {
        Ok(file) if file.version == version => return Ok(Some(file.value)),
        Ok(file) => file.version,
        Err(error) => match serde_json::from_slice::<VersionOnly>(&json) {
            Ok(file) if file.version != version => file.version,
            _ => return Err(io::Error::other(error)),
        },
    };
    log::info!(
        "{} is of version {found}: ignored",
        crate::logging::without_home(path).display()
    );
    Ok(None)
}

/// Writes `value` with `version` to the JSON file `name` of `directory`, atomically.
pub fn write_versioned_json<T: Serialize>(
    directory: &Path,
    name: &str,
    version: u32,
    value: &T,
) -> io::Result<()> {
    let json = serde_json::to_vec(&Versioned { version, value }).map_err(io::Error::other)?;
    write_atomically(directory, name, &json)
}

/// Writes `contents` to the file `name` of `directory`, created if needed. The contents go to
/// a new file synced to the disk first, then renamed, so that a crash while writing never
/// leaves half a file.
pub fn write_atomically(directory: &Path, name: &str, contents: &[u8]) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    let new_path = directory.join(new_file_name(name));
    {
        let mut file = fs::File::create(&new_path)?;
        file.write_all(contents)?;
        // On the disk before the rename makes it the file; closed at the end of the block,
        // before the rename.
        file.sync_all()?;
    }
    fs::rename(new_path, directory.join(name))
}

/// The file the contents of `name` are written to before the rename.
fn new_file_name(name: &str) -> String {
    format!("{name}.new")
}

/// A folder of its own for one test, removed when it ends.
#[cfg(test)]
pub struct TestDirectory(pub std::path::PathBuf);

#[cfg(test)]
impl TestDirectory {
    pub fn new(name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("exile-market-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        Self(path)
    }
}

#[cfg(test)]
impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_file_without_leaving_the_new_one() {
        let directory = TestDirectory::new("write-atomically");

        write_atomically(&directory.0, "a.txt", b"first").expect("written");
        write_atomically(&directory.0, "a.txt", b"second").expect("written again");

        assert_eq!(
            fs::read(directory.0.join("a.txt")).expect("read"),
            b"second"
        );
        assert!(!directory.0.join(new_file_name("a.txt")).exists());
    }

    #[test]
    fn a_versioned_file_reads_back_with_its_version_only() {
        let directory = TestDirectory::new("versioned-json");
        let path = directory.0.join("a.json");

        write_versioned_json(&directory.0, "a.json", 2, &vec![1, 2]).expect("written");

        assert_eq!(
            read_versioned_json::<Vec<i32>>(&path, 2).unwrap(),
            Some(vec![1, 2])
        );
        assert_eq!(read_versioned_json::<Vec<i32>>(&path, 3).unwrap(), None);
        assert_eq!(
            read_versioned_json::<Vec<i32>>(&directory.0.join("missing.json"), 2).unwrap(),
            None
        );
        fs::write(&path, "{").unwrap();
        assert!(read_versioned_json::<Vec<i32>>(&path, 2).is_err());
    }

    #[test]
    fn a_file_of_another_version_is_ignored_whatever_its_value() {
        let directory = TestDirectory::new("versioned-json-other-shape");
        let path = directory.0.join("a.json");

        write_versioned_json(&directory.0, "a.json", 1, &"another shape").expect("written");

        assert_eq!(read_versioned_json::<Vec<i32>>(&path, 2).unwrap(), None);
    }

    #[test]
    fn a_file_of_its_version_whose_value_doesnt_read_is_an_error() {
        let directory = TestDirectory::new("versioned-json-broken-value");
        let path = directory.0.join("a.json");

        write_versioned_json(&directory.0, "a.json", 2, &"another shape").expect("written");

        assert!(read_versioned_json::<Vec<i32>>(&path, 2).is_err());
    }

    /// The core review of 2026-11-10: a failure was never tested. The new file can't be
    /// made here, whatever the rights of the user: a folder has its name.
    #[test]
    fn a_failed_write_leaves_the_file_as_it_was() {
        let directory = TestDirectory::new("write-failed");
        write_atomically(&directory.0, "a.txt", b"first").expect("written");
        fs::create_dir(directory.0.join(new_file_name("a.txt"))).unwrap();

        let written = write_atomically(&directory.0, "a.txt", b"second");

        assert!(written.is_err());
        assert_eq!(fs::read(directory.0.join("a.txt")).expect("read"), b"first");
    }

    #[test]
    fn a_folder_that_cant_be_made_is_an_error() {
        let directory = TestDirectory::new("write-blocked");
        fs::create_dir_all(&directory.0).unwrap();
        fs::write(directory.0.join("blocked"), "").unwrap();

        let written = write_atomically(&directory.0.join("blocked").join("sub"), "a.txt", b"a");

        assert!(written.is_err());
    }
}
