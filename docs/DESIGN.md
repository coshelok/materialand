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

`input.touchpad.tap_to_click` and `tap_drag` default to `true`. They configure libinput when a tapping-capable device is added or reconnected. A one-finger tap clicks at the pointer; a tap followed by a held second touch supports dragging. `drag_lock` permits repositioning the finger during a drag, and `disable_while_typing` suppresses accidental input. These options apply to direct sessions; nested sessions inherit the host's touchpad behavior.

`input.touchpad.invert_scroll` reverses both horizontal and vertical finger scrolling in applications and the launcher. It does not affect mouse-wheel, wheel-tilt or continuous-device scrolling. Set it to `false` to keep the original direction. Changes apply after restarting the session.

`kinetic_scroll` enables release inertia for shell lists, `scroll_friction` sets its exponential decay, and `swipe_distance` controls the travel needed for navigation gestures. Finger scrolling directly updates the visible offset; its release inertia is integrated with display-paced animation ticks. Wheel/keyboard scrolling keeps its existing springs. New touches stop coasting, while overlay changes consume the remaining old sequence without redirecting it.

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
| `navigation`, `statusbar` | Tablet/desktop layout, launcher origin and the top status strip |
| `shade`, `overview`, `split`, `drag` | System shade, live previews, split divider and shortcut dragging |
| `topbar`, `dock`, `home`, `launcher` | Shell dimensions, spacing, grids and element positions |
| `floating_dock` | Shared pastel capsule tint, opacity, outline and elevation shadow for navigation and the app dock |
| `apps`, `app_menu` | Shared app medallions, captions, tonal families and shortcut menus |
| `window` | Initial size, placement, titlebar and window controls |
| `panels`, `quick_settings`, `brightness`, `power`, `calendar`, `switch` | Panel and control geometry |
| `wallpaper`, `cursor` | Wallpaper forms and tones; fallback cursor size, strokes and colors |

The complete list of keys and values is in [config/default.toml](../config/default.toml), also available through `materialand --print-default-config`. Vector coordinates inside cursor glyphs, protocol rules and numerical integration tolerances remain implementation details.

The status strip and desktop topbar have transparent backgrounds by default; `statusbar.background_alpha` and `topbar.background_alpha` can restore an opaque backing. Their clock, icons and input areas remain available. Both bottom docks use the shared `[floating_dock]` capsule, mixed from Material's fixed pastel roles so dark mode does not turn it black. Exterior shadow padding affects only drawing; the capsule and icon hit areas keep their original screen positions.

The tablet shell takes its visual hierarchy from [AOSP SystemUI](https://android.googlesource.com/platform/frameworks/base/+/android16-qpr2-release/packages/SystemUI/docs/qs-tiles.md), [LineageOS Trebuchet](https://github.com/LineageOS/android_packages_apps_Trebuchet) and [Material 3 Expressive](https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/). Quick controls use leading icons, two text lines and an emphasized active color; the shade places a real backlight slider under its clock/date. `shade.brightness_*`, `shade.tile_*` and `shade.clock_*` centralize their layout. The shared header reserves enough room for the slider even with an older `header_height`. `apps.themed_icons` selects the colors of fallback icons when an original application icon cannot be loaded. Home page dots and workspace geometry are controlled by `[home]`; launcher dimensions and input remain driven by `[launcher]`. Laptop mouse controls and existing gestures are retained.

The tablet shell uses a full-width workspace, a full-width app drawer, transparent three-button navigation and a split shade on wide outputs. Its spatial hierarchy follows [Android's large-screen layout](https://blog.google/products-and-platforms/platforms/android/12l-larger-screens/): quick controls and notifications appear side by side. `launcher.fullscreen` and `shade.fullscreen` can still restore floating panels.

Recent apps use `overview.grid_columns` × `overview.grid_rows` on outputs wider than `overview.grid_threshold` when the rows fit vertically. Compact outputs retain a carousel. Each card owns separate horizontal and vertical reflow springs, so crossing a row boundary after dismissal does not teleport it. Keyboard selection has a visible header highlight; horizontal arrows move between columns and vertical arrows between rows. Clicking a preview restores it; split and close controls remain available. GPU-rounded client previews do not resize applications merely to show them.

## Shell layout

Home cells fill the available width and height, bounded only by screen margins, search and system navigation. `home.cell_width` / `cell_height` set minimum cell sizes; `max_columns` / `max_rows` cap counts. All cells accept shortcuts; the redundant home clock has been removed. The clock in the status strip or shade still opens the calendar.

`workspace.json` stores application identities and normalized cell anchors per page. Empty cells survive removal and restart. Drops onto occupied cells swap two shortcuts; other icons stay put. Display resizing projects saved anchors onto the available grid and resolves collisions without overwriting the saved layout. Missing applications retain their entries. Existing `home-apps` files are imported and left unchanged; the new workspace is written atomically when a shortcut changes. A malformed workspace blocks saves to avoid replacing user data.

Old home keys for the centered column, hero band and clock offsets remain accepted for config compatibility and are marked as legacy in the full default file. The workspace no longer uses them. The drawer distributes columns across its width independently of home shortcut positions.

Fullscreen panels preserve native content proportions. The drawer slides up; the shade reveals downward from a fixed top edge through an expanding rounded GPU contour. Header, brightness, tile rows and notifications use successive opacity phases driven by expansion, so reversing a gesture reverses the reveal without queued delays. Existing cached textures supply the cropped bands; content is not stretched or rerasterized for progress changes. `motion.sheet_stiffness` and `motion.sheet_damping` control release motion (defaults 380 and 0.72), with overshoot softly limited to 16 logical pixels. Owned touchpad swipes set progress directly and settle from distance/recent velocity. Cancellation restores the starting state; reduced motion keeps threshold actions and snaps to the endpoint.

Above `shade.wide_threshold`, the fullscreen shade has a left controls column and a right notifications column. Each scrolls independently beneath its fixed header. Compact outputs use a single column with a separate notification page. `shade.header_button_width` controls clock, power and back targets. Both displayed and pending scroll positions clamp when lists shrink. The wide layout uses the output width; `shade.width` still bounds single-column and floating layouts.

Home and drawer use original `.desktop` icons, resolved through the selected GTK/KDE icon theme, inherited themes, hicolor, and unthemed/pixmap directories. Absolute icon paths and Flatpak export roots are supported. PNG uses tiny-skia, SVG uses librsvg/Cairo, and legacy formats use GdkPixbuf. Decoded images retain their aspect ratio and alpha; failed lookups or decoding retain Material glyphs. Images are loaded once before the first shell frame.

Home and drawer icons are cached separately from their backgrounds. Page changes and scrolling move and crop those textures on the GPU; the drawer scrollbar has its own small cache. The fullscreen shade uses fixed-header and scrolling-body crops of a shared cached texture. Status controls render above overlay scrims. Recent-app headers and chrome participate in the same theme reveal as the shell.

`navigation.mode = "tablet"` is the default: the old topbar and application dock are replaced by a slim top status strip. The bottom Back/Home/Recents controls and their reserved space have been removed; bare Super returns home on release. The legacy navigation dimensions remain accepted for the invisible launcher animation origin. `[statusbar]` controls the reserved height, padding, icons and hit areas. `statusbar.font_size` sets clock and battery text (18 logical pixels by default). Clock text aligns toward its adjacent notification button; battery text aligns to the screen inset. Window placement and home layout respect this inset; fullscreen content uses the full output. `navigation.mode = "desktop"` restores both older components; saved dock pins are retained. Window placement and home layout share the same reserved insets.

The `[shade]` dimensions apply to the tablet system sheet. Existing `[quick_settings]` and typography tokens still style its controls. The body scrolls independently of its fixed header on smaller outputs. `[overview]` controls live recent-window cards: `transition_*` tunes entry/return, `reflow_*` tunes neighboring cards sliding into a gap, and `dismiss_*` tunes upward dismissal and its drag threshold. `dismiss_damping` must be at least 1 to avoid a downward rebound. `close_response_timeout` controls when an app that remains open returns to the row. Geometry retains the entry spring rebound while opacity stays bounded, and window decorations move with client pixels. `[split]` controls the visible divider gap, its wider `divider_hit_width` and preferred minimum pane width, subject to client minimum sizes. `[drag]` controls pointer movement needed to start dragging and the edge dwell used to change home pages.

All new keys are annotated in the complete default file. Existing partial files inherit these defaults; adding them is optional.

## Validation and maintenance

`--check-config` checks syntax, names, types, finite numbers, safe ranges and related dimensions before any renderer or seat is created. An invalid file causes a normal session to log the error and use built-in design values. The file remains intact; UI saves refuse to overwrite invalid input.

To add a design setting, declare its default in `config/default.toml`, add its typed field in `src/config/design/schema.rs`, validate any constraints and read it in the relevant component. Rendering and input should use the same geometry function. Designs are immutable during a session, so cached layers and active springs use consistent values.
