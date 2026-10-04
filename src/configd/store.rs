//! Atomic writes for the system configuration files. The daemon runs as root
//! and owns `/etc/waylight`: the directory is created 0755 when missing and
//! every file lands as root:root 0644 via a temp file in the target directory
//! plus `rename(2)`, so the greeter never observes a half-written file.

use std::{
    fs::{self, File, OpenOptions, Permissions, create_dir_all, rename, set_permissions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process,
};

pub const DIRECTORY_MODE: u32 = 0o755;
pub const FILE_MODE: u32 = 0o644;

fn denied_directory() -> io::Error {
    io::Error::other("target has no parent directory")
}

/// Creates `directory` (and parents) 0755 when it does not exist yet. An
/// existing directory is never re-permissioned.
pub fn ensure_directory(directory: &Path) -> io::Result<()> {
    if directory.exists() {
        return Ok(());
    }
    create_dir_all(directory)?;
    set_permissions(directory, Permissions::from_mode(DIRECTORY_MODE))
}

/// Writes `contents` to `path` atomically: a fresh temp file (0644) in the
/// target directory, fsync, then rename. On failure the temp file is removed
/// and the target is untouched.
pub fn write_atomic(path: &Path, contents: &str) -> io::Result<()> {
    let directory = path.parent().ok_or_else(denied_directory)?;
    ensure_directory(directory)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(denied_directory)?;
    // The pid plus an attempt counter keeps concurrent SetTheme/SetLanguage
    // calls (the object server handles methods concurrently) out of each
    // other's way; create_new makes any residue an error, never an overwrite.
    for attempt in 0..64u32 {
        let temp = directory.join(format!(".{name}.{}.{}.tmp", process::id(), attempt));
        let file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(FILE_MODE)
            .open(&temp)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        return commit(file, &temp, path, directory, contents);
    }
    Err(io::Error::other("no usable temporary file name"))
}

fn commit(
    file: File,
    temp: &Path,
    path: &Path,
    directory: &Path,
    contents: &str,
) -> io::Result<()> {
    let mut file = file;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    drop(file);
    if let Err(error) = rename(temp, path) {
        let _ = fs::remove_file(temp);
        return Err(error);
    }
    sync_directory(directory);
    Ok(())
}

/// Best-effort durability for the rename itself; a failed directory fsync
/// never fails an otherwise successful write.
fn sync_directory(directory: &Path) {
    let opened = rustix::fs::open(
        directory,
        rustix::fs::OFlags::RDONLY | rustix::fs::OFlags::DIRECTORY,
        rustix::fs::Mode::empty(),
    );
    if let Ok(fd) = opened {
        let _ = rustix::fs::fsync(&fd);
    }
}

/// Reads a file as UTF-8; missing or unreadable files read as empty, matching
/// the greeter's fail-open handling of absent configuration.
pub fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

/// The paths the packaged daemon manages. Tests inject their own.
pub fn system_paths() -> Paths {
    let directory = PathBuf::from("/etc/waylight");
    Paths::new(
        directory.clone(),
        directory.join("theme.json"),
        directory.join("waylight.json"),
    )
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub directory: PathBuf,
    pub theme: PathBuf,
    pub config: PathBuf,
}

impl Paths {
    pub fn new(directory: PathBuf, theme: PathBuf, config: PathBuf) -> Self {
        Self {
            directory,
            theme,
            config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    #[test]
    fn write_atomic_lands_0644_without_temp_residue() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("theme.json");
        write_atomic(&target, r#"{"colors": {}}"#).expect("write");
        let metadata = fs::metadata(&target).expect("metadata");
        assert_eq!(metadata.mode() & 0o777, FILE_MODE);
        assert_eq!(
            fs::read_to_string(&target).expect("read"),
            r#"{"colors": {}}"#
        );
        // Only the target file remains.
        let entries: Vec<_> = fs::read_dir(dir.path())
            .expect("read_dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert_eq!(entries, vec![std::ffi::OsString::from("theme.json")]);
    }

    #[test]
    fn write_atomic_replaces_existing_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("waylight.json");
        write_atomic(&target, "{\"language\": \"en\"}").expect("first write");
        write_atomic(&target, "{\"language\": \"ur\"}").expect("second write");
        assert_eq!(
            fs::read_to_string(&target).expect("read"),
            "{\"language\": \"ur\"}"
        );
    }

    #[test]
    fn write_atomic_creates_the_directory_0755_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("waylight");
        let target = nested.join("theme.json");
        write_atomic(&target, "{}").expect("write");
        let metadata = fs::metadata(&nested).expect("metadata");
        assert_eq!(metadata.mode() & 0o777, DIRECTORY_MODE);
        assert_eq!(fs::read_to_string(&target).expect("read"), "{}");
    }

    #[test]
    fn write_atomic_keeps_an_existing_directory_unrepermissioned() {
        let dir = tempfile::tempdir().expect("tempdir");
        let nested = dir.path().join("waylight");
        create_dir_all(&nested).expect("mkdir");
        set_permissions(&nested, Permissions::from_mode(0o700)).expect("chmod");
        let target = nested.join("theme.json");
        write_atomic(&target, "{}").expect("write");
        assert_eq!(
            fs::metadata(&nested).expect("metadata").mode() & 0o777,
            0o700
        );
    }

    #[test]
    fn write_atomic_never_touches_the_target_on_failure() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("theme.json");
        write_atomic(&target, "original").expect("write");
        // A parent path component that is a regular file makes every open and
        // mkdir inside write_atomic fail; the existing target must survive.
        let file_as_dir = dir.path().join("blocker");
        fs::write(&file_as_dir, "blocker").expect("write blocker");
        let broken = file_as_dir.join("theme.json");
        assert!(write_atomic(&broken, "x").is_err());
        assert_eq!(fs::read_to_string(&target).expect("read"), "original");
    }

    #[test]
    fn read_fails_open_to_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(read(&dir.path().join("missing.json")), "");
        let file = dir.path().join("present.json");
        fs::write(&file, b"content").expect("write");
        assert_eq!(read(&file), "content");
    }

    #[test]
    fn system_paths_live_under_etc_waylight() {
        let paths = system_paths();
        assert_eq!(paths.directory, PathBuf::from("/etc/waylight"));
        assert_eq!(paths.theme, PathBuf::from("/etc/waylight/theme.json"));
        assert_eq!(paths.config, PathBuf::from("/etc/waylight/waylight.json"));
    }
}
