use crate::sessions;
use cxx_qt_lib::{QByteArray, QImage};
use std::{
    fs::{self, File},
    future::Future,
    io::Read,
    path::{Component, Path},
    time::Duration,
};
use zbus::{Connection, Proxy, proxy::CacheProperties, zvariant::OwnedObjectPath};

const MAX_ACCOUNTS: usize = 64;
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(3);
const ICON_CACHE: &str = "/var/lib/AccountsService/icons";
const MAX_ICON_BYTES: usize = 256 * 1024;
const MAX_ICON_DIMENSION: u32 = 256;

// Inspect the PNG header before invoking Qt's decoder. No custom image decoder,
// external SVG, filename handed to Qt, or unbounded decoded pixel dimensions.
fn png_data_url(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 33
        || bytes.len() > MAX_ICON_BYTES
        || &bytes[..16] != b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR"
    {
        return None;
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
    let height = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
    if !(1..=MAX_ICON_DIMENSION).contains(&width) || !(1..=MAX_ICON_DIMENSION).contains(&height) {
        return None;
    }
    let image = QImage::from_data(bytes, Some("PNG"))?;
    if image.width() != width as i32 || image.height() != height as i32 {
        return None;
    }
    let encoded = QByteArray::from(bytes).to_base64(Default::default());
    Some(format!("data:image/png;base64,{encoded}"))
}

fn cache_picture(path: &Path, cache: &Path, trusted: impl Fn(&Path) -> bool) -> Option<String> {
    // Deliberately accept only immediate cache entries. Reject relative paths,
    // URLs, dot traversal and links escaping the cache before reading any bytes.
    let relative = path.strip_prefix(cache).ok()?;
    let mut parts = relative.components();
    if !cache.is_absolute()
        || !matches!(parts.next(), Some(Component::Normal(_)))
        || parts.next().is_some()
        || !trusted(path)
        || fs::canonicalize(path).ok()?.parent() != Some(cache)
    {
        return None;
    }
    // No-follow/nonblocking prevents a substituted leaf symlink/FIFO from
    // redirecting or blocking the open. Trusted ancestors are root-controlled.
    let fd = rustix::fs::open(
        path,
        rustix::fs::OFlags::RDONLY
            | rustix::fs::OFlags::CLOEXEC
            | rustix::fs::OFlags::NOFOLLOW
            | rustix::fs::OFlags::NONBLOCK,
        rustix::fs::Mode::empty(),
    )
    .ok()?;
    let file = File::from(fd);
    let metadata = file.metadata().ok()?;
    if !metadata.is_file() || metadata.len() > MAX_ICON_BYTES as u64 {
        return None;
    }
    let mut bytes = Vec::new();
    file.take((MAX_ICON_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .ok()?;
    // QML receives only these already-checked bytes, never a path it can reopen.
    png_data_url(&bytes)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Account {
    pub username: String,
    pub name: String,
    pub picture: String,
}

pub fn valid_username(username: &str) -> bool {
    !username.is_empty() && username.len() <= 256 && !username.chars().any(char::is_control)
}

struct Record {
    username: String,
    name: String,
    system: bool,
    locked: bool,
    icon_file: String,
}

// The same bounded read/filter path is exercised with injected records, never the host bus.
async fn collect<T, F, R>(paths: Vec<T>, mut read: F) -> Vec<Account>
where
    F: FnMut(T) -> R,
    R: Future<Output = Option<Record>>,
{
    let mut accounts = Vec::new();
    for path in paths.into_iter().take(MAX_ACCOUNTS) {
        let Some(mut record) = read(path).await else {
            continue;
        };
        if record.system
            || record.locked
            || !valid_username(&record.username)
            || record.name.len() > 256
            || record.name.chars().any(char::is_control)
        {
            continue;
        }
        if record.name.trim().is_empty() {
            record.name = record.username.clone();
        }
        accounts.push(record);
    }
    accounts.sort_by(|a, b| (&a.username, &a.name).cmp(&(&b.username, &b.name)));
    accounts.dedup_by(|a, b| a.username == b.username);
    accounts
        .into_iter()
        .enumerate()
        .map(|(index, record)| Account {
            name: record.name,
            username: record.username,
            // Only the five displayed tiles need pictures. Empty/missing IconFile
            // never discards an otherwise valid account.
            picture: if index < 5 {
                cache_picture(
                    Path::new(&record.icon_file),
                    Path::new(ICON_CACHE),
                    sessions::trusted,
                )
                .unwrap_or_default()
            } else {
                String::new()
            },
        })
        .collect()
}

async fn bounded(
    discovery: impl Future<Output = zbus::Result<Vec<Account>>>,
    timeout: Duration,
) -> Vec<Account> {
    tokio::time::timeout(timeout, discovery)
        .await
        .ok()
        .and_then(Result::ok)
        .unwrap_or_default()
}

pub async fn discover() -> Vec<Account> {
    bounded(
        async {
            let connection = Connection::system().await?;
            let manager = Proxy::new(
                &connection,
                "org.freedesktop.Accounts",
                "/org/freedesktop/Accounts",
                "org.freedesktop.Accounts",
            )
            .await?;
            let paths: Vec<OwnedObjectPath> = manager.call("ListCachedUsers", &()).await?;
            Ok(collect(paths, |path| async {
                let proxy = zbus::proxy::Builder::<Proxy<'_>>::new(&connection)
                    .destination("org.freedesktop.Accounts")
                    .ok()?
                    .path(path)
                    .ok()?
                    .interface("org.freedesktop.Accounts.User")
                    .ok()?
                    .cache_properties(CacheProperties::No)
                    .build()
                    .await
                    .ok()?;
                Some(Record {
                    username: proxy.get_property("UserName").await.ok()?,
                    name: proxy.get_property("RealName").await.ok()?,
                    system: proxy.get_property("SystemAccount").await.ok()?,
                    locked: proxy.get_property("Locked").await.ok()?,
                    icon_file: proxy.get_property("IconFile").await.unwrap_or_default(),
                })
            })
            .await)
        },
        DISCOVERY_TIMEOUT,
    )
    .await
}

pub fn preview() -> Vec<Account> {
    [
        (
            "alex",
            "Alex",
            include_bytes!("../icons/demo-alex.png").as_slice(),
        ),
        (
            "sam",
            "Sam",
            include_bytes!("../icons/demo-sam.png").as_slice(),
        ),
        (
            "lee",
            "Lee",
            include_bytes!("../icons/demo-lee.png").as_slice(),
        ),
        (
            "robin",
            "Robin",
            include_bytes!("../icons/demo-robin.png").as_slice(),
        ),
        (
            "jules",
            "Jules",
            include_bytes!("../icons/demo-jules.png").as_slice(),
        ),
    ]
    .into_iter()
    .map(|(username, name, bytes)| Account {
        username: format!("demo-{username}"),
        name: format!("{name} (demo)"),
        picture: png_data_url(bytes).unwrap_or_default(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn record(username: &str, name: &str) -> Record {
        Record {
            username: username.into(),
            name: name.into(),
            system: false,
            locked: false,
            icon_file: String::new(),
        }
    }

    #[tokio::test]
    async fn filters_invalid_locked_system_and_unavailable_records_deterministically() {
        let records = vec![
            Some(record("z-user", "")),
            Some(record("a-user", "Zed")),
            Some(record("a-user", "<b>Literal</b>")),
            None,
            Some(Record {
                system: true,
                ..record("system", "System")
            }),
            Some(Record {
                locked: true,
                ..record("locked", "Locked")
            }),
            Some(record("", "Empty")),
            Some(record("bad\0user", "NUL")),
            Some(record(&"x".repeat(257), "Long")),
            Some(record("bad-name", "a\nb")),
            Some(record("long-name", &"x".repeat(257))),
        ];
        let accounts = collect(records, std::future::ready).await;
        assert_eq!(
            accounts,
            vec![
                Account {
                    username: "a-user".into(),
                    name: "<b>Literal</b>".into(),
                    picture: String::new(),
                },
                Account {
                    username: "z-user".into(),
                    name: "z-user".into(),
                    picture: String::new(),
                },
            ]
        );
    }

    #[tokio::test]
    async fn absent_or_unsupported_icon_does_not_discard_account() {
        let accounts = collect(
            vec![
                Some(record("absent", "")),
                Some(Record {
                    icon_file: "https://example.invalid/icon.png".into(),
                    ..record("url", "URL")
                }),
                Some(Record {
                    icon_file: "/home/synthetic-user/.face".into(),
                    ..record("home", "Home")
                }),
            ],
            std::future::ready,
        )
        .await;
        assert_eq!(accounts.len(), 3);
        assert!(accounts.iter().all(|account| account.picture.is_empty()));
        assert_eq!(accounts[0].name, "absent");
    }

    #[test]
    fn png_bounds_decode_and_preview_pictures() {
        let png = include_bytes!("../icons/demo-alex.png");
        let url = png_data_url(png).unwrap();
        assert!(url.starts_with("data:image/png;base64,iVBORw0KGgo"));
        let encoded = QByteArray::from(
            url.strip_prefix("data:image/png;base64,")
                .unwrap()
                .as_bytes(),
        );
        assert_eq!(
            QByteArray::from_base64_encoding(&encoded, Default::default())
                .unwrap()
                .as_slice(),
            png
        );
        for invalid in [b"<svg></svg>".as_slice(), b"GIF89a", &png[..32], &png[..40]] {
            assert!(png_data_url(invalid).is_none());
        }
        for dimension in [0, MAX_ICON_DIMENSION + 1, u32::MAX] {
            for offset in [16, 20] {
                let mut invalid = png.to_vec();
                invalid[offset..offset + 4].copy_from_slice(&dimension.to_be_bytes());
                assert!(png_data_url(&invalid).is_none());
            }
        }
        let mut oversized = png.to_vec();
        oversized.resize(MAX_ICON_BYTES + 1, 0);
        assert!(png_data_url(&oversized).is_none());
        let mut corrupt = png.to_vec();
        corrupt[29] ^= 1; // Invalid IHDR CRC, not just a plausible PNG header.
        assert!(png_data_url(&corrupt).is_none());
        let demos = preview();
        assert_eq!(demos.len(), 5);
        assert!(
            demos
                .iter()
                .all(|account| account.picture.starts_with("data:image/png;base64,"))
        );
        assert!(
            demos
                .windows(2)
                .all(|pair| pair[0].picture != pair[1].picture)
        );
    }

    #[test]
    fn cache_policy_checks_components_regular_files_and_bounded_bytes_without_root() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path();
        let cache = root.join("icons");
        fs::create_dir(&cache).unwrap();
        let png = include_bytes!("../icons/demo-alex.png");
        fs::write(cache.join("good"), png).unwrap();
        fs::write(cache.join("writable"), png).unwrap();
        fs::set_permissions(cache.join("writable"), fs::Permissions::from_mode(0o666)).unwrap();
        fs::write(cache.join("unowned"), png).unwrap();
        fs::write(root.join("outside"), png).unwrap();
        fs::write(cache.join("corrupt"), b"not a picture").unwrap();
        fs::write(cache.join("oversized"), vec![0; MAX_ICON_BYTES + 1]).unwrap();
        fs::create_dir(cache.join("directory")).unwrap();
        symlink(root.join("outside"), cache.join("escape")).unwrap();
        symlink("good", cache.join("link")).unwrap();
        rustix::fs::mknodat(
            rustix::fs::CWD,
            cache.join("fifo"),
            rustix::fs::FileType::Fifo,
            rustix::fs::Mode::RUSR,
            0,
        )
        .unwrap();
        let uid = fs::metadata(root).unwrap().uid();
        // Model root-controlled ownership in our private fixture. The real
        // component walker still checks every node; /tmp ancestors alone exempt.
        let trusted = |path: &Path| {
            sessions::trusted_with(path, |node, metadata| {
                (node != root && root.starts_with(node))
                    || (metadata.uid() == uid
                        && node != cache.join("unowned")
                        && (metadata.file_type().is_symlink() || metadata.mode() & 0o022 == 0))
            })
        };
        assert!(cache_picture(&cache.join("good"), &cache, trusted).is_some());
        for name in [
            "missing",
            "writable",
            "unowned",
            "corrupt",
            "oversized",
            "directory",
            "escape",
            "link",
            "fifo",
            "../outside",
        ] {
            assert!(
                cache_picture(&cache.join(name), &cache, trusted).is_none(),
                "{name}"
            );
        }
        for path in [
            root.join("outside"),
            Path::new("relative").to_owned(),
            Path::new("https://example.invalid/avatar.png").to_owned(),
        ] {
            assert!(cache_picture(&path, &cache, trusted).is_none());
        }
        assert!(cache_picture(&cache.join("good"), &cache, |_| false).is_none());
        // Production ownership rejects the same user-owned fixture.
        assert!(cache_picture(&cache.join("good"), &cache, sessions::trusted).is_none());
        fs::set_permissions(&cache, fs::Permissions::from_mode(0o777)).unwrap();
        assert!(cache_picture(&cache.join("good"), &cache, trusted).is_none());
    }

    #[tokio::test]
    async fn count_errors_and_entire_discovery_are_bounded() {
        let reads = Cell::new(0);
        let accounts = collect((0..MAX_ACCOUNTS + 10).collect(), |i| {
            reads.set(reads.get() + 1);
            std::future::ready(Some(record(&format!("user-{i}"), "Demo")))
        })
        .await;
        assert_eq!(reads.get(), MAX_ACCOUNTS);
        assert_eq!(accounts.len(), MAX_ACCOUNTS);
        let timeout = Duration::from_millis(10);
        assert!(
            bounded(
                async { Err(zbus::Error::Failure("unavailable".into())) },
                timeout
            )
            .await
            .is_empty()
        );
        // A stalled connect/list and a stalled record both share one overall deadline.
        assert!(bounded(std::future::pending(), timeout).await.is_empty());
        assert!(
            bounded(
                async { Ok(collect(vec![0], |_| std::future::pending()).await) },
                timeout
            )
            .await
            .is_empty()
        );
    }
}
