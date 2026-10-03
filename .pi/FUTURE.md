# Future work

## Fingerprint support

Use a Rust backend with CXX-Qt for the QML interface.
Use zbus to query fprintd devices and the selected user's enrolled fingerprints over system D-Bus.
Enrollment alone does not prove that greetd's PAM configuration supports fingerprint login.
Keep actual verification in greetd → PAM → pam_fprintd → fprintd.
Do not claim the reader or start verification directly while PAM uses it.
Handle missing devices and permission errors without blocking password login.
Defer this integration until password authentication works.

## Other authentication methods

Evaluate camera recognition and security keys separately.
Do not infer supported methods from PAM message text.
Keep password fallback and recovery access.
