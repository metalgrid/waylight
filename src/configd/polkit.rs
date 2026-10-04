//! polkit authorization for every mutating daemon call, behind an injectable
//! trait so unit tests decide allow/deny without a polkit service.
//!
//! The subject is always the D-Bus caller's *system-bus-name* (its unique
//! name, taken from the message header), never the daemon's own identity:
//! polkit resolves it through the bus and evaluates the real human behind it.

use std::{collections::HashMap, future::Future, pin::Pin};

use zbus::zvariant::{OwnedValue, Value};

/// The one action guarding SetTheme and SetLanguage.
pub const ACTION_ID: &str = "dev.waylight.config.set";
/// org.freedesktop.PolicyKit1.CheckAuthorizationFlags.ALLOW_USER_INTERACTION:
/// an authentication agent may prompt the user.
const ALLOW_USER_INTERACTION: u32 = 1;

pub type AuthFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;

/// Decides whether one caller (by unique bus name) may mutate configuration.
/// The production implementation consults polkit; tests inject the decision.
pub trait Authorizer: Send + Sync + 'static {
    fn authorize<'a>(&'a self, caller: &'a str) -> AuthFuture<'a>;
}

#[zbus::proxy(
    interface = "org.freedesktop.PolicyKit1.Authority",
    default_service = "org.freedesktop.PolicyKit1",
    default_path = "/org/freedesktop/PolicyKit1/Authority"
)]
trait PolicyKitAuthority {
    fn check_authorization(
        &self,
        subject: (String, HashMap<String, Value<'_>>),
        action_id: &str,
        details: HashMap<String, Value<'_>>,
        flags: u32,
        cancellation_id: &str,
    ) -> zbus::Result<(bool, bool, HashMap<String, OwnedValue>)>;
}

/// Production authorizer: one polkit `CheckAuthorization` per call with
/// interactive prompting allowed. A missing or failing polkit service denies
/// (fail closed); a plain "not authorized" denial is reported as such.
pub struct Polkit {
    connection: zbus::Connection,
}

impl Polkit {
    pub fn new(connection: zbus::Connection) -> Self {
        Self { connection }
    }
}

impl Authorizer for Polkit {
    fn authorize<'a>(&'a self, caller: &'a str) -> AuthFuture<'a> {
        Box::pin(async move {
            let proxy = PolicyKitAuthorityProxy::new(&self.connection)
                .await
                .map_err(|error| format!("polkit authority unavailable: {error}"))?;
            let subject = (
                String::from("system-bus-name"),
                HashMap::from([(String::from("name"), Value::from(caller))]),
            );
            let (authorized, _challenge, _details) = proxy
                .check_authorization(
                    subject,
                    ACTION_ID,
                    HashMap::new(),
                    ALLOW_USER_INTERACTION,
                    // No cancellation id: the call is short and idempotent.
                    "",
                )
                .await
                .map_err(|error| format!("polkit check failed: {error}"))?;
            if authorized {
                Ok(())
            } else {
                Err(format!("polkit denied {ACTION_ID} for {caller}"))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test authorizers also prove the trait is object-usable behind the
    /// generic service without an async-trait dependency.
    struct Static(bool);

    impl Authorizer for Static {
        fn authorize<'a>(&'a self, caller: &'a str) -> AuthFuture<'a> {
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

    #[tokio::test]
    async fn injected_decisions_reach_the_caller() {
        assert!(Static(true).authorize(":1.5").await.is_ok());
        assert!(Static(false).authorize(":1.5").await.is_err());
    }

    #[test]
    fn action_id_is_the_locked_one() {
        assert_eq!(ACTION_ID, "dev.waylight.config.set");
    }
}
