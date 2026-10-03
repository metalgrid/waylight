use std::time::Duration;
use zbus::{Connection, Proxy};

pub const ACTIONS: [&str; 3] = ["Suspend", "Reboot", "PowerOff"];
async fn proxy(connection: &Connection) -> zbus::Result<Proxy<'_>> {
    Proxy::new(
        connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
}
pub async fn capabilities() -> u32 {
    tokio::time::timeout(Duration::from_secs(3), async {
        let connection = Connection::system().await.ok()?;
        let proxy = proxy(&connection).await.ok()?;
        let mut result = 0;
        for (i, method) in ["CanSuspend", "CanReboot", "CanPowerOff"]
            .iter()
            .enumerate()
        {
            if proxy
                .call::<_, _, String>(*method, &())
                .await
                .ok()
                .as_deref()
                == Some("yes")
            {
                result |= 1 << i;
            }
        }
        Some(result)
    })
    .await
    .ok()
    .flatten()
    .unwrap_or(0)
}
pub async fn perform(action: usize) -> Result<(), &'static str> {
    let method = *ACTIONS.get(action).ok_or("Unsupported power action.")?;
    tokio::time::timeout(Duration::from_secs(5), async {
        let connection = Connection::system().await?;
        let proxy = proxy(&connection).await?;
        // Never request interactive polkit authorization.
        proxy.call::<_, _, ()>(method, &false).await
    })
    .await
    .map_err(|_| "Power request timed out; its outcome is unknown.")?
    .map_err(|_| "Power request failed; no fallback was attempted.")
}
