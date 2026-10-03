# VM desktop test setup

> Status: LOCKED
> User authorizes installation of this greeter, Noctalia, and Hyprland inside the running test VM.

## Boundary

Only guest system changes are authorized. Do not change host packages, services, PAM, accounts, or login configuration.
Guest SSH: arch@127.0.0.1:2222, pinned host key in vm/known_hosts, initial password arch.
Guest machine ID: 1ffb34f196844df1aec96892ca0893a1. Virtualization: kvm.
QEMU process PID initially 467549; monitor vm/monitor.sock; writable disk vm/greeter-test.qcow2.
Do not modify the base vm/arch-basic.qcow2 or inspect unrelated VMs.
Keep guest SSH working as recovery access. No broad host shares, disk mounts, or exposed network listeners.

## Steps

1. Update guest packages and install Hyprland, Noctalia, greetd, Cage, AccountsService, Qt runtime, graphics/fonts, and essential desktop runtime packages.
2. Build and install our executable in the guest or copy a compatible host build. Prefer a release binary and check Qt compatibility.
3. Configure the guest arch user's Hyprland session to launch Noctalia. Follow the installed Hyprland version's configuration format.
4. Use a dedicated unprivileged greetd account, private session-state directory, and foreground Cage with VT switching.
5. Back up guest config before edits. Start guest greetd. Keep SSH and recovery VT access.
6. Test the actual guest greeter, login with arch's test credentials, and confirm Hyprland plus Noctalia run. Test logout if practical.
7. Record versions, guest paths, logs, status, and useful test commands in vm/README.md.

## Limits

Do not install Noctalia's separate greeter. Test our greeter.
No biometric integration, extra test users, or host authentication changes are needed for this step.
Do not invoke host power controls. VM reboot is permitted if the guest update requires it; retain the same overlay and SSH forwarding.
Do not change greeter application logic without identifying and reporting a concrete runtime blocker.

## References

https://docs.noctalia.dev/noctalia/getting-started/installation/ specifies pacman -S noctalia.
https://docs.noctalia.dev/noctalia/compositor-settings/hyprland/ documents current Lua autostart; use guest-version syntax.
