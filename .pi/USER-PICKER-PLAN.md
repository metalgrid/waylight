# User picker and login controls

> Created: 2026-09-30
> Status: LOCKED
> Scope follows the user's requested interface corrections.

## Goal

Remove the disabled response field that looks like a second login button.
Add a list of users and an Other user option for manual username entry.
Replace generic preview response labels with a simulated Password prompt.
Preserve real PAM prompts, authentication safeguards, session selection, and power controls.
Do not deploy or change services, PAM, accounts, or system configuration.

## Implementation

One worker owns these changes in the current directory. Do not initialize Git.
Use the existing zbus dependency for read-only AccountsService discovery.
AccountsService is installed and active on this machine.
Query cached users and UserName, RealName, SystemAccount, and Locked properties.
Use a finite account count and one overall timeout. Skip invalid or unavailable records.
Exclude system and locked accounts from the picker. PAM remains the authentication authority.
Validate usernames using existing rules, deduplicate, and sort deterministically.
Show display names and usernames as plain text. Do not read avatar files.
Always offer Other user. Discovery failure must preserve manual login.
Preview stays isolated from the system bus and real account discovery. Supply clearly synthetic users.

Publish discovery independently from authentication state. Late results must not replace typed usernames or steal focus.
Keep identity as a username, not a list index. Default to Other user with manual input focus.
Provide idle-only identity cleanup through the backend; do not assign backend state from QML.
Clear responses and stale status when the user intentionally changes identity.
Disable identity changes during authentication. Cancellation restores focus only after confirmed cleanup.
Escape closes the user popup before requesting authentication cancellation.

Show one submit arrow for idle identity entry and one for a secret/visible prompt.
Show one Continue control for information/error acknowledgement.
Hide the response input except during a secret/visible prompt. No inactive duplicate field or orphan arrow.
Use Password in preview and Enter answer for real generic input. Preserve the exact real PAM prompt above the field.
Never assume every secret prompt is a password. Keep empty answers and existing input validation.

## Verification and delivery

Add focused account filtering/failure tests and QML picker/manual/late-arrival/focus/control-visibility tests.
Preserve executable preview-isolation and fake-greetd checks. Do not query real accounts in tests.
Run sh tests/check.sh and inspect application-only images of the updated controls.
Update README.md with cached-user scope and synthetic preview behavior.
Parent checks and independent review follow implementation.
Real PAM, host power actions, VT tests, and installation remain outside scope.

## Review

An architect reviewed this plan and found no remaining design gaps.
Use Main.qml, src/accounts.rs, src/backend.rs, src/controller.rs, src/main.rs, and existing tests.
No new dependencies or authentication protocol redesign are needed.
