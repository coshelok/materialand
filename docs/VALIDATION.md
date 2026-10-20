# Validation

Run from the repository root:

```sh
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
./target/release/materialand --help
```

The execution environment has no usable seat/KMS session. Unix socket pairs can be created, but Wayland server socket writes fail with `Operation not permitted`; system D-Bus is also restricted. CPU tests and explicit offscreen EGL checks can run here. Real client interaction, radio operations and physical presentation need the checks below outside that environment.

The reveal shader also has an explicit offscreen EGL pixel test. It can run on an EGL device or Mesa's software renderer without taking a seat:

```sh
cargo test --locked egl_reveal_preserves_pixels_clipping_scaling_and_shader_isolation -- --ignored
```

This checks shader compilation, radial clipping, alpha blending, scaled/cropped sources, framebuffer orientation and shader state isolation. It does not measure physical display FPS. CPU regressions cover native-resolution refresh selection, 30–360 Hz/fractional animation intervals and retention of both cached theme endpoints.

Live recent-app corners have a separate offscreen GPU regression:

```sh
cargo test --locked egl_rounded_previews_preserve_clipping_transforms_and_shader_isolation -- --ignored
```

It checks actual rounded clipping and alpha with scaled/cropped sources, both direct/nested framebuffer orientations, and shader isolation. Check real client snapshots, app identity/actions, neighboring card depth and upward dismissal in a normal session. Check the home search capsule, five-column compact and bounded tablet app grids, and full-height drawer/shade in both themes. Existing gesture ownership and split behavior must remain intact.

## Nested session

1. Start `./target/release/materialand --nested` from a working Wayland desktop.
2. Open two overlapping clients, including VS Code/Cursor with client-side decorations and a terminal requesting server-side decorations. Check that each has one titlebar, a translucent terminal shows the scene beneath it, and no light body outline or opaque system backdrop is visible. Check focus and stacking.
3. Open menus/submenus that extend outside the parent window. Move and click inside the protruding part; scroll and dismiss the menu.
4. Check arrow/text/resize cursors, client cursor surfaces and hotspot alignment.
5. Move/resize each edge and corner, including fast resize then immediate release. Check client minimum/maximum sizes and opposite-edge anchoring.
6. Maximize, fullscreen, minimize, restore, close and remap clients. Resize the host window while clients are maximized/fullscreen.
7. Exercise launcher search, dock, quick settings, theme/palette transitions, brightness and reduced motion. Shell overlays must consume clicks/scroll over their bounds.
8. Open/close applications rapidly, including changing focus while the dock is resizing. Check spring motion, stable icon order and click targets throughout the animation. Repeat with reduced motion enabled.
9. Pin and unpin an app from both the launcher and dock context menus. Restart the compositor and verify the selection, including a dock with no pins. Open/close a pinned app and check it uses its existing slot. Dismiss the context menu outside its bounds and check that the click does not reach an application behind it.
10. Scroll All apps to the last entry using the wheel, touchpad and Page Down/End. Click partially visible tiles, check clipping beneath the fixed search header, then enter a query and verify scrolling resets.
11. Open the clock and check local date, seconds and timezone against the system. Browse December/January and leap-year February, return with Home, use keyboard arrows/Page Up/Page Down and close with Escape or an outside click. Repeat in both themes and with reduced motion. In nested mode brightness and Night light should show Unavailable and leave the host display unchanged.
12. Add/remove home shortcuts from the launcher, dock and home context menus. Restart and verify persistence and independence from dock pins. Fill multiple pages, navigate to the last app, shrink the output and remove apps on the last page. Check icon hit regions while pages animate and repeat with reduced motion.
13. Press Home or Super+H, including while dragging/resizing an app or with the launcher open. Windows should minimize and remain recoverable from the dock, home shortcut or launcher. Opening a running app should restore it without spawning a second instance. A fresh session should show the home screen.
14. Launch apps from home, dock and launcher, including after a long idle period and with slow startup. Check that the window grows from the selected icon without pausing an extra frame, stays visible above the closing launcher, preserves aspect ratio and keeps decorations/popups attached. Restore minimized apps and repeat with reduced motion. Click a moving window and check that the click does not activate an invisible final-position control. Check the home screen has no All apps button or empty-state text, and the calendar has no Today button or calendar icon.
15. Select dark mode and Vibrant, then restart the session and check both are restored on the first frame. Switch back to light and Tonal spot and repeat. Test with a temporary XDG_CONFIG_HOME as well. Saved choices should override startup defaults; a missing config must initialize from defaults and legacy appearance; an invalid config must be preserved and reported while the session uses built-in values.
15. Copy/paste text between two clients, and check applications using D-Bus activation and portal dialogs.

## Direct session

1. Use `--check` to inspect the intended DRM card/connector before entering the session.
2. Start from SDDM or an active local TTY, then repeat the window/input checks above.
3. Verify the advertised refresh on both integer and fractional-rate modes. Look for stalls/tearing during client and shell animations.
4. Switch to another VT while keys/buttons are held, then return. Input state should recover without stuck modifiers or grabs.
5. Disconnect/reconnect the active monitor; verify connector recovery and window/cursor bounds. Only one output is driven at a time.
6. Run fastfetch and compare its display diagonal with the connector dimensions reported by `--check`. Missing physical dimensions must remain unknown; switching VTs must retain the same output identity.
7. Drag brightness and verify actual panel luminance changes without modifying pixel colors or contrast. Compare the slider with `/sys/class/backlight/*/brightness` and `max_brightness`; use hardware brightness keys/external controls and check the slider follows. Simulate denied access and check an error is shown with the actual value restored.
8. Toggle Night light on black, gray, white and colorful windows, including fullscreen/direct-scanout clients and the cursor. Black must remain black, the white point must change gently, and turning it off must restore neutral color. Switch VTs, resume, reconnect and exit; ensure the LUT is restored/reapplied without affecting the other session.
9. Swipe up with three fingers on the touchpad to open the launcher, then down to close it. Try short, horizontal, cancelled, wrong-direction and four-finger swipes. Change launcher state with the keyboard during a swipe and check that its end does not reverse that change. Switch VT during a gesture and unplug/replug the input device; a stale gesture must not open the launcher. Repeat above a fullscreen app. These gestures are provided by libinput in the direct session; nested winit does not deliver them.
10. Maximize CSD and SSD clients, especially when decoration negotiation occurs after the first configure. Resize the output and restore/minimize/reopen windows. Normal and maximized titlebars must stay below the top bar; true fullscreen must fill the output with shell bars hidden. Test F11 entry/exit, opening another window above a fullscreen window and Super+H/Super+Q.
11. Close the compositor normally and verify that the display manager/console regains the device.

Record GPU, driver, connector/mode, client, action and logs with any failure. These checks remain pending until run on the actual graphical environment.

16. Run `materialand --init-config` with a temporary XDG_CONFIG_HOME containing a legacy `materialand/appearance` file. Verify migration of dark/Vibrant and that the legacy file remains untouched. Run initialization again and confirm existing edits are preserved. Validate both the generated file and a partial file with `--check-config`.
17. Customize bar/dock dimensions, window titlebar height, app medallions, text sizes, color roles and spring values in `config.toml`. Restart and check that drawing, click targets, launch origins and maximized placement use the new values together. Toggle theme/palette/reduced motion, then confirm custom layout, colors and comments survive a restart. Check invalid values, typos, unknown schema versions and malformed files through the validator.

18. Initialize a fresh configuration and confirm it contains every built-in section and parameter comments. Change theme/palette/reduced motion through quick settings and confirm the full file, comments and custom values remain. If explicitly using `--compact-config`, compare effective settings before/after, including custom calendar values; verify the exact backup. Enable `input.touchpad.invert_scroll`, restart, and check both axes inside native apps and the launcher. Finger-stop events must preserve kinetic scrolling behavior. Check a mouse wheel and tilted wheel retain their original direction and speed; set the option to false and repeat.

19. On a high-refresh display, verify the direct-session `output initialized` log reports the highest progressive rate at its preferred resolution. Exercise app opening/closing, dock resizing, launcher and sheets, home-page changes, press feedback and repeated theme/palette transitions. Inspect frame intervals with GPU/KMS tracing: the target period is 1000 / refresh_hz milliseconds (6.94 ms at 144 Hz). Check both the first frame after idle and the final settled frame, plus simultaneous app opening and theme changes. Repeat nested on two monitors with different rates and move the host window between them. Check reduced motion, VT pause/resume and idle CPU use. Record actual missed frames and render times on the target hardware; an offscreen/software test cannot establish refresh-rate presentation.

## Tablet shell checks

Automated tests cover distinct gesture directions, navigation press/release behavior, shade scrolling/hit geometry, persistent shortcut order, failed writes, spring motion, split geometry, battery parsing, escaped Wi-Fi names, BlueZ object decoding, stale service updates and command timeouts.

Explicit in-process Wayland integration tests are retained for environments permitting socket IPC:

```sh
cargo test --locked desktop::integration_tests -- --ignored --nocapture
```

They create real SHM clients and exercise minimize/overview/restore, split configure/ack/buffer commits, output resizing, close requests and leaving split by maximizing. They could not complete in the restricted environment described above, so they are not counted as verified client behavior there.

To generate manual shell design previews in `/tmp` without changing the running desktop or system radios:

```sh
cargo test --locked write_tablet_design_previews -- --ignored --nocapture
```

On the target laptop:

1. Verify three fingers up/down opens/closes the launcher; four up opens recent apps; four down opens the shade. Four up while the shade is open must only close it; four down while recents are open must only close recents. Unrelated gestures inside an open panel must not switch layers. Repeat during closing animations and over fullscreen applications. Canceled, short and diagonal swipes should do nothing.
2. Use navigation buttons and Super+P/Tab/S without a touchpad. Enter each overlay while moving/resizing a window or holding a client button; release it and check that input recovers. Escape and outside clicks must dismiss the overlay without reaching the app behind it.
3. Minimize two clients, open recent apps, restore each, close one with unsaved work and handle its confirmation. Check live previews, arrows and fine touchpad scroll. Drag a card upward past the dismissal threshold and release: it must continue upward from the release position, completely leave the output, and send one close request. The next card must slide slowly into the vacated position without a jump when the client disappears. Repeat with the first, middle and last cards, rapid consecutive dismissals, a short canceled drag, and reduced motion. An app that ignores/cancels closing must return as a reachable card after the configured timeout.
4. Pair two clients from overview by clicking both split icons; repeat with a split icon followed by the second preview body. Clicking the first icon again should cancel. If the output is too narrow, selection must remain on the first app with a size notice. Check entry/return at 60 Hz and high refresh: no disappearing titlebar, stacking jump or missing offscreen-to-card window; the small rebound must finish smoothly. Repeat with minimized windows and reduced motion. Grab the divider at its center and near either adjacent CSD/SSD edge, including before new client buffers arrive. Both panes must resize together with their outer edges fixed; grabbing off-center must not jump the divider. Test client-initiated resize at both inner edges, releasing outside the handle, unrelated windows and popups over the gap, minimum sizes, fullscreen/maximize/minimize, closing one member, and output resizing. Exit with Super+BackSpace or right-click the divider.
5. Drag icons within a page, from the launcher, and between pages via edge dwell. Drop outside the grid to cancel. Restart and verify order, including missing/reinstalled apps. Repeat with reduced motion.
6. Compare battery percentage/charging with sysfs. Toggle Wi-Fi/Bluetooth and compare with NetworkManager/BlueZ. Connect a known network and paired device, disconnect, handle a new secured network through settings, and test missing services, permission denial and hardware radio switches. Check errors, busy state and frame pacing while commands run.
7. Check light/dark palettes, small screens, scrolling/clipping, rapid overlay changes and theme reveals. Switch `navigation.mode` to desktop and verify the old topbar/dock, independent pins and all keyboard gestures remain usable.

## Tapping, status area and notifications

1. In a direct session, tap without physically clicking: home/launcher icons, panel tiles and client buttons must receive one click. Check two-finger context taps and tap-and-drag. Reconnect the touchpad and repeat; disable `input.touchpad.tap_to_click` or `tap_drag` and verify the requested setting after restarting. Nested mode follows host configuration.
2. Check the top strip in both themes, at small resolution and with long battery/clock text. Maximize and split clients and confirm titles stay below it. Fullscreen content must use the whole output; opening a shell panel should make its status area accessible again.
3. Tap each status target. Clock must toggle the calendar; radio icons must open the matching list; the bell must open notifications. Taps must be consumed exactly once, including while another sheet is open. Test title/header clipping and scrolled notification close buttons.
4. From a terminal launched inside Materialand, run `notify-send 'Materialand test' 'Message body'`. Check the bell indicator, list order, individual dismissal and Clear all. Test replacing an existing ID, expiry and client withdrawal. Test missing D-Bus without synthetic messages or counts.

An explicit real D-Bus protocol test uses an isolated private session bus:

```sh
cargo test --locked real_session_bus_delivers_replaces_and_withdraws_notifications -- --ignored --nocapture
```

This requires Unix socket and D-Bus permissions. CPU tests cover ownership/routing of gestures, status hit areas, notification ID replacement/expiry/close reasons and bounds. The preview generator also writes status-strip and notification-list images.
