# THEME-PLAN: Theme support for Waylight (locked)

> Approved by user 2026-10-03. Customizes: colors + background + font + element positioning.
> Provision: built-in presets + custom JSON file. Selection: XDG auto-load, no flag.

## Goals

1. Extract every visual constant in `Main.qml` into one theme object with current values as defaults.
2. Load optional JSON theme files from fixed XDG locations. A missing or broken theme must never prevent startup.
3. Ship three built-in presets: `dusk` (exact current look), `midnight`, `daylight`.
4. Keep behavior, focus order, accessibility, tests, and isolation unchanged. Visual defaults stay pixel-identical.

## Non-goals

- No free-form x/y element placement; positioning is a bounded token set.
- No hot reload, no theme picker UI, no network access, no new dependencies.
- No changes to greetd flow, PAM, SDDM, `vm/`, or the auth/backend logic.

## Theme file format (JSON)

```json
{
  "preset": "dusk",
  "colors":   { "accent": "#d9e2ff", "...": "..." },
  "background": { "mode": "builtin", "image": "", "color": "", "top": "", "bottom": "" },
  "font":     { "family": "sans-serif", "scale": 1.0 },
  "layout":   { "panelMaxWidth": 580, "...": "..." }
}
```

- `mode`: `builtin` (embedded background.svg) | `image` (local file, PreserveAspectCrop) | `solid` | `gradient` (top→bottom `colors.windowColor`-independent).
- Preset applies first; explicit keys override afterwards. Unknown preset name → keep defaults.

## Search order (first existing file wins)

1. `${XDG_CONFIG_HOME:-$HOME/.config}/waylight/theme.json`
2. `/etc/waylight/theme.json`

Rust computes the ordered candidate list once at Backend construction and exposes it to QML
as a JSON string property `themePaths` (pure helper + unit test; do not mutate env in tests).

## Validation (fail-open, always)

- Missing file → defaults. Parse error → defaults + `console.warn`.
- Unknown keys ignored. Wrong type → that key's default.
- Colors must match `#rgb|#rgba|#rrggbb|#aarrggbb`, else default.
- Clamp: ratios 0–0.9, sizes 1–2000 px, `font.scale` 0.5–2.0.

## Token table (defaults = current literals)

colors: windowColor `#182b56`, overlayTop `#18070b20`, overlayBottom `#50070b20`,
textBar `#dce0ed`, textPrimary `#ffffff`, textBright `#f4f1fa`, textDate `#eef0f8`,
textSecondary `#e2e4ef`, textPlaceholder `#d8d9e3`, accent `#d9e2ff`, error `#ed29324c`,
fillHover `#35ffffff`, fillRest `#18ffffff`, fillDown `#80ffffff`, fillAnswer `#60ffffff`,
fillSubtle `#30ffffff`, borderSelected `#a0ffffff`, borderField `#60ffffff`,
borderSubtle `#40ffffff`, borderButton `#70ffffff`, borderAvatar `#80ffffff`,
avatarFallback `#648695`, avatarGlyph `#e4e9e7`, popupBackground `#ed29324c`,
popupBorder `#50ffffff`.

background: as above; default `builtin`.

font: family `sans-serif`, scale `1.0` (applied to every `font.pixelSize` and the clock cap).

layout: clockTopRatio `0.115`, clockDateSize `21`, clockTimeCap `100`,
panelTopRatio `0.37`, panelTopRatioShort `0.33`, panelShortHeight `650`,
panelMaxWidth `580`, panelBottomMargin `100`, tileWidth `84`, tileHeight `100`,
avatarSize `56`, tileRadius `12`, fieldWidth `260`, fieldHeight `38`, fieldRadius `19`,
controlRadius `20`, popupRadius `14`, optionRadius `8`, powerSize `44`, powerRadius `22`,
barMarginLeft `24`, barMarginRight `16`, bottomBarMargin `28`.

## Presets

- `dusk`: token-for-token identical to today's look.
- `midnight`: near-black blues, dimmer fills, same structure.
- `daylight`: light frosted palette, dark text; must keep ≥4.5:1 contrast for body text.

## Implementation shape

- `Theme.qml` (QtObject, not a singleton; instantiated inside `Main.qml` as `id: theme`)
  with all tokens as properties, `applyJson(text)`, `loadFile(path)`, `resetTo(presetName)`.
- `Main.qml` references only theme tokens; no literal colors/sizes remain except white fallbacks in glyphs where required for contrast tests.
- `src/backend.rs`: `themePaths` property (JSON array string).
- Register both new files in `build.rs`/`resources.qrc` exactly like existing QML.

## Verification gate (parent)

`cargo fmt --check`, `cargo build --locked`, `cargo test --locked`,
`cargo clippy --locked --all-targets -- -D warnings`, qmllint clean, qmltestrunner all pass,
Cage harness cases pass, preview screenshots for all three presets reviewed.
