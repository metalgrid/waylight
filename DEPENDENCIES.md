# Dependency licensing

Waylight's original code and artwork are licensed **GPL-3.0-only** (`LICENSE`, Cargo
`license = "GPL-3.0-only"`). Copyright 2026 Iskren Hadzhinedev.

**`greetd_ipc` 0.10.3 declares `GPL-3.0-only`, not LGPL.** It is linked into this executable,
which is compatible with the project license. Any distribution of binaries still carries the
customary GPLv3 duties: corresponding source and notice preservation.

Direct dependencies from `cargo metadata --locked`:

| Crate | Locked version | Declared license |
|---|---|---|
| cxx | 1.0.202 | MIT OR Apache-2.0 |
| cxx-qt | 0.10.0 | MIT OR Apache-2.0 |
| cxx-qt-lib | 0.10.0 | MIT OR Apache-2.0 |
| cxx-qt-build (build) | 0.10.0 | MIT OR Apache-2.0 |
| greetd_ipc | 0.10.3 | GPL-3.0-only |
| freedesktop-desktop-entry | 0.8.3 | MPL-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| tokio | 1.53.1 | MIT |
| zbus | 5.19.0 | MIT |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| tempfile (test) | 3.27.0 | MIT OR Apache-2.0 |

Cargo.lock pins the full dependency graph. Before any distribution, collect the actual
license files/notices of **all transitive dependencies** too, using
`cargo metadata --locked --format-version 1` and the resolved crate sources.
The metadata snapshot used during verification is `/tmp/waylight-checks/metadata.json`.
This table is not a complete redistribution notice bundle or legal opinion.

The executable dynamically uses the installed Qt 6 libraries/plugins (Core, Gui, Qml,
Quick, Quick Controls, Wayland and SVG support). Qt offers commercial and open-source
licenses; check the actual modules' LGPL/GPL terms and provide required notices/source
or relinking arrangements. Fontconfig and the platform libraries retain their own terms.
Cage, QtTest, Wayland client and xkbcommon are test/runtime tools, not vendored here.
The test virtual-keyboard helper implements the public protocol documented by
https://github.com/atx/wtype/blob/master/protocol/virtual-keyboard-unstable-v1.xml.
