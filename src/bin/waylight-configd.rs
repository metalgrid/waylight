//! `waylight-configd` — privileged headless system D-Bus service that owns
//! `/etc/waylight/theme.json` and `/etc/waylight/waylight.json` for the
//! Waylight greeter. D-Bus-activated by systemd; every mutating call is
//! authorized by polkit (`dev.waylight.config.set`). See src/configd/.

use waylight_greeter::configd;

fn main() -> std::process::ExitCode {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Tokio runtime");
    match runtime.block_on(configd::run(configd::store::system_paths())) {
        Ok(()) => 0.into(),
        Err(error) => {
            eprintln!("waylight-configd: {error}");
            1.into()
        }
    }
}
