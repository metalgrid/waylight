# Development Plan: Wayland Greeter Preview

> Created: 2026-09-30
> Status: LOCKED
> Changes require a new plan.

## Goal
Build the approved windowed Qt/QML preview for a macOS-style greetd and Cage greeter.
Keep SDDM and the system login configuration unchanged.

## Steps
1. Create an original background and a responsive login interface.
2. Add keyboard controls and simulated login feedback. Never authenticate or run power commands.
3. Test the controls and preview the interface locally. Document the launch command and limits.

## Dependencies and risks
Qt 6 is installed. Cage is installed.
This preview does not connect to greetd. It must clearly identify simulated controls.
Real authentication and a system switch require later work and approval.
