# Configuration

Design settings live in `~/.config/materialand/config.toml` (or `$XDG_CONFIG_HOME/materialand/config.toml`). See [Design configuration](DESIGN.md) for layout, typography, colors and animations. Environment variables supply defaults for settings omitted from this file; keyboard and display options remain environment settings.

## Appearance

| Variable | Value |
| --- | --- |
| `MATERIALAND_THEME` | Initial mode: `light` (default) or `dark`, when omitted from `config.toml` |
| `MATERIALAND_SEED` | Palette seed, for example `'#6b85b3'`; takes priority over image extraction |
| `MATERIALAND_WALLPAPER` | Image path used for palette extraction |
| `MATERIALAND_VARIANT` | Initial palette: `tonalspot`, `vibrant`, `expressive`, `content`, `fidelity`, `rainbow`, `fruitsalad`, `neutral` or `monochrome` |
| `MATERIALAND_FONT_DIR` | First font search directory; bundled and system fonts are fallbacks |

For a first launch before a config exists:

```sh
MATERIALAND_THEME=dark MATERIALAND_SEED='#6b85b3' ./target/release/materialand
```

Light/dark mode, palette and reduced motion save immediately to the `appearance` table in `config.toml` and return before the next session's first frame. Other settings and comments are preserved. When this file is missing, the legacy `appearance` file is migrated automatically. New configs contain all settings, with comments explaining each parameter. Partial files remain supported and inherit omitted defaults.

## Touchpad

Set this in the existing `[input.touchpad]` section of `config.toml`, then restart the session:

```toml
[input.touchpad]
invert_scroll = true
```

`tap_to_click = true` and `tap_drag = true` in the same section enable tapping and tap-and-drag in direct sessions. They are enabled by default and apply to newly connected devices as well. Nested sessions inherit the host desktop's tapping settings.

`invert_scroll` reverses horizontal and vertical touchpad scrolling in apps and the launcher. Mouse-wheel direction is independent. Set `false` to restore the original direction.

`drag_lock = true` allows lifting/repositioning a finger during tap-and-drag; tap again to release. Set it to `false` for release on finger lift. `disable_while_typing = true` uses the device's typing suppression when available. One/two/three-finger taps map to left/right/middle buttons; supported pads use two-finger scrolling.

`kinetic_scroll = true` enables shell-list inertia after finger release. `scroll_friction = 9.0` controls its decay (2–30; higher stops sooner). A new touch/click or keyboard action stops coasting; reduced motion disables it. Client applications keep their own scrolling. `swipe_distance = 70.0` sets the gesture threshold (20–300 input units), including horizontal home-page changes. These changes take effect after restarting.

Three fingers up opens the launcher or closes an open shade. Three fingers down opens the shade or closes an open launcher. Four fingers up/down opens/closes recents. Each gesture belongs to its initial panel, so it cannot close one panel and open another in the same movement.


## Keyboard

| Variable | Example |
| --- | --- |
| `MATERIALAND_XKB_LAYOUT` | `us,ru` |
| `MATERIALAND_XKB_OPTIONS` | `grp:alt_shift_toggle` |
| `MATERIALAND_XKB_VARIANT` | XKB variant names corresponding to the chosen layouts |
| `MATERIALAND_XKB_MODEL` | `pc105` |
| `MATERIALAND_XKB_RULES` | `evdev` |

Settings fall back to the corresponding `XKB_DEFAULT_*` variables, then the system keyboard configuration. Multiple layouts use Alt+Shift unless another `grp:` option is set. Super shortcuts use Latin key positions across layouts.

## Session and display

| Variable | Value |
| --- | --- |
| `MATERIALAND_NESTED` | `1` defaults to nested mode; `--nested` is also available |
| `MATERIALAND_DRM_DEVICE` | Preferred DRM device, for example `/dev/dri/card1` |

Use `materialand --check` to inspect connected outputs, modes and reported physical dimensions. Direct mode needs an active local seat through logind or seatd.

Direct sessions select the highest advertised progressive refresh rate at the display's preferred resolution. The initialized output log includes `refresh_hz`. Animation frames follow KMS page flips; there is no fixed 60 Hz animation cap. Nested mode follows the host monitor's reported refresh rate, including when the window moves between displays; 60 Hz is used only when the host reports no valid rate. Actual presentation speed still depends on the renderer and, in nested mode, the host compositor.

Brightness controls the backlight associated with the active connector, with a 2% minimum. It uses direct access when permitted, otherwise `busctl` and logind. Night light adjusts the output's color channels through its gamma ramps. Both controls show **Unavailable** in nested mode or when the required output support is missing. Three-finger launcher/shade gestures and four-finger overview gestures use libinput in direct sessions. All three overlays also have mouse controls and keyboard shortcuts.

## Saved applications

Files live in `$XDG_CONFIG_HOME/materialand/`, or `~/.config/materialand/` when `XDG_CONFIG_HOME` is unset.

| File | Contents |
| --- | --- |
| `workspace.json` | Home shortcut identities, pages and sparse cell positions |
| `home-apps` | Legacy ordered home shortcuts; imported if no workspace exists, then preserved |
| `pinned-apps` | Dock pin identities, in display order |

Manage these through an application's context menu. Drag home shortcuts into free cells or onto another shortcut to swap positions. Hold an icon near the right edge to create another page. Empty cells remain empty. The dock pin action is available in `navigation.mode = "desktop"`. Home shortcuts and dock pins are independent; empty files preserve an intentionally empty selection. Missing application identities remain saved for reinstalls. Unpinning the terminal leaves `Super+Enter` available.

## Navigation and system services

The default `[navigation] mode = "tablet"` uses a home screen, app grid, recent-app overview and system shade, with the top status controls and no bottom navigation buttons. Bare Super returns home on release; Super chords retain their existing actions. Set `mode = "desktop"` to restore the topbar and app dock. See the [controls table](../README.md#controls) for gestures and keyboard shortcuts.

Battery, Wi-Fi and Bluetooth readings come from Linux power-supply attributes, NetworkManager and BlueZ respectively. [System controls](SYSTEM_CONTROLS.md) documents runtime dependencies and connection setup. No radio state is stored in the design configuration.
