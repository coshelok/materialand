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

`input.touchpad.tap_to_click` and `tap_drag` default to `true`. They configure libinput when a tapping-capable device is added or reconnected. A one-finger tap clicks at the pointer; a tap followed by a held second touch supports dragging. These options apply to direct sessions; nested sessions inherit the host's touchpad behavior.

`input.touchpad.invert_scroll` reverses both horizontal and vertical finger scrolling in applications and the launcher. It does not affect mouse-wheel, wheel-tilt or continuous-device scrolling. Set it to `false` to keep the original direction. Changes apply after restarting the session.

The generated file already includes advanced sections such as `[dock]` and `[colors.dark]`; edit existing keys instead of adding duplicate tables. Partial files such as the example above are also supported. All dimensions are logical pixels at the compositor's current scale 1. Durations are seconds, spring damping is a ratio, and opacities are fractions from 0 to 1. Typography accepts `regular`, `medium`, `bold` and `icon` weights. Material role overrides use opaque `#RRGGBB` colors; unspecified roles come from the generated Material palette.

`appearance.seed = ""` enables extraction from `appearance.wallpaper`. This selects palette colors; it does not display the image as wallpaper. The tonal wallpaper's vector control points, mixes and glows have their own section.

## Sections

| Section | Controls |
| --- | --- |
| `input.touchpad` | Touchpad scroll inversion, tap-to-click and tap dragging |
| `appearance`, `colors.light`, `colors.dark` | Theme, Material variant, palette source, reduced motion and individual color roles |
| `shape`, `state`, `shadow` | Corner radii, hover/press/ripple opacity and elevation shadows |
| `typography`, `fonts` | Text roles, sizes, line heights, tracking, weights and optional font directory |
| `motion`, `easing` | Spring stiffness/damping, transition durations, entrance scales and easing curves |
| `navigation`, `statusbar` | Tablet/desktop layout, navigation controls and the top status strip |
| `shade`, `overview`, `split`, `drag` | System shade, live previews, split divider and shortcut dragging |
| `topbar`, `dock`, `home`, `launcher` | Shell dimensions, spacing, grids and element positions |
| `floating_dock` | Shared pastel capsule tint, opacity, outline and elevation shadow for navigation and the app dock |
| `apps`, `app_menu` | Shared app medallions, captions, tonal families and shortcut menus |
| `window` | Initial size, placement, titlebar and window controls |
| `panels`, `quick_settings`, `brightness`, `power`, `calendar`, `switch` | Panel and control geometry |
| `wallpaper`, `cursor` | Wallpaper forms and tones; fallback cursor size, strokes and colors |

The complete list of keys and values is in [config/default.toml](../config/default.toml), also available through `materialand --print-default-config`. Vector coordinates inside cursor glyphs, protocol rules and numerical integration tolerances remain implementation details.

The status strip and desktop topbar have transparent backgrounds by default; `statusbar.background_alpha` and `topbar.background_alpha` can restore an opaque backing. Their clock, icons and input areas remain available. Both bottom docks use the shared `[floating_dock]` capsule, mixed from Material's fixed pastel roles so dark mode does not turn it black. Exterior shadow padding affects only drawing; the capsule and icon hit areas keep their original screen positions.

The tablet shell takes its visual hierarchy from [AOSP SystemUI](https://android.googlesource.com/platform/frameworks/base/+/android16-qpr2-release/packages/SystemUI/docs/qs-tiles.md), [LineageOS Trebuchet](https://github.com/LineageOS/android_packages_apps_Trebuchet) and [Material 3 Expressive](https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/). Quick controls use leading icons, two text lines and an emphasized active color; the shade places a real backlight slider under its clock/date. `shade.brightness_*`, `shade.tile_*` and `shade.clock_*` centralize their layout. The shared header reserves enough room for the slider even with an older `header_height`. `apps.themed_icons` selects a single Material icon family or the previous category colors. Home page dots and the large clock are controlled by `[home]`; launcher dimensions and input remain driven by `[launcher]`. Laptop mouse controls and existing gestures are retained.

The full tablet layout combines Android 15's home/app-drawer/recents structure with Expressive shapes. The home clock uses separate hour/minute shapes, a bounded column count and a functional local app-search capsule. `home.widget_*`, `home.search_*` and `home.max_columns` control these elements. `launcher.fullscreen` and `shade.fullscreen` fill the work area; setting either to `false` restores its floating panel. The shade chooses two columns on compact screens and `shade.wide_columns` above `shade.wide_threshold`.

Recent apps use one large center preview with smaller, lower neighbors. App identity sits above the snapshot; Open, split and close actions sit below it. `overview.card_radius`, `app_header_height`, `side_scale`, `side_offset` and `scrim_alpha` style that layout. The GPU rounds actual client pixels in framebuffer coordinates, including transformed buffers; chrome textures retain their base size throughout the carousel animation. Home icons fade out while entering recents, leaving the wallpaper behind the cards. These are compositor visuals and do not resize the client merely to show its preview.

## Shell layout

`navigation.mode = "tablet"` is the default: the old topbar and application dock are replaced by a slim top status strip and compact bottom navigation. `[statusbar]` controls the reserved height, padding, icons and hit areas. Window placement and home layout respect this inset; fullscreen content uses the full output. `navigation.mode = "desktop"` restores both older components; saved dock pins are retained. Window placement and home layout share the same reserved insets.

The `[shade]` dimensions apply to the tablet system sheet. Existing `[quick_settings]` and typography tokens still style its controls. The body scrolls independently of its fixed header on smaller outputs. `[overview]` controls live recent-window cards: `transition_*` tunes entry/return, `reflow_*` tunes neighboring cards sliding into a gap, and `dismiss_*` tunes upward dismissal and its drag threshold. `dismiss_damping` must be at least 1 to avoid a downward rebound. `close_response_timeout` controls when an app that remains open returns to the row. Geometry retains the entry spring rebound while opacity stays bounded, and window decorations move with client pixels. `[split]` controls the visible divider gap, its wider `divider_hit_width` and preferred minimum pane width, subject to client minimum sizes. `[drag]` controls pointer movement needed to start dragging and the edge dwell used to change home pages.

All new keys are annotated in the complete default file. Existing partial files inherit these defaults; adding them is optional.

## Validation and maintenance

`--check-config` checks syntax, names, types, finite numbers, safe ranges and related dimensions before any renderer or seat is created. An invalid file causes a normal session to log the error and use built-in design values. The file remains intact; UI saves refuse to overwrite invalid input.

To add a design setting, declare its default in `config/default.toml`, add its typed field in `src/config/design/schema.rs`, validate any constraints and read it in the relevant component. Rendering and input should use the same geometry function. Designs are immutable during a session, so cached layers and active springs use consistent values.
