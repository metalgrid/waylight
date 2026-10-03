# VM desktop progress

Derived from locked `VM-DESKTOP-PLAN.md`; updated 2026-10-01.

- [x] Verify pinned guest identity and sudo/SSH recovery; read app runtime/licensing requirements.
- [x] Full guest update and desktop/runtime packages (Noctalia 5.2.0; one kernel-required guest reboot).
- [x] Build locked host release (Qt 6.11.2); matching guest Qt, ldd/help and actual GUI verified.
- [x] Back up and configure greetd and arch's Hyprland/Noctalia session.
- [x] Real GUI login performed by the human; independently confirmed PAM arch session, Hyprland/Noctalia, guest screenshot, correct environment, no config errors, and saved session preference.
- [x] Initial logout returned to greeter just before parent relayed human use; paused on receipt. User then explicitly authorized continued testing.
- [x] Independent agent cycle: observed Password prompt; arch/arch login at 11:43:57 UTC; confirmed Hyprland + Noctalia, correct environment, terminal `whoami`, and visible launcher.
- [x] Final authorized logout 11:45:11 UTC; verified new idle greeter, no old desktop processes, and SSH active at 11:45:22 UTC.
- [x] Record versions, evidence, recovery, and observed screen in `vm/README.md`.

Progress: all installation and practical GUI checks complete; no application code changes. Human and independent agent real logins both confirmed. Final screen: idle Waylight greeter (Hyprland selected), VM running. Evidence: `vm/evidence/agent-login.txt`, `agent-desktop-ready.txt`, `agent-terminal-command.png`, `agent-launcher.png`, `agent-logout.txt`, `final-greeter.png`. No power tests. User may subsequently log in; do not interrupt without coordination.

Host system changes forbidden; no Git operations. Guest identity: `1ffb34f196844df1aec96892ca0893a1`.
