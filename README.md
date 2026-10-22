# Materialand

An Android-inspired Wayland desktop for Linux, built with Rust and Smithay. Android 15-style home and app navigation meet Material 3 Expressive colors, shapes and spring motion: a freely arranged home grid and app search, a full-screen app grid, large rounded recent-app previews and a full system shade.

- Place apps in any free home-screen cell, keep empty spaces, swap occupied cells and create pages by dragging to an edge.
- Open the app grid with three fingers up, recent apps with four fingers up, and the system shade with three fingers down.
- Restore or close recent apps and pair two windows with a spring-animated, draggable split divider.
- Monitor battery charge and control Wi-Fi, Bluetooth, appearance, Night light and hardware brightness from the shade.
- Animate at the active display's refresh rate, with GPU window transforms and circular theme reveals.
- Use a floating pastel dock and transparent Android-style status area in either theme.
- Use Android-style quick-control capsules, a brightness slider in the shade, and original application icons with Material-style fallbacks.
- Run as a desktop session from SDDM or a TTY, or inside another desktop with `--nested`.

## Build and run

Install Rust, `pkg-config` and development libraries for DRM, EGL, GBM, GLES, libinput, libseat, udev, Wayland, xkbcommon, GdkPixbuf, librsvg (2.52 or newer) and Cairo. The [CI workflow](.github/workflows/ci.yml) lists the packages used on Ubuntu.

```sh
cargo build --release --locked
./target/release/materialand --nested  # Run inside your current desktop
```

To start a desktop session, run as your normal user from an active local TTY with logind or seatd:

```sh
./target/release/materialand
```

Use `./target/release/materialand --check` to inspect DRM connectors before launching.

## SDDM installation

Run from the repository root after building:

```sh
sudo install -Dm755 target/release/materialand /usr/local/bin/materialand
sudo install -Dm644 packaging/materialand.desktop /usr/share/wayland-sessions/materialand.desktop
```

Select **Materialand** in SDDM. After rebuilding, reinstall the binary to update the installed session.

## Controls

| Input | Action |
| --- | --- |
| Tap `Super` (Win), or `Super+H` | Show home and minimize open windows |
| Home search or `Super+P` | Open the launcher; type to search, Enter to launch, Escape to close |
| Three-finger swipe up | Open the launcher; close the shade when it is open |
| Three-finger swipe down | Open the shade; close the launcher when it is open |
| Two-finger horizontal scroll on home | Move one home page per gesture |
| Four-finger swipe up | Open recent apps |
| Four-finger swipe down | Close recent apps |
| `Super+Tab` / `Super+S` | Toggle recent apps / system shade |
| `Super+BackSpace` | Leave split screen and restore both windows |
| `Super+Enter` | Open a terminal |
| `Super+D` | Switch light/dark theme |
| `Super+Q` | Close the focused window |
| Clock in the status strip or shade | Open date, time and calendar |
| Bell / Wi-Fi / Bluetooth in the status strip | Open notifications / networks / paired devices |

The default tablet layout uses a compact Android-style status strip at the top (time, notifications, radios and battery), with no bottom navigation buttons or reserved navigation strip. A bare Super press returns home on release; using Super in a shortcut does not trigger Home. App windows reserve the status-strip height; ordinary fullscreen applications hide it. Escape dismisses the current overlay.

Gestures belong to the panel visible when they start. Three fingers down closes an open launcher without opening the shade; three fingers up closes the shade without opening the launcher or recents. Four fingers down closes recents. Other global swipes inside a panel are ignored. A closing animation cannot turn the same action into another overlay. Global swipe directions do not change with scroll inversion.

Touchpad tapping, tap-and-drag and drag lock are enabled by default in direct sessions. One-finger tap clicks; two-finger tap opens a context menu. Tap, then touch again and move to drag icons, windows, recent-app cards, split dividers or sliders without pressing the pad. Drag lock lets you reposition the finger during a drag; tap again to finish. Two-finger scrolling is explicitly selected when supported, and accidental input while typing is suppressed.

Launcher and shade lists follow two-finger motion directly, then coast on release. Touch the pad again or click to stop; reduced motion disables coasting. A scroll remains attached to its original list, including the two shade columns. Home pages turn once per horizontal two-finger gesture. Native applications retain their own scrolling and inertia. These controls act at the pointer; the touchpad is not mapped to absolute screen coordinates. Nested sessions use the host's tapping/device settings and depend on which gesture events it forwards.


On wide, sufficiently tall outputs, recent apps use a two-row tablet grid; compact outputs use a carousel. Scroll or use arrow keys, click a preview to restore it, and use its close button or drag it upward to close it. The card continues upward after release; once it leaves the screen, the app receives a normal close request and neighboring cards slide into the gap. A short drag returns smoothly. Applications may still ask to save work; a card returns if its app ignores or cancels closing. Click the split icon under one preview, then click another preview or its split icon to complete the pair. Click the first icon again to cancel. If the apps need more room, the first selection stays active so you can choose another app. Drag the divider or either adjacent inner window edge to resize both panes; right-click the divider or maximize either app to leave split screen.

Right-click an app to add/remove its home shortcut. Drag an icon to any free cell, or drag it from the launcher onto home. Empty cells stay empty; dropping onto another shortcut swaps the two. Hold an icon near the right edge to create another page. Positions persist in `workspace.json`; existing home shortcuts migrate from `home-apps`, which is kept intact. The home screen has no duplicate clock; all grid cells are available for shortcuts.

Wi-Fi and Bluetooth tiles show actual service state. Click a tile to toggle the radio; its chevron opens available Wi-Fi networks or paired Bluetooth devices. The shade also shows battery percentage and charging state. On wide outputs, quick settings and notifications sit side by side with independent scrolling. On compact outputs, the status-strip bell opens the notification page. Notifications support individual dismissal and Clear all. See [System controls](docs/SYSTEM_CONTROLS.md) for dependencies and connection setup.

App icons come from the `Icon` field of installed `.desktop` entries, including absolute paths and installed icon themes. Home and launcher share these images; missing or unreadable icons retain their Material fallback. PNG and SVG decode directly at startup and are cached, so scrolling and animations do not reload image files. Legacy image formats use GdkPixbuf.

## Configuration

Appearance, layout, typography and animation settings live in `~/.config/materialand/config.toml`, or under `$XDG_CONFIG_HOME/materialand/`. A complete file with comments explaining every parameter is created on first launch; `materialand --init-config` creates it without starting a session. Saved theme and palette choices migrate automatically.

Edit the file and restart the session to apply design changes. Quick settings save theme, palette and reduced motion immediately, preserving comments and other settings. Set `[input.touchpad] invert_scroll = true` to reverse touchpad scrolling. Use `materialand --check-config` to validate edits. See [Design configuration](docs/DESIGN.md) and the commented [default configuration](config/default.toml).

Set `[navigation] mode = "desktop"` to restore the topbar and application dock, including its separate pin menu. Home shortcuts and dock pins persist separately in the same directory. [Configuration](docs/CONFIGURATION.md) covers keyboard and display options.

## Project status

Materialand is experimental. It currently drives one output at scale 1 and supports native Wayland applications. XWayland and multi-monitor layouts are not implemented. Hardware brightness, Night light and touchpad gestures require a direct session and compatible hardware.

[Architecture](docs/ARCHITECTURE.md) · [Build checks and manual validation](docs/VALIDATION.md)
