# Login interaction polish

> Created: 2026-09-30
> Status: LOCKED
> Implements the user's requested one-click login and usable switching/power controls.

## Goal

Activating a pictured user starts authentication immediately with the selected session.
More… permits manual username entry and explicit submission.
User switching and permitted power controls remain available during cancellable authentication.
Cancel authentication safely before executing a requested switch or power action.
Do not change services, packages, PAM, account settings, or deployment.

## Design

Keep one pending intention in the Rust backend: User(username, session), Manual, or Power(action).
Do not add a competing QML queue or change the serialized greetd protocol actor.
An idle request executes immediately. A request during waiting/secret/visible/info/error stores the intention and requests cancellation once.
During cancellation, the latest valid intention replaces the previous one without another Cancel command.
Validate before replacement and recheck before execution. Never store authentication responses in an intention.
Only a controller idle event after confirmed cleanup releases pending work.
Do not expose a temporary interactive idle state before the next action.
A complete greetd 0.10.3 cancellation Error remains an accepted cleanup boundary, as previously reviewed.
Transport uncertainty, worker failure, shutdown, starting, and handoff discard pending work.
In particular, a switch that loses the race to StartSession must never execute after a later start rejection.
Keep StartSession irreversible and never replay it.
Public Cancel/Escape clears any queued intention; internal cancellation preserves it.

Expose closing and an identityChosen(username) signal from the backend.
Update displayed identity only when the backend executes the requested switch.
Selecting a tile calls one backend action; it must not independently call begin from QML.
More… cancels first, then focuses manual input without authentication.
Keep manual begin and identity editing idle-only. Preserve the selected session snapshot.

Enable identity tiles and permitted power controls in idle and cancellable/cancelling states when not closing.
Keep them disabled during loading, power execution, disconnection, shutdown, starting, and handoff.
Display the requested action while cleanup waits. Explain that committed session start prevents further changes.
Preserve literal PAM prompts, visible/secret masking, empty responses, input limits, and isolated preview behavior.

Restart/shutdown confirmation must precede cancellation. Opening or declining a dialog must not alter authentication or erase an unchanged answer.
Accepting requests the backend intention; Sleep requests its intention directly.
Actual logind calls occur only after confirmed cleanup. Preserve capability checks and noninteractive calls.
Revalidate at acceptance. Close obsolete dialogs on committed start, power execution, disconnection, or shutdown.
Do not let prompt updates steal modal focus. Restore appropriate focus when the dialog closes.
Preserve tile focus across discovery in the newly interactive states without stealing input focus.

## Implementation and checks

One worker edits src/backend.rs, Main.qml, focused tests, and README.md.
No new dependencies or protocol redesign.
Test production pending-intention handling and fake socket cancellation ordering, including complete Error and uncertain cleanup.
Capture power dispatch without calling host logind. Test latest-intention replacement, Cancel clearing, shutdown, and start races.
Update QML tests for one-click authentication, manual fallback, switching, confirmation decline/accept, focus, and disabled committed-start states.
Preserve all existing prompt, image, trust, preview isolation, and handoff checks.
Use layout readiness before coordinate input. Run sh tests/check.sh and inspect application-only normal/minimum images.
Parent checks and independent review follow implementation.
Real PAM/VT handoff and host power operations remain outside scope.

## Review

Architect design review approves this approach with the cancellation and focus constraints above.
