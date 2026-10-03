use freedesktop_desktop_entry::DesktopEntry;
use std::{
    collections::{BTreeSet, VecDeque},
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub command: String,
    pub env: Vec<String>,
}

// Check every traversed node, not just canonical targets. Only root may
// change these after validation; writable HOME/PATH entries are never used.
pub fn trusted(path: &Path) -> bool {
    trusted_with(path, |_, m| {
        m.uid() == 0 && (m.file_type().is_symlink() || m.mode() & 0o022 == 0)
    })
}

pub(crate) fn trusted_with(path: &Path, check: impl Fn(&Path, &fs::Metadata) -> bool) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let mut pending: VecDeque<_> = path
        .components()
        .map(|c| c.as_os_str().to_owned())
        .collect();
    let mut resolved = PathBuf::new();
    let mut links = 0;
    while let Some(part) = pending.pop_front() {
        // Resolve .. only AFTER checking the node it traverses, including any
        // symlink expansion. Relative link targets start at the link's parent.
        if part == ".." {
            resolved.pop();
        } else {
            resolved.push(part);
        }
        let Ok(m) = fs::symlink_metadata(&resolved) else {
            return false;
        };
        if !check(&resolved, &m) {
            return false;
        }
        if m.file_type().is_symlink() {
            links += 1;
            if links > 40 {
                return false; // Linux's traversal bound also rejects cycles.
            }
            let Ok(target) = fs::read_link(&resolved) else {
                return false;
            };
            resolved.pop();
            for component in target.components().rev() {
                pending.push_front(component.as_os_str().to_owned());
            }
        } else if !pending.is_empty() && !m.is_dir() {
            return false;
        }
    }
    true
}

fn resolve(executable: &str) -> Option<PathBuf> {
    let paths = if executable.starts_with('/') {
        vec![PathBuf::from(executable)]
    } else if executable.contains('/') {
        return None;
    } else {
        ["/usr/local/bin", "/usr/bin", "/bin"]
            .map(|p| Path::new(p).join(executable))
            .to_vec()
    };
    paths
        .into_iter()
        .find(|p| trusted(p) && fs::metadata(p).is_ok_and(|m| m.is_file() && m.mode() & 0o111 != 0))
}

pub fn quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

fn safe_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/_-.+:=@,".contains(&c))
}

fn command(exec: &str) -> Option<String> {
    // ponytail: deliberately not a desktop-entry shell parser. Add proper
    // Desktop Entry argument/field-code parsing only when a session needs it.
    if exec
        .bytes()
        .any(|b| b != b' ' && b != b'\t' && !b.is_ascii_graphic())
    {
        return None;
    }
    let mut words: Vec<String> = exec.split_ascii_whitespace().map(str::to_owned).collect();
    if words.is_empty() || !words.iter().all(|w| safe_word(w)) {
        return None;
    }
    words[0] = resolve(&words[0])?.to_str()?.to_owned();
    Some(words.iter().map(|w| quote(w)).collect::<Vec<_>>().join(" "))
}

pub fn valid_id(id: &str) -> bool {
    id.ends_with(".desktop")
        && id.len() <= 255
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-+.".contains(&b))
}

fn parse(path: &Path) -> Option<Session> {
    let id = path.file_name()?.to_str()?.to_owned();
    if !valid_id(&id) {
        return None;
    }
    let m = fs::metadata(path).ok()?;
    if !m.is_file() || m.len() > 64 * 1024 || !trusted(path) {
        return None;
    }
    let entry = DesktopEntry::from_path(path, Some(&[] as &[&str])).ok()?;
    from_entry(&entry, id)
}

fn from_entry(entry: &DesktopEntry, id: String) -> Option<Session> {
    if entry.desktop_entry("Type") != Some("Application") {
        return None;
    }
    for flag in ["Hidden", "NoDisplay", "Terminal"] {
        if !matches!(entry.desktop_entry(flag), None | Some("false")) {
            return None;
        }
    }
    if let Some(exec) = entry.try_exec()
        && (!safe_word(exec) || resolve(exec).is_none())
    {
        return None;
    }
    let name = entry.name::<&str>(&[])?.into_owned();
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return None;
    }
    let command = command(entry.exec()?)?;
    let mut env = vec![
        "XDG_SESSION_TYPE=wayland".into(),
        format!("XDG_SESSION_DESKTOP={}", id.trim_end_matches(".desktop")),
    ];
    if let Some(desktops) = entry.desktop_entry("DesktopNames") {
        let names: Vec<_> = desktops.trim_end_matches(';').split(';').collect();
        if names.iter().any(|n| !safe_word(n)) {
            return None;
        }
        env.push(format!("XDG_CURRENT_DESKTOP={}", names.join(":")));
    }
    Some(Session {
        id,
        name,
        command,
        env,
    })
}

pub fn discover() -> Vec<Session> {
    let mut seen = BTreeSet::new();
    let mut sessions = Vec::new();
    for dir in [
        "/usr/local/share/wayland-sessions",
        "/usr/share/wayland-sessions",
    ] {
        let dir = Path::new(dir);
        if !trusted(dir) {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        let mut paths: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for path in paths {
            // Higher-priority hidden/invalid files mask lower-priority IDs too.
            if let Some(id) = path.file_name().and_then(|n| n.to_str())
                && seen.insert(id.to_owned())
                && let Some(session) = parse(&path)
            {
                sessions.push(session);
            }
        }
    }
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_quoting_and_subset() {
        assert_eq!(quote("a'b $c"), "'a'\\''b $c'");
        assert_eq!(
            command("/usr/bin/true a-b c.desktop").unwrap(),
            "'/usr/bin/true' 'a-b' 'c.desktop'"
        );
        for value in [
            "true 'a b'",
            "true a\\b",
            "true %f",
            "true a;b",
            "true $X",
            "true `id`",
            "true a\0b",
            "true a\nb",
            "true ~/a",
            "true *",
            "./true",
        ] {
            assert!(command(value).is_none(), "{value:?}");
        }
    }
    #[test]
    fn metadata_rejects_unsupported_entries() {
        let mut entry = DesktopEntry::from_appid("test".into());
        entry.add_desktop_entry("Type".into(), "Application".into());
        entry.add_desktop_entry("Exec".into(), "true".into());
        assert!(from_entry(&entry, "test.desktop".into()).is_some());
        for (key, value) in [
            ("Type", "Link"),
            ("Hidden", "true"),
            ("NoDisplay", "true"),
            ("Terminal", "true"),
            ("Terminal", "invalid"),
            ("TryExec", "missing-greeter-test-executable"),
            ("Exec", "true %u"),
            ("DesktopNames", "Bad$Name"),
            ("Name", ""),
        ] {
            let mut changed = entry.clone();
            changed.add_desktop_entry(key.into(), value.into());
            assert!(
                from_entry(&changed, "test.desktop".into()).is_none(),
                "{key}"
            );
        }
    }
    #[test]
    fn rejects_user_paths() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("entry.desktop");
        fs::write(
            &path,
            "[Desktop Entry]\nType=Application\nName=Test\nExec=true\n",
        )
        .unwrap();
        assert!(!trusted(&path));
        assert!(parse(&path).is_none());
        std::os::unix::fs::symlink("/usr/bin/true", dir.path().join("link")).unwrap();
        assert!(!trusted(&dir.path().join("link")));
        assert!(trusted(Path::new("/usr/bin/true")));
    }
    #[test]
    fn symlink_walk_checks_intermediate_links_ancestors_and_cycles() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        fs::create_dir(root.join("safe")).unwrap();
        fs::create_dir(root.join("writable")).unwrap();
        fs::set_permissions(root.join("writable"), fs::Permissions::from_mode(0o777)).unwrap();
        fs::write(root.join("target"), "trusted target").unwrap();
        symlink("../target", root.join("safe/bridge")).unwrap();
        symlink("safe/bridge", root.join("trusted-link")).unwrap();
        symlink(root.join("trusted-link"), root.join("absolute-link")).unwrap();
        symlink("../target", root.join("writable/bridge")).unwrap();
        symlink("writable/bridge", root.join("unsafe-link")).unwrap();
        symlink("target", root.join("unowned-link")).unwrap();
        symlink("unowned-link", root.join("owner-bypass")).unwrap();
        symlink("writable/../target", root.join("dotdot-bypass")).unwrap();
        symlink("cycle-b", root.join("cycle-a")).unwrap();
        symlink("cycle-a", root.join("cycle-b")).unwrap();
        symlink("self-cycle", root.join("self-cycle")).unwrap();

        // Model root ownership inside a private fixture without sudo/chown.
        // Only its real ancestors (e.g. /tmp) bypass the production mode check;
        // unowned-link models a non-root-owned symlink, whose mode is irrelevant.
        let uid = fs::metadata(root).unwrap().uid();
        let check = |path: &Path, m: &fs::Metadata| {
            (path != root && root.starts_with(path))
                || (m.uid() == uid
                    && path != root.join("unowned-link")
                    && (m.file_type().is_symlink() || m.mode() & 0o022 == 0))
        };
        for path in ["target", "trusted-link", "absolute-link", "safe/../target"] {
            assert!(trusted_with(&root.join(path), check), "{path}");
        }
        for path in [
            "unsafe-link",
            "owner-bypass",
            "dotdot-bypass",
            "writable/../target",
            "cycle-a",
            "self-cycle",
        ] {
            assert!(!trusted_with(&root.join(path), check), "{path}");
        }
        // Trusted distribution symlinks remain usable with root-only policy.
        assert!(trusted(Path::new("/bin/true")));
    }
    #[test]
    fn installed_sessions_are_supported() {
        let sessions = discover();
        for id in [
            "hyprland.desktop",
            "hyprland-uwsm.desktop",
            "weston.desktop",
        ] {
            if Path::new("/usr/share/wayland-sessions").join(id).exists() {
                assert!(sessions.iter().any(|s| s.id == id), "missing {id}");
            }
        }
        for s in sessions {
            assert!(s.env.iter().all(|v| v.starts_with("XDG_SESSION_") || v.starts_with("XDG_CURRENT_DESKTOP=")));
        }
    }
}
