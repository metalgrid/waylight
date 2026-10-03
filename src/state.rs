use crate::sessions::valid_id;
use rustix::fs::{AtFlags, Mode, OFlags, Stat, open, openat, renameat, statat, unlinkat};
use std::{
    fs::File,
    io::{self, Read, Write},
    os::fd::OwnedFd,
    path::{Component, Path},
};

fn denied() -> io::Error {
    io::Error::other("unsafe state path or file")
}
fn directory(path: &Path) -> io::Result<OwnedFd> {
    if !path.is_absolute() {
        return Err(denied());
    }
    let uid = rustix::process::geteuid().as_raw();
    let mut fd = open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )?;
    for part in path.components() {
        match part {
            Component::RootDir => continue,
            Component::Normal(name) => {
                fd = openat(
                    &fd,
                    name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )?;
                let stat = rustix::fs::fstat(&fd)?;
                // A root-owned sticky ancestor (e.g. /tmp) cannot replace our
                // private directory. The final directory must still be 0700.
                if (stat.st_uid != 0 && stat.st_uid != uid)
                    || (stat.st_mode & 0o022 != 0
                        && !(stat.st_uid == 0 && stat.st_mode & 0o1000 != 0))
                {
                    return Err(denied());
                }
            }
            _ => return Err(denied()),
        }
    }
    let stat = rustix::fs::fstat(&fd)?;
    if stat.st_uid != uid || stat.st_mode & 0o777 != 0o700 {
        return Err(denied());
    }
    Ok(fd)
}
fn safe_file(s: &Stat) -> bool {
    s.st_uid == rustix::process::geteuid().as_raw()
        && s.st_mode & 0o170777 == 0o100600
        && s.st_nlink == 1
}
fn check_file(fd: &OwnedFd, name: &str) -> io::Result<()> {
    match statat(fd, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(s) if safe_file(&s) => Ok(()),
        Err(rustix::io::Errno::NOENT) => Ok(()),
        _ => Err(denied()),
    }
}
pub fn load(path: &Path) -> io::Result<String> {
    let dir = directory(path)?;
    check_file(&dir, "session")?;
    let file = openat(
        &dir,
        "session",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )?;
    if !safe_file(&rustix::fs::fstat(&file)?) {
        return Err(denied());
    }
    let mut id = String::new();
    File::from(file).take(256).read_to_string(&mut id)?;
    if !valid_id(&id) {
        return Err(denied());
    }
    Ok(id)
}
pub fn save(path: &Path, id: &str) -> io::Result<()> {
    if !valid_id(id) {
        return Err(denied());
    }
    let dir = directory(path)?;
    check_file(&dir, "session")?;
    let name = format!(".session-{}", std::process::id());
    let fd = openat(
        &dir,
        name.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?;
    let result = (|| {
        let mut file = File::from(fd);
        file.write_all(id.as_bytes())?;
        // Advisory preference: atomic replacement, not crash-durable storage.
        // Do not delay scheduled handoff for a disk flush.
        renameat(&dir, name.as_str(), &dir, "session")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = unlinkat(&dir, name.as_str(), AtFlags::empty());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    #[test]
    fn private_atomic_id_only() {
        let d = tempfile::tempdir().unwrap();
        std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        save(d.path(), "weston.desktop").unwrap();
        assert_eq!(load(d.path()).unwrap(), "weston.desktop");
        save(d.path(), "hyprland.desktop").unwrap();
        assert_eq!(load(d.path()).unwrap(), "hyprland.desktop");
        assert!(save(d.path(), "../x.desktop").is_err());
        std::fs::remove_file(d.path().join("session")).unwrap();
        symlink("elsewhere", d.path().join("session")).unwrap();
        assert!(save(d.path(), "weston.desktop").is_err());
        assert!(load(d.path()).is_err());
        assert!(!d.path().join("elsewhere").exists());
        std::fs::set_permissions(d.path(), std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(save(d.path(), "weston.desktop").is_err());
    }
}
