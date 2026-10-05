//! Atomic writes for the system configuration files. The daemon runs as root
//! and owns `/etc/waylight`: the directory is created 0755 when missing and
//! every file lands as root:root 0644 via a temp file in the target directory
//! plus `rename(2)`, so the greeter never observes a half-written file.

use std::{
    fs::{self, File, OpenOptions, Permissions, create_dir_all, rename, set_permissions},
    io::{self, Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process,
};

use super::validate::MAX_JSON_BYTES;

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
    // Any failure after the temp file exists must unlink it: the caller only
    // sees an io::Result and has no handle to clean up with.
    let written = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all());
    if let Err(error) = written {
        drop(file);
        let _ = fs::remove_file(temp);
        return Err(error);
    }
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

/// Reads a file as UTF-8, at most `MAX_JSON_BYTES` bytes. Missing, unreadable,
/// oversized or non-UTF-8 files read as empty, matching the greeter's
/// fail-open handling of absent configuration and Theme.qml's own loadFile cap:
/// a file GetAll would hand out is always one the greeter will apply.
pub fn read(path: &Path) -> String {
    let Ok(file) = fs::File::open(path) else {
        return String::new();
    };
    // Read one byte beyond the cap so an oversized file is detected and
    // dropped whole — the greeter ignores files over the same limit — instead
    // of handing out a truncated document.
    let mut bytes = Vec::new();
    if file
        .take(MAX_JSON_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return String::new();
    }
    if bytes.len() > MAX_JSON_BYTES {
        return String::new();
    }
    String::from_utf8(bytes).unwrap_or_default()
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
    fn commit_removes_the_temp_file_when_the_write_fails() {
        // /dev/full accepts opens but fails every write with ENOSPC, so the
        // mid-write failure path is exercised for real.
        let file = OpenOptions::new()
            .write(true)
            .open("/dev/full")
            .expect("/dev/full exists on Linux");
        let dir = tempfile::tempdir().expect("tempdir");
        // Simulate the temp file write_atomic would already have created.
        let temp = dir.path().join(".theme.json.999.0.tmp");
        fs::write(&temp, "residue").expect("seed temp");
        let target = dir.path().join("theme.json");
        let error = commit(file, &temp, &target, dir.path(), "contents")
            .expect_err("writes to /dev/full fail");
        assert_eq!(error.kind(), io::ErrorKind::StorageFull);
        // The temp residue is gone and the target was never created.
        let entries: Vec<_> = fs::read_dir(dir.path())
            .expect("read_dir")
            .map(|entry| entry.expect("entry").file_name())
            .collect();
        assert!(entries.is_empty(), "temp residue: {entries:?}");
        assert!(!target.exists());
    }

    #[test]
    fn read_caps_the_size_and_fails_open() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("theme.json");
        // Missing file reads as empty (existing behavior).
        assert_eq!(read(&path), "");
        fs::write(&path, "{\"colors\": {}}").expect("write");
        assert_eq!(read(&path), "{\"colors\": {}}");
        // A file at the cap is read; anything larger reads as empty instead of
        // handing the greeter a file it would wholly ignore.
        let exact = format!("{}{}", "x".repeat(MAX_JSON_BYTES - 1), "\n");
        fs::write(&path, &exact).expect("write");
        assert_eq!(read(&path), exact);
        fs::write(&path, "x".repeat(MAX_JSON_BYTES + 1)).expect("write");
        assert_eq!(read(&path), "");
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
