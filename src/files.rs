//! The folders of the application, and the files it writes: never half a file, even after a
//! crash.

use std::fs;
use std::io::{self, Write};
use std::path::Path;

use directories::ProjectDirs;

/// The folders of the application in the user's directories (configuration, cache, log);
/// none when the system gives no home directory.
#[must_use]
pub fn project_directories() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", crate::FOLDER_NAME)
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
}
