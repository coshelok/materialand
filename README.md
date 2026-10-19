# Materialand

An Android-inspired Wayland desktop for Linux, built with Rust and Smithay. Material 3 colors, pastel surfaces and spring animations shape a tablet-style home screen, app launcher and dock.

- Add your favorite apps to the home screen and keep separate dock pins.
- Search and scroll installed apps, or use three-finger swipes to open and close the launcher.
- Use the clock/calendar, saved theme and palette choices, Night light, hardware brightness and reduced motion.
- Run as a desktop session from SDDM or a TTY, or inside another desktop with `--nested`.

## Build and run

Install Rust, `pkg-config` and development libraries for DRM, EGL, GBM, GLES, libinput, libseat, udev, Wayland and xkbcommon. The [CI workflow](.github/workflows/ci.yml) lists the packages used on Ubuntu.

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
| Home button or `Super+H` | Show the home screen and minimize open windows |
| `Super+P` | Open the launcher; type to search, Enter to launch, Escape to close |
| Three-finger swipe up / down | Open / close the launcher in a direct session |
| `Super+Enter` | Open a terminal |
| `Super+D` | Switch light/dark theme |
| `Super+Q` | Close the focused window |
| Click the clock | Open date, time and calendar |

Right-click an app to add/remove its home shortcut or dock pin. Click a home shortcut to launch the app or restore its running window. Use the home page arrows to reach additional shortcuts; scroll the launcher with a mouse wheel or touchpad.

## Configuration

For example, use dark mode on first launch and English/Russian keyboard layouts:

```sh
MATERIALAND_THEME=dark MATERIALAND_XKB_LAYOUT=us,ru ./target/release/materialand
```

Shortcuts and dock pins persist in `~/.config/materialand/`, or under `$XDG_CONFIG_HOME/materialand/` when configured. See [Configuration](docs/CONFIGURATION.md) for palette, keyboard and display options.

## Project status

Materialand is experimental. It currently drives one output at scale 1 and supports native Wayland applications. XWayland and multi-monitor layouts are not implemented. Hardware brightness, Night light and touchpad gestures require a direct session and compatible hardware.

[Release notes](docs/RELEASE_NOTES_v0.2.5.md) · [Architecture](docs/ARCHITECTURE.md) · [Build checks and manual validation](docs/VALIDATION.md)
