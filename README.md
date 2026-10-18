# Materialand — Material 3 Wayland compositor

Materialand is a small Wayland compositor built with Smithay. It runs either as a **real session on the hardware** (DRM/KMS scanout, libinput seats, libseat) or **nested inside another compositor** for development. This rewrite focuses on consistent Material 3 colour, shape, type and motion tokens; Material You dynamic colour with switchable palettes and wallpaper seeds; a compact quick-settings sheet with a palette picker; launcher, window controls, brightness dimming and an animated night-light overlay; and safer handling of window state, input and desktop entries.

> **Scope:** this is a compositor prototype, not a complete Linux distribution or a full desktop environment. It cannot control host Wi-Fi, Bluetooth, battery profiles, audio devices or hardware brightness without platform services and protocol integration. The quick-settings tiles only expose features actually implemented by this compositor; they intentionally do not fake unsupported device switches.

## Build and run

A working Rust toolchain and the native dependencies required by Smithay's DRM/KMS, EGL, GBM, libinput, udev and libseat backends must be installed on the host (on Arch: `libdrm`, `mesa`, `libinput`, `systemd-libs`, `libseat`; the `backend_session_libseat` feature links libseat, which finds logind or seatd at runtime).

```sh
cargo check
cargo test
cargo build --release
```

Without arguments the binary takes over the machine as a real session (see below). For development inside a running desktop use the nested mode:

```sh
cargo run --release -- --nested        # a window in the current compositor
cargo run --release -- --check         # what the KMS backend would drive, no modeset
```

In a real session the compositor starts a terminal if one is discovered in the application list, otherwise it tries `foot`; the nested mode behaves the same way.

## Running as a real session

Requirements:

- A seat managed by **logind** (or **seatd**) and a user who may take it: logind grants the active local session access to `/dev/dri/card*` and input devices, so no root is needed. Sitting in the `video`/`input` groups is not required but also not harmful.
- A GPU with a **connected connector**. The compositor tries the firmware-primary GPU first (`boot_vga`), then every other DRM card, and drives the first one whose connector reports a connected display. The mode is the connector's preferred mode.

Launch from the display manager by installing the binary and the session file from `packaging/materialand.desktop`:

```sh
cargo build --release
# system-wide, with the binary on the DM's PATH (`Exec=materialand` in the session file)
sudo install -Dm755 target/release/materialand /usr/local/bin/materialand
sudo install -Dm644 packaging/materialand.desktop /usr/share/wayland-sessions/materialand.desktop
```

Without the session file the compositor does not appear in a display manager's session list at all. **SDDM** only reads the system directories (`/usr/share/wayland-sessions` and `/usr/local/share/wayland-sessions`) — a copy in `~/.local/share/wayland-sessions` is ignored, so install it system-wide there. Managers that follow the XDG data directories (GDM) also read the per-user path, with `Exec` edited to the absolute path of the binary.

From a TTY, log in on the VT you want the session on (logind must see it as the active session; a session started over SSH or on a background VT will be denied the devices) and run the binary directly:

```sh
cargo run --release            # or the installed binary
```

`materialand --check` lists the DRM cards, connectors and modes that would be used. It opens the devices through libseat read-only and never programs a CRTC, so it is safe to run inside a running graphical session. `MATERIALAND_DRM_DEVICE=/dev/dri/cardN` forces a specific card to the front of the candidate list (useful on hybrid-GPU laptops).

Details of the KMS backend:

- **Single output:** one connector is driven at a time. Plugging the monitor that is currently in use unplugs the other, hotplug events re-run the device scan and switch the active connector; multiple outputs at once are future work.
- **VT switches** (and screen locking, which pauses the session) drop every device and release DRM master; switching back re-opens the devices and re-initialises the output.
- **Frame pacing:** frames are queued one at a time and the next render happens on vblank; client frame callbacks fire when the frame is actually on screen.
- **Clients** are launched with `WAYLAND_DISPLAY`, `XDG_SESSION_TYPE=wayland` and `XDG_CURRENT_DESKTOP=Materialand`; `DISPLAY` is removed because there is no XWayland yet. The session D-Bus bus is kept so Portals and D-Bus activation work.

### If start-up fails with `FailedToOpenSession`

`Error: FailedToOpenSession(Os { code: 38, kind: Unsupported, message: "Function not implemented" })` means libseat found neither a running **seatd** nor a **logind session on a seat** for this process. Typical causes and fixes:

- The compositor was started from a context logind does not associate with a session — a systemd user service (for example UWSM-managed sessions, where applications run under `user@.service` instead of the session scope), an SSH connection, or a container. Start it from a display-manager session or a TTY login instead.
- `seatd` is not running and logind is not usable: install/enable seatd (`sudo systemctl enable --now seatd`), add your user to the `seat` group and log back in.
- `sudo` is not needed and should be avoided: libseat opens the devices through logind for the logged-in user, while a root launch strips the session bus and portals from every client and makes applications run as root, which is a common source of strange client behaviour. Run it as your own user on the active VT.
- Even with seat access the DRM device must be free: a running desktop on the same GPU keeps DRM master, so stop it (or use a GPU that no compositor drives) before starting a real session. `materialand --check` works under either backend and reports what would be used.


## Colour and palettes

The whole scheme is computed with the Material You (HCT) algorithm from the `material-colors` crate, so every role follows the M3 tonal palette system. Three environment variables configure the seed at start-up:

- `MATERIALAND_SEED` — a hex seed colour (`#RRGGBB`, `RRGGBB` or `0xAARRGGBB`). Takes precedence over the wallpaper.
- `MATERIALAND_WALLPAPER` — path to an image; the dominant colour is extracted (Celebi quantiser over a downscaled copy) and used as the seed when `MATERIALAND_SEED` is not set. Corrupt or unreadable images are reported and the default seed is used.
- `MATERIALAND_VARIANT` — one of the nine Material You variants: `tonalspot` (default pastel), `vibrant`, `expressive`, `content`, `fidelity`, `rainbow`, `fruitsalad`, `neutral`, `monochrome`.

All nine variants are also available at runtime in the quick-settings palette grid (three dots per chip show its primary, secondary and tertiary tones). Picking a palette crossfades the scheme; the light/dark toggle uses a radial reveal that grows out of the control you pressed, composited as a masked second pass of the entire UI.

## Fonts

Font files are optional: the compositor runs with the fonts installed on the host. The search order is

1. `MATERIALAND_FONT_DIR`, if set;
2. `assets/fonts` (this repository ships optional Roboto and Material Icons copies there);
3. `~/.local/share/fonts` and `~/.fonts`;
4. the common system directories (`/usr/share/fonts`, `/usr/local/share/fonts`, …).

The UI font is the first found of Roboto, Noto Sans, DejaVu Sans or Liberation Sans; the medium and bold weights are looked up separately and fall back to the regular face. Material Icons are used only when `MaterialIcons-Regular.ttf` and its `codepoints` map are found in the same directory; otherwise the built-in Unicode icon fallback is used. As a result, exact glyph shapes depend on the fonts installed on the host.

## Motion

All animated transitions use the official M3 Expressive motion-physics tokens (`src/anim.rs`): spatial springs at stiffness 1400/700/300 with damping ratio 0.9 for position, size and scale, and critically damped effects springs at 3800/1600/800 for colour and opacity. Highlights:

- Chrome controls (top bar, dock, quick-settings tiles, palette chips, launcher tiles) activate on pointer *release* and play a press scale plus a bounded ripple clipped to the control.
- The launcher opens with a container transform: a linear mapping of the final layout onto a rectangle that morphs out of the dock/top-bar button, with an animated corner radius and a translucent scrim.
- The quick-settings sheet slides out of its top-bar button with a scale and radius morph; switches bounce slightly when toggled.
- The brightness slider handle grows while grabbed.
- Windows play a scale-and-fade entrance on map and an exit animation before `send_close` is delivered; while closing they no longer accept input.
- "Reduce motion" snaps every spring to its end pose.

## Keyboard

The seat starts with the `us` layout. Layouts, variants and options are
xkeyboard-config names, resolved at start-up from the first source that sets
them:

1. `MATERIALAND_XKB_LAYOUT`, `MATERIALAND_XKB_VARIANT`,
   `MATERIALAND_XKB_MODEL`, `MATERIALAND_XKB_RULES` and
   `MATERIALAND_XKB_OPTIONS`;
2. the standard xkbcommon variables `XKB_DEFAULT_LAYOUT`,
   `XKB_DEFAULT_VARIANT`, `XKB_DEFAULT_MODEL`, `XKB_DEFAULT_RULES` and
   `XKB_DEFAULT_OPTIONS`;
3. `/etc/X11/xorg.conf.d/00-keyboard.conf`, the fragment
   `localectl set-x11-keymap` writes and Xorg reads;
4. the evdev/pc105/us defaults.

Several layouts are a comma-separated list (`us,ru`) and are only usable with
a switching chord: when more than one layout is configured and no `grp:`
option is given, `grp:alt_shift_toggle` is appended automatically. For a
daily-driver `us,ru` laptop, either export the two variables in
`/etc/environment` (SDDM sessions and TTY logins both read it):

```sh
MATERIALAND_XKB_LAYOUT=us,ru
MATERIALAND_XKB_OPTIONS=grp:alt_shift_toggle
```

or configure the system-wide file, which Xorg and materialand share:

```sh
localectl set-x11-keymap us,ru pc105 "" grp:alt_shift_toggle
```

A layout name xkbcommon cannot compile is not fatal: the compositor logs a
warning and keeps the plain US keymap.

The Super chords are matched on the raw Latin keysym, so `Super+P`, `Super+D`
and the others keep working while the Russian layout is selected.

Key repeat (how a held key repeats) is a client-side affair on Wayland; the
seat only advertises `config::KEY_REPEAT_DELAY_MS` (500 ms) and
`config::KEY_REPEAT_RATE_HZ` (25/s) through `wl_keyboard.repeat_info`.

## Current controls

- `Super + P`: app launcher
- `Super + Enter`: terminal
- `Super + D`: light/dark theme (radial reveal from the top-bar theme button)
- `Super + Q`: request close for the focused window
- `Alt + Shift`: switch the keyboard layout (when several layouts are configured; see [Keyboard](#keyboard))
- Top bar: launcher, power/close, theme, brightness dimmer and quick settings
- Dock: leading launcher button, then pinned/running applications
- Quick settings: theme, night-light tint, reduced motion, brightness popup and the nine-variant palette grid

The "brightness" control is a compositor overlay; it does not change the laptop panel's hardware backlight. The "night light" control adds a warm tint to the rendered output; it does not change the display's physical colour-temperature setting.

## Validation status

Verified on this tree (Rust 1.96, Linux host):

- [x] `cargo check --all-targets` and `cargo clippy --all-targets` — clean;
- [x] `cargo test` — 83 unit tests pass;
- [x] `cargo build --release` — produces a working binary (`--help` runs);
- [x] interactive Wayland client smoke test — nested (GNOME Disks, VS Code) and a real session from a TTY: windows, pointer and keyboard input, focus and theme switching work;
- [ ] visual/pixel comparison against the Material 3 reference;
- [ ] DRM/KMS run on real hardware — two field tests so far: the seat, scanout, input and client rendering all work. The first run produced the frame-pacing, undrawn-toplevel and session-installation fixes, the second the key-repeat rate; all of them plus the new keyboard-layout support still want their re-run on the machine.

The DRM/KMS path is compile-checked and has run on real hardware; the issues the field tests found are fixed in this tree but not yet re-tested on the machine. Do not treat this archive as a guarantee of zero defects or a pixel-perfect reproduction of Google's proprietary/system-specific implementation. Run `materialand --check` on the target machine, then start a real session from a TTY, and report any output before using it as a daily-driver compositor.

## Known limitations before daily use

- Only one output is driven at a time (the first connector that reports a connected display, on the firmware-primary GPU); there is no output-management protocol and no multi-monitor layout yet.
- The output mode is the connector's preferred mode. Scale, transform, VRR and colour management are not exposed.
- Wayland clients only: there is no XWayland, so X11 applications cannot run.
- Relative pointer motion uses libinput's default acceleration profile; there is no pointer/touchpad settings UI. Touch, tablet and gesture events are not wired into the shell.
- The XDG popup manager is wired into tracking, rendering and input: `xdg_popup.grab` moves keyboard and pointer grabs onto the popup, and reposition requests are answered with geometry clamped to the output. Unconstraining menus across multiple outputs is future work along with the rest of multi-monitor support.
- Full drag-and-drop/data-device behaviour and several desktop protocols are not fully implemented.
- This is a compositor with custom-drawn chrome, not a bootable LineageOS build or a complete laptop shell. It currently does not manage hardware keys, lock screen, suspend, power profiles or hardware radio devices.
- Motion is approximated: tiny-skia has no GPU blur, so elevation shadows are layered translucent rounded rectangles. Overlay container transforms (launcher, sheets, menus) are applied by the GPU to cached layers; only the radial theme reveal masks on the CPU.
