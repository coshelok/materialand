# Design configuration

Materialand reads `$XDG_CONFIG_HOME/materialand/config.toml`, or `~/.config/materialand/config.toml`. The first session creates a complete file with comments explaining every parameter, its units and relevant constraints. To create or validate it without starting a compositor:

```sh
materialand --init-config
materialand --check-config
```

Existing files are preserved by `--init-config`. New files contain every built-in setting; automatic initialization and UI saves never shorten them. The optional `materialand --compact-config` command explicitly removes redundant defaults, keeping an exact backup and retaining custom values. The legacy `appearance` file supplies the saved theme and palette when the new file does not exist; it is retained after migration. Home shortcuts and dock pins remain independent application lists.

## One source of defaults

[config/default.toml](../config/default.toml) declares every built-in design value. Rust includes that document in the binary and exposes a typed, immutable view to layout, rendering, input and window placement. Components do not keep competing copies of these values.

User files can override only the settings they need. Omitted values inherit startup environment defaults, then the built-in document. Only settings explicitly present in the file override startup environment defaults.

Changes to the file apply when the session restarts; rebuilding is unnecessary. Light/dark mode, palette and reduced motion change immediately through quick settings and save back to this same file. Saving these preferences preserves other settings and comments, including edits made since startup. Writes use atomic replacement.

## Partial-file example

```toml
schema_version = 1

[appearance]
theme = "dark"
variant = "Vibrant"
seed = "#6b85b3"
reduce_motion = false

[input.touchpad]
invert_scroll = true
```

`input.touchpad.invert_scroll` reverses both horizontal and vertical finger scrolling in applications and the launcher. It does not affect mouse-wheel, wheel-tilt or continuous-device scrolling. Set it to `false` to keep the original direction. Changes apply after restarting the session.

The generated file already includes advanced sections such as `[dock]` and `[colors.dark]`; edit existing keys instead of adding duplicate tables. Partial files such as the example above are also supported. All dimensions are logical pixels at the compositor's current scale 1. Durations are seconds, spring damping is a ratio, and opacities are fractions from 0 to 1. Typography accepts `regular`, `medium`, `bold` and `icon` weights. Material role overrides use opaque `#RRGGBB` colors; unspecified roles come from the generated Material palette.

`appearance.seed = ""` enables extraction from `appearance.wallpaper`. This selects palette colors; it does not display the image as wallpaper. The tonal wallpaper's vector control points, mixes and glows have their own section.

## Sections

| Section | Controls |
| --- | --- |
| `input.touchpad` | Touchpad scroll inversion |
| `appearance`, `colors.light`, `colors.dark` | Theme, Material variant, palette source, reduced motion and individual color roles |
| `shape`, `state`, `shadow` | Corner radii, hover/press/ripple opacity and elevation shadows |
| `typography`, `fonts` | Text roles, sizes, line heights, tracking, weights and optional font directory |
| `motion`, `easing` | Spring stiffness/damping, transition durations, entrance scales and easing curves |
| `topbar`, `dock`, `home`, `launcher` | Shell dimensions, spacing, grids and element positions |
| `apps`, `app_menu` | Shared app medallions, captions, tonal families and shortcut menus |
| `window` | Initial size, placement, titlebar and window controls |
| `panels`, `quick_settings`, `brightness`, `power`, `calendar`, `switch` | Panel and control geometry |
| `wallpaper`, `cursor` | Wallpaper forms and tones; fallback cursor size, strokes and colors |

The complete list of keys and values is in [config/default.toml](../config/default.toml), also available through `materialand --print-default-config`. Vector coordinates inside cursor glyphs, protocol rules and numerical integration tolerances remain implementation details.

## Validation and maintenance

`--check-config` checks syntax, names, types, finite numbers, safe ranges and related dimensions before any renderer or seat is created. An invalid file causes a normal session to log the error and use built-in design values. The file remains intact; UI saves refuse to overwrite invalid input.

To add a design setting, declare its default in `config/default.toml`, add its typed field in `src/config/design/schema.rs`, validate any constraints and read it in the relevant component. Rendering and input should use the same geometry function. Designs are immutable during a session, so cached layers and active springs use consistent values.
