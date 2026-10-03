# Theming reference

Every visual constant of the sign-in view is a theme token. This is the complete list: the
key path in `theme.json`, the default, the accepted values, and what each token draws.
`Theme.qml` is the source of truth; this document matches it. For the format rules, see
[README → Theming](README.md#theming). A commented example covering every key is in
[`examples/theme.json`](examples/theme.json).

- [Presets](#presets)
- [colors](#colors)
- [background](#background)
- [font](#font)
- [layout](#layout)
- [Validation and limits](#validation-and-limits)

## Presets

Key: `"preset"`. Applied first; explicit keys in the file override it.

| Name | Look |
| --- | --- |
| `dusk` | The stock look: dusk gradient artwork, white text, frosted fills. |
| `midnight` | Near-black blues, dimmer fills. |
| `daylight` | Light frosted gradient, dark text. |

An unknown name keeps `dusk` and logs a warning.

## colors

All values are colors: `#rgb`, `#rgba`, `#rrggbb` or `#aarrggbb` (alpha first in the 8-digit
form, so `#35ffffff` is white at ~20% alpha).

| Key | Default | Draws |
| --- | --- | --- |
| `windowColor` | `#182b56` | Window base color behind the background layer. |
| `overlayTop` | `#18070b20` | Scrim gradient over the background, top stop. |
| `overlayBottom` | `#50070b20` | Scrim gradient over the background, bottom stop. |
| `textBar` | `#dce0ed` | Top-left corner label (`SIGN IN · Wayland` / `PREVIEW …`). |
| `textPrimary` | `#ffffff` | Main text: user names, field text, the `→` buttons, the `…` tile, focused tile border. |
| `textBright` | `#f4f1fa` | Clock time. |
| `textDate` | `#eef0f8` | Clock date line. |
| `textSecondary` | `#e2e4ef` | Status line under the prompt; focused power-button border. |
| `textPlaceholder` | `#d8d9e3` | Placeholder text in the username and password fields. |
| `textCombo` | `#ecedf5` | Session picker text (closed control and its menu items). |
| `accent` | `#d9e2ff` | Focus border of the session picker and both text fields. |
| `error` | `#ed29324c` | Reserved; currently unused. Safe to set, nothing reads it yet. |
| `fillRest` | `#18ffffff` | Session picker fill at rest; user-tile fill on hover; username field fill. |
| `fillHover` | `#35ffffff` | Session picker fill on hover; highlighted menu item; selected user-tile fill; password field fill. |
| `fillSubtle` | `#30ffffff` | `→` buttons at rest; power buttons on hover. |
| `fillAnswer` | `#60ffffff` | `→` buttons on hover. |
| `fillDown` | `#80ffffff` | `→` buttons while pressed. |
| `borderSubtle` | `#40ffffff` | Session picker border at rest. |
| `borderField` | `#60ffffff` | Password field border at rest. |
| `borderButton` | `#70ffffff` | `→` button border at rest. |
| `borderSelected` | `#a0ffffff` | Border of the selected user tile. |
| `borderAvatar` | `#80ffffff` | Border ring of every avatar circle. |
| `avatarFallback` | `#648695` | Avatar circle fill when the user has no picture. |
| `avatarGlyph` | `#e4e9e7` | Placeholder person silhouette inside avatars without a picture. |
| `popupBackground` | `#ed29324c` | Session dropdown menu background. |
| `popupBorder` | `#50ffffff` | Session dropdown menu border. |

## background

| Key | Default | Meaning |
| --- | --- | --- |
| `mode` | `builtin` | `builtin` = embedded SVG artwork; `image` = a picture file; `solid` = one color; `gradient` = `top` → `bottom`. |
| `image` | `""` | Used when `mode` is `image`: a local file path, drawn cropped to fill (`PreserveAspectCrop`). Remote URLs are not supported. |
| `color` | `#182b56` | Used when `mode` is `solid`. |
| `top` | `#182b56` | Used when `mode` is `gradient`: top stop. |
| `bottom` | `#182b56` | Used when `mode` is `gradient`: bottom stop. |

The scrim above the background is themed separately with `colors.overlayTop` /
`colors.overlayBottom`. Modes not in the list fall back to `builtin`.

## font

| Key | Default | Meaning |
| --- | --- | --- |
| `family` | `sans-serif` | Any font family Qt can resolve on the system. |
| `scale` | `1.0` | Multiplies every font size, including the clock cap. Clamped to 0.5–2.0. |

## layout

Positions are fractions of the window size (`clockTopRatio`, `panelTopRatio`,
`panelTopRatioShort`); everything else is pixels. Ratio keys clamp to 0–0.9, pixel keys to
1–2000. There is no free-form x/y placement.

| Key | Default | Draws |
| --- | --- | --- |
| `clockTopRatio` | `0.115` | Clock block top edge, as a fraction of window height. |
| `clockDateSize` | `21` | Clock date font size, px, before `font.scale`. |
| `clockTimeCap` | `100` | Largest clock time size, px; also capped at 13.5% of window height. |
| `panelTopRatio` | `0.37` | Sign-in panel top edge, as a fraction of window height. |
| `panelTopRatioShort` | `0.33` | Same, on short windows. |
| `panelShortHeight` | `650` | Window height below which `panelTopRatioShort` applies. |
| `panelMaxWidth` | `580` | Sign-in panel column width; also capped at window width − 32. |
| `panelBottomMargin` | `100` | Gap between the panel bottom and the window bottom. |
| `tileWidth` | `84` | User tile width. |
| `tileHeight` | `100` | User tile height. |
| `avatarSize` | `56` | Avatar circle diameter. |
| `tileRadius` | `12` | User tile corner radius. |
| `fieldWidth` | `260` | Username field width. |
| `fieldHeight` | `38` | Username field height. |
| `fieldRadius` | `19` | Username field corner radius. |
| `controlRadius` | `20` | Session picker corner radius. |
| `popupRadius` | `14` | Session dropdown menu corner radius. |
| `optionRadius` | `8` | Highlighted menu item corner radius. |
| `powerSize` | `44` | Power button size (square). |
| `powerRadius` | `22` | Power button corner radius. |
| `barMarginLeft` | `24` | Margin of the top-left corner label. |
| `barMarginRight` | `16` | Margin of the session picker from the top-right corner. |
| `bottomBarMargin` | `28` | Margin of the power-button row from the bottom edge. |

The password field width stays fixed at 224 px; the `→` buttons size to `fieldHeight` and
reuse `fieldRadius`, so the answer row stays aligned with the fields.

## Validation and limits

- The first readable file in the search order wins, even if its content is broken; broken
  content means defaults. Startup never fails because of a theme.
- Unknown keys and unknown sections are ignored. A wrong-typed value falls back to that
  key's default. Numbers must be finite.
- Colors accept `#rgb`, `#rgba`, `#rrggbb`, `#aarrggbb`; anything else keeps the default.
- `font.scale` clamps to 0.5–2.0; ratios to 0–0.9; pixels to 1–2000.
- `background.image` must be an absolute local path (`/…`); it is stored as a `file://` URL.
  Remote URLs and relative paths keep the default empty image.
- Theme files over 1 MiB are ignored, as if unreadable.
- Themes load once at startup. There is no hot reload; restart the greeter (or the preview)
  to apply changes.
- Not themed: the embedded background and power-icon SVGs and the bundled demo portraits.
  The white power icons have low contrast on `daylight`; their hover and focus borders stay
  themed.
