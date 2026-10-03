# TODO: Usable greetd Client

> Derived from: .pi/GREETD-PLAN.md
> Last updated: 2026-09-30

## Progress: 7/7 completed

- [x] Build the Rust executable and embed resources.
- [x] Implement the serialized authentication controller and bounded transport.
- [x] Discover trusted sessions and persist the selected session ID.
- [x] Connect QML controls and confirmed logind actions.
- [x] Run Rust, QML, executable, and nested Cage checks.
- [x] Document build, installation proposals, and recovery.
- [x] Complete parent checks and independent review.

Implementation evidence and review map: `.pi/GREETD-VERIFICATION.md`.
The parent reran all checks successfully. Independent review accepts the corrected build.
Final parent evidence: `/tmp/waylight-parent-final/all-checks.txt`.

## Future system test

Real PAM authentication and VT handoff remain outside this approval.
Keep SDDM active and greetd stopped.
