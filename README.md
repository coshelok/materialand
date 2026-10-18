# Materialand

A Material 3 Wayland compositor built with Smithay. It runs as a DRM/KMS session from SDDM or a local TTY, and as a nested Winit window for development.

The compositor core has been reorganized around explicit ownership of windows, outputs, input targets and submitted frames. The Material palettes, typography, widgets and motion primitives live in separate component modules.

## Build

Install Rust and the native development libraries for DRM, EGL, GBM, GLES, libinput, libseat, udev, Wayland and xkbcommon. On Arch the corresponding runtime packages include `libdrm`, `mesa`, `libinput`, `libseat`, `systemd-libs`, `wayland` and `libxkbcommon`.

```sh
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

```sh
./target/release/materialand --nested   # development window
./target/release/materialand --check    # inspect DRM connectors without modesetting
./target/release/materialand            # take the active seat and start a real session
```

Direct mode requires an active local logind/seatd session. Use a display manager or a TTY login; starting it through SSH or a detached user service does not automatically grant seat access. Run as the normal user.

## SDDM

```sh
sudo install -Dm755 target/release/materialand /usr/local/bin/materialand
sudo install -Dm644 packaging/materialand.desktop /usr/share/wayland-sessions/materialand.desktop
```

The session entry runs `materialand`. SDDM reads system Wayland-session directories; installing only into `~/.local/share/wayland-sessions` is insufficient. When developing directly from this checkout, an existing user login-profile override can instead select the checkout's `target/release/materialand`.

## Code organization

| Directory | Responsibility |
| --- | --- |
| `src/app/` | CLI, runtime/socket, client dispatch, clock and animation tick, private D-Bus lifetime |
| `src/backend/nested.rs` | Winit/EGL framebuffer ownership and nested presentation |
| `src/backend/drm/` | Seat/device lifetime, connector selection, scanout, vblank and frame scheduling |
| `src/desktop/` | Window ownership, shared surface coordinates, focus, output layout and resize transactions |
| `src/desktop/protocol/` | Smithay protocol adapters |
| `src/input/` | Target selection, keyboard, pointer, scroll, grabs, actions and seat reset |
| `src/graphics/` | Scene groups, client rendering, uploads, cursor and frame callbacks |
| `src/graphics/canvas/`, `src/graphics/text/` | Raster primitives, font discovery, glyph caching and text composition |
| `src/shell/` | Independent widgets, layout, caches and animation state |
| `src/config/`, `src/theme/`, `src/motion/`, `src/style/` | Desktop entries, keyboard settings, palettes and design tokens |

The rendering and lifecycle contracts are documented in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Controls

- `Super+P`: launcher; type to search, Enter to launch, Escape to close.
- `Super+Enter`: selected installed terminal.
- `Super+D`: light/dark theme.
- `Super+Q`: close the focused window.
- Window decorations: move, resize, minimize, maximize and close.
- Dock: launcher, terminal and running/minimized windows.
- Top bar: launcher, theme, brightness overlay, quick settings and session menu.
- Quick settings: theme, warm night-light tint, reduced motion and nine Material palette variants.

Power-off and reboot actions are restricted to the direct session. In nested mode logout closes the compositor window. Brightness is a compositor dimming overlay, not hardware backlight control.

## Configuration

- `MATERIALAND_NESTED=1`: default to nested mode.
- `MATERIALAND_DRM_DEVICE=/dev/dri/cardN`: try this card first.
- `MATERIALAND_SEED=#RRGGBB`: palette seed; overrides wallpaper extraction.
- `MATERIALAND_WALLPAPER=/path/to/image`: image used to derive the palette seed.
- `MATERIALAND_VARIANT`: `tonalspot`, `vibrant`, `expressive`, `content`, `fidelity`, `rainbow`, `fruitsalad`, `neutral`, `monochrome`.
- `MATERIALAND_FONT_DIR`: first font search directory; repository font assets and system fonts are fallbacks.
- `MATERIALAND_XKB_LAYOUT=us,ru`, `MATERIALAND_XKB_OPTIONS=grp:alt_shift_toggle`: keyboard configuration.

Keyboard settings also fall back to `XKB_DEFAULT_*` and the system keyboard configuration. Multiple layouts get an Alt+Shift group switch unless a `grp:` option is already configured. Super shortcuts use Latin key positions across layouts.

## Validation and current scope

Unit tests cover scene ordering, surface coordinate conversion, output mode changes, frame-state transitions, resize anchoring and limits, input cancellation, component geometry/caches, palettes and raster/text primitives. CI checks formatting, Clippy, tests and a release build.

Automated checks are not a live hardware validation. The rewritten nested/DRM paths still need interactive testing on the target desktop and GPU; no pixel-perfect or daily-driver claim is made. Use the focused checklist in [docs/VALIDATION.md](docs/VALIDATION.md).

The current backend drives one output at scale 1. Multi-monitor layouts, output management, XWayland, locking/suspend, hardware radio/audio/backlight services, touch/tablet/gesture routing and complete drag-and-drop integration are not implemented. Private D-Bus activation is scoped to the compositor; portal compatibility and applications using shared-profile IPC require live testing.
