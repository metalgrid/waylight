# User tiles

> Created: 2026-09-30
> Status: LOCKED
> Implements the user's requested replacement for the dropdown.

## Scope

Replace the user dropdown with a horizontal row of up to five user tiles.
Show a picture and display name on each tile. Keep the username available for accessibility and disambiguation.
Add a More… tile that opens manual username input, including when discovery returns no users.
Keep one submit control. Selecting a tile chooses identity without starting authentication automatically.
Do not change authentication, session selection, power policy, or installed system configuration.

Use AccountsService IconFile for real account pictures. Treat it as untrusted file input.
Accept only bounded regular local PNG files from the root-controlled AccountsService icon cache.
Validate path ownership and all path components using existing helpers. No arbitrary URL or home-directory image loading.
Use bounded reads and explicit image sizing. Invalid, missing, unsupported, or unreadable images use the default avatar.
Do not give QML unchecked paths. Prefer checked PNG bytes as a bounded data URL if this avoids path races.
Use original bundled demo pictures in isolated preview. Preview must not read host accounts, pictures, or system D-Bus.
A missing IconFile property must not hide an otherwise valid user.

Default to manual entry when no identity is selected. Preserve manual text and focus during late discovery.
Disable identity changes during authentication. Preserve stable username identity and cancellation behavior.
Fit five tiles and More… at the supported minimum width, with readable names and keyboard navigation.
Use pictures at a fixed display size, plain text labels, and visible selection/focus indication.

## Checks

Extend account-picture filtering tests and QML tile selection/manual fallback/overflow/focus checks.
Preserve preview isolation, existing authentication checks, and private headless Cage tests.
Inspect application-only images at normal and minimum sizes.
Run sh tests/check.sh. Parent and independent review follow implementation.
Do not install packages, mutate accounts, change services/PAM, or test real login/power.
