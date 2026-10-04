//! `waylight-configd`: the privileged headless system D-Bus service that owns
//! `/etc/waylight/theme.json` and `/etc/waylight/waylight.json`.
//!
//! Bus name `dev.waylight.Config`, object `/dev/waylight/Config`, interface
//! `dev.waylight.Config1`. `GetAll` is read-only; `SetTheme` and `SetLanguage`
//! validate structurally, authorize the caller via polkit (action
//! `dev.waylight.config.set`, injected behind `Authorizer`) and then write
//! atomically. The daemon never touches greetd, PAM, users, services or the
//! network.

pub mod polkit;
pub mod store;
pub mod validate;

use zbus::{DBusError, interface, message::Header};

pub use polkit::{Authorizer, Polkit};
pub use store::Paths;
pub use validate::MAX_JSON_BYTES;

pub const BUS_NAME: &str = "dev.waylight.Config";
pub const OBJECT_PATH: &str = "/dev/waylight/Config";
pub const INTERFACE_NAME: &str = "dev.waylight.Config1";

/// D-Bus errors reported to clients. The error names are
/// `dev.waylight.Config1.Error.<Variant>`.
#[derive(Debug, DBusError)]
#[zbus(prefix = "dev.waylight.Config1.Error")]
pub enum ConfigError {
    #[zbus(error)]
    ZBus(zbus::Error),
    /// polkit refused the caller, or could not be consulted. Nothing was
    /// written.
    Denied,
    /// The submitted theme.json failed structural validation.
    InvalidTheme(String),
    /// The submitted language is not one of the supported codes.
    InvalidLanguage(String),
    /// Reading or writing the system files failed. Nothing was changed
    /// atomically (writes are temp+rename).
    Io(String),
}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub struct ConfigService<A: Authorizer> {
    authorizer: A,
    paths: Paths,
}

impl<A: Authorizer> ConfigService<A> {
    pub fn new(authorizer: A, paths: Paths) -> Self {
        Self { authorizer, paths }
    }

    /// Raw file contents plus the paths a subsequent Apply would write. The
    /// same data as GetAll, for tests and the run() wiring.
    pub fn read_all(&self) -> (String, String) {
        (
            store::read(&self.paths.theme),
            store::read(&self.paths.config),
        )
    }

    /// Validates and writes theme.json.
    pub fn store_theme(&self, json: &str) -> Result<(), ConfigError> {
        validate::theme(json).map_err(ConfigError::InvalidTheme)?;
        store::write_atomic(&self.paths.theme, json)?;
        Ok(())
    }

    /// Validates the language code and writes waylight.json as
    /// `{"language": "<code>"}`.
    pub fn store_language(&self, code: &str) -> Result<(), ConfigError> {
        validate::language(code).map_err(ConfigError::InvalidLanguage)?;
        let json = serde_json::json!({ "language": code }).to_string();
        store::write_atomic(&self.paths.config, &json)?;
        Ok(())
    }

    /// The shared polkit gate for mutating methods. A caller without a bus
    /// identity (never possible over the bus) is denied like any other.
    async fn authorize_caller(&self, caller: Option<&str>) -> Result<(), ConfigError> {
        let Some(caller) = caller else {
            return Err(ConfigError::Denied);
        };
        self.authorizer
            .authorize(caller)
            .await
            .map_err(|_| ConfigError::Denied)
    }
}

#[interface(name = "dev.waylight.Config1")]
impl<A: Authorizer> ConfigService<A> {
    // (theme, config, theme_path, config_path): raw file contents (empty when
    // a file does not exist yet) and the system paths Apply writes to.
    async fn get_all(&self) -> Result<(String, String, String, String), ConfigError> {
        let (theme, config) = self.read_all();
        Ok((
            theme,
            config,
            self.paths.theme.to_string_lossy().into_owned(),
            self.paths.config.to_string_lossy().into_owned(),
        ))
    }

    async fn set_theme(
        &self,
        #[zbus(header)] header: Header<'_>,
        json: String,
    ) -> Result<(), ConfigError> {
        self.authorize_caller(
            header
                .sender()
                .map(zbus::names::UniqueName::to_string)
                .as_deref(),
        )
        .await?;
        self.store_theme(&json)
    }

    async fn set_language(
        &self,
        #[zbus(header)] header: Header<'_>,
        code: String,
    ) -> Result<(), ConfigError> {
        self.authorize_caller(
            header
                .sender()
                .map(zbus::names::UniqueName::to_string)
                .as_deref(),
        )
        .await?;
        self.store_language(&code)
    }
}

/// Serves forever on the system bus until SIGTERM/SIGINT. D-Bus-activated by
/// systemd (Type=dbus): the process starts on first use, publishes the name
/// and stays resident.
pub async fn run(paths: Paths) -> Result<(), Box<dyn std::error::Error>> {
    let connection = zbus::Connection::system().await?;
    let service = ConfigService::new(Polkit::new(connection.clone()), paths);
    connection.object_server().at(OBJECT_PATH, service).await?;
    connection.request_name(BUS_NAME).await?;
    let (Ok(mut terminate), Ok(mut interrupt)) = (
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()),
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()),
    ) else {
        return Err("cannot install signal handlers".into());
    };
    tokio::select! {
        _ = terminate.recv() => {}
        _ = interrupt.recv() => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct Static(bool);

    impl Authorizer for Static {
        fn authorize<'a>(&'a self, caller: &'a str) -> polkit::AuthFuture<'a> {
            let allowed = self.0;
            Box::pin(async move {
                if allowed {
                    Ok(())
                } else {
                    Err(format!("denied: {caller}"))
                }
            })
        }
    }

    fn temp_paths(tag: &str) -> (tempfile::TempDir, Paths) {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = Paths::new(
            dir.path().join(tag),
            dir.path().join(tag).join("theme.json"),
            dir.path().join(tag).join("waylight.json"),
        );
        (dir, paths)
    }

    // Direct store_layer checks (validation + write + denial) run against the
    // same methods the zbus interface calls; the interface wrapper only adds
    // header/plumbing that needs a live bus.
    #[test]
    fn allowed_theme_write_passes_validation_and_lands_on_disk() {
        let (_dir, paths) = temp_paths("allowed");
        let service = ConfigService::new(Static(true), paths.clone());
        service
            .store_theme(r##"{"colors": {"accent": "#d9e2ff"}}"##)
            .expect("store");
        let (theme, config) = service.read_all();
        assert_eq!(theme, r##"{"colors": {"accent": "#d9e2ff"}}"##);
        assert_eq!(config, "");
    }

    #[tokio::test]
    async fn denied_authorization_refuses_without_writing() {
        let (_dir, paths) = temp_paths("denied");
        let service = ConfigService::new(Static(false), paths.clone());
        // The exact call sequence of the interface methods, minus the header.
        let outcome = service
            .authorize_caller(Some(":1.42"))
            .await
            .and_then(|()| service.store_theme("{}"));
        assert!(matches!(outcome, Err(ConfigError::Denied)));
        let outcome = service
            .authorize_caller(Some(":1.42"))
            .await
            .and_then(|()| service.store_language("ar"));
        assert!(matches!(outcome, Err(ConfigError::Denied)));
        // Nothing was written: no files, no directory.
        assert!(!paths.directory.exists());
    }

    #[tokio::test]
    async fn missing_caller_identity_is_denied() {
        let (_dir, paths) = temp_paths("nocaller");
        let service = ConfigService::new(Static(true), paths.clone());
        let outcome = service
            .authorize_caller(None)
            .await
            .and_then(|()| service.store_theme("{}"));
        assert!(matches!(outcome, Err(ConfigError::Denied)));
        assert!(!paths.directory.exists());
    }

    #[tokio::test]
    async fn invalid_theme_is_rejected_before_any_write() {
        let (_dir, paths) = temp_paths("invalid");
        let service = ConfigService::new(Static(true), paths.clone());
        let outcome = service
            .authorize_caller(Some(":1.7"))
            .await
            .and_then(|()| service.store_theme(r#"{"preset": "midnight"}"#));
        assert!(matches!(outcome, Err(ConfigError::InvalidTheme(_))));
        assert!(!paths.theme.exists());
    }

    #[tokio::test]
    async fn language_write_produces_waylight_json() {
        let (_dir, paths) = temp_paths("lang");
        let service = ConfigService::new(Static(true), paths.clone());
        service
            .authorize_caller(Some(":1.9"))
            .await
            .and_then(|()| service.store_language("ur"))
            .expect("store");
        assert_eq!(store::read(&paths.config), "{\"language\":\"ur\"}");
    }

    #[test]
    fn get_all_shape_matches_the_locked_api() {
        let dir = tempfile::tempdir().expect("tempdir");
        let paths = Paths::new(
            dir.path().to_path_buf(),
            PathBuf::from("/etc/waylight/theme.json"),
            PathBuf::from("/etc/waylight/waylight.json"),
        );
        let service = ConfigService::new(Static(true), paths);
        let (theme, config) = service.read_all();
        assert_eq!((theme.as_str(), config.as_str()), ("", ""));
        assert_eq!(
            service.paths.theme,
            PathBuf::from("/etc/waylight/theme.json")
        );
        assert_eq!(
            service.paths.config,
            PathBuf::from("/etc/waylight/waylight.json")
        );
    }
}
