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

This checks shader compilation, radial and rounded clipping independently and together, antialiased edges, alpha blending, scaled/cropped sources, framebuffer orientation and shader state isolation. It does not measure physical display FPS. CPU regressions cover native-resolution refresh selection, 30–360 Hz/fractional animation intervals and retention of both cached theme endpoints.

For sheet motion, run `cargo test --locked --offline write_expressive_motion_previews -- --ignored --nocapture` with ffmpeg installed. Inspect the videos and trajectory CSVs in `target/visual-audit`: opening, bounded rebound, closing and reversal before closing finishes. In a real session, also check rounded corners during scrolling, clicking both shade columns, rapid theme changes and reduced motion. The synthetic videos do not verify live frame pacing or gesture tracking.

The generator also writes `expressive-shade-gesture.mp4`, which scripts a drag,
hold and reversal through the shell progress API. Check that the contour expands
downward and that the heading, brightness and tile rows appear in sequence, with
native-size text. This is a visual fixture, not recorded libinput input.

Live recent-app corners have a separate offscreen GPU regression:

```sh
cargo test --locked egl_rounded_previews_preserve_clipping_transforms_and_shader_isolation -- --ignored
```

It checks actual rounded clipping and alpha with scaled/cropped sources, both direct/nested framebuffer orientations, and shader isolation. Check real client snapshots, app identity/actions, neighboring card depth and upward dismissal in a normal session. Check the home search capsule, centered, width-limited home/drawer grids and available top-left cells, and full-height drawer/shade in both themes. Existing gesture ownership and split behavior must remain intact.

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
   Also drag down with three fingers to open the shade and up with four to open
   recents; reverse to dismiss each. Hold halfway without lifting: the panel must
   stay still. Reverse before release, compare a short fast fling with a slow drag,
   and pause before releasing to discard old velocity. Cancel an opening and a
   closing gesture, interrupt with a key/click/two-finger scroll, resize the output,
   and try a second touchpad. No panel should remain stuck halfway or switch to a
   different overlay. Repeat with reduced motion. Check both shade columns and
   ensure controls below the expanding contour cannot be clicked.
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

To generate 40 manual shell design previews in `target/visual-audit/` without changing the running desktop or system radios:

```sh
cargo test --locked write_tablet_design_previews -- --ignored --nocapture
```

The previews cover home, drawer, shade, notifications and recents in both themes at 320×640, 640×480, 1280×800 and 1920×1080. They use fixture applications, system status and labeled preview-client surfaces, with the same cropped layer geometry as rendering. They establish static layout, not animation frame pacing or real client behavior.

CPU regressions check full-width cells, unreserved top-left cells, sparse-position persistence, occupied-cell swaps, new-page creation and resized-output collision handling, stable icon/background caches during scrolling, fixed shade-header geometry, scroll clamping after notifications disappear, and spring trajectories at 30–360 Hz including fractional rates and reversal. The reveal EGL regression exercises the actual layer-upload path with nonzero source crops.

During manual review, reverse opening/closing sheets repeatedly, scroll long network and notification lists, and click the fixed headings and visible rows. Clear notifications while still scrolling. Check that no stale offset, exposed panel edge or click-through remains. Toggle themes while recents is visible and inspect both its header and card chrome.

On the target laptop:

1. Verify three fingers up opens the launcher and three down opens the shade. Three down while the launcher is open must only close it; three up while the shade is open must only close it. Four up/down opens/closes recents; four down from home must do nothing. Unrelated gestures inside an open panel must not switch layers. Repeat during closing animations and over fullscreen applications. Canceled, short and diagonal swipes should do nothing.
2. Use bare Super and Super+P/Tab/S without a touchpad. Enter each overlay while moving/resizing a window or holding a client button; release it and check that input recovers. Escape and outside clicks must dismiss the overlay without reaching the app behind it.
3. Minimize two clients, open recent apps, restore each, close one with unsaved work and handle its confirmation. Check live previews, arrows and fine touchpad scroll. Drag a card upward past the dismissal threshold and release: it must continue upward from the release position, completely leave the output, and send one close request. The next card must slide slowly into the vacated position without a jump when the client disappears. Repeat with the first, middle and last cards, rapid consecutive dismissals, a short canceled drag, and reduced motion. An app that ignores/cancels closing must return as a reachable card after the configured timeout.
4. Pair two clients from overview by clicking both split icons; repeat with a split icon followed by the second preview body. Clicking the first icon again should cancel. If the output is too narrow, selection must remain on the first app with a size notice. Check entry/return at 60 Hz and high refresh: no disappearing titlebar, stacking jump or missing offscreen-to-card window; the small rebound must finish smoothly. Repeat with minimized windows and reduced motion. Grab the divider at its center and near either adjacent CSD/SSD edge, including before new client buffers arrive. Both panes must resize together with their outer edges fixed; grabbing off-center must not jump the divider. Test client-initiated resize at both inner edges, releasing outside the handle, unrelated windows and popups over the gap, minimum sizes, fullscreen/maximize/minimize, closing one member, and output resizing. Exit with Super+BackSpace or right-click the divider.
5. Drag icons into distant empty cells, onto occupied cells and from the launcher. Verify only the two occupied cells swap and unrelated icons stay put. Drag to the right edge of the last page to create a page. Drop into the top-left cells; drop outside the grid to cancel. Remove an icon and restart: holes and distant positions must remain. Resize the output and return to its original size; check missing/reinstalled apps and saved positions. Repeat with reduced motion.
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

## Rewritten workspace and tablet layout

Check status-strip time at 00:00, 11:11 and 23:59 and battery at 9%, 67% and 100% in both themes: text must stay vertically centered and fit beside icons. Click the time to open the calendar. Confirm the home grid has no duplicate clock and no reserved cells. On wide outputs, scroll notifications without moving controls, then scroll controls without moving notifications; dismiss a visible notification after scrolling. Back returns from connection lists before closing the shade.

In tablet recents, use all four arrow keys and Enter, move across page boundaries, dismiss cards in each row and verify smooth reflow. Repeat split selection with two cards in different rows and pages. Real client configure/ack timing and physical frame pacing still require a normal KMS or nested session.

To preview actual installed application icons, run `cargo test --locked write_installed_icons_preview -- --ignored --nocapture`; the image is saved as `target/visual-audit/installed-app-icons.png`. Icon regressions cover theme inheritance/cycles, hicolor and pixmap fallback, absolute paths, transparent PNG/SVG decoding and invalid images. Verify Firefox, Cursor and other installed apps in the launcher and on home; absent icon files must keep the fallback. Check the main shade has no redundant bell and that the status-strip bell still opens notifications.

## Touchpad scrolling and dragging

CPU tests cover direct finger tracking, release inertia, refresh-independent decay, stale releases, reduced motion, boundary stopping, per-device/list ownership, overlay interruption and one home-page turn per horizontal gesture. Gesture routing checks every combination of initial panel and three/four-finger direction.

On the laptop, flick long launcher, network and notification lists, touch again to stop, then click an app. Move the pointer between shade columns during a scroll: the original column must keep ownership. Open another panel mid-scroll and verify the old sequence does not scroll it. Check horizontal two-finger home paging, direction inversion and vertical/diagonal motion. Test tap-and-drag on shortcuts, recents dismissal, the split divider and brightness slider; lift/reposition the finger and tap to finish, then repeat with drag lock disabled. Verify typing suppression and touchpad reconnects. These hardware interactions require a direct session; automated tests do not establish the physical pad's behavior.

Bare Super regression: press/release either Win key to minimize all windows and show home. Hold Super and use P, Tab, S, or a client shortcut; releasing Super must not then show home. Check both release orders, Shift held before Super, and a VT switch mid-press. Verify the old Back/Home/Recents area has no visible buttons or hidden click targets and that maximized windows and fullscreen sheets use the freed bottom space.

## User-facing backlog regressions

CPU tests cover elapsed animation time at 10–360 Hz and across dropped frames,
activation input ownership/expiry/one-time use, and titlebar press/release
cancellation. The ignored Wayland suite additionally checks a client rejecting
close, a save-dialog toplevel, activation-token replay and titlebar releases over
an overlay. CI's `integration` job runs ignored Wayland/D-Bus/EGL tests with Mesa
software rendering, excluding local preview writers. A configured job is not
evidence of a successful CI run.

Check server-side titlebar buttons with a real client: hover, press, leave and
return, release outside, open a shell overlay while holding a button, and switch
VT. Close must reach the client only on a matching release; a rejected close
must leave the content and save dialog usable. Ordinary close currently waits
for the client's unmap without an exit animation; a safe snapshot exit is
tracked in TODO U16.

`write_installed_icons_preview` also writes
`target/visual-audit/installed-app-identities.png`, comparing launcher icons with
recent-app headers. These are installed icons in synthetic cards. CPU pixel
tests verify that the original icon reaches a card, preserving aspect/alpha,
and exact desktop IDs beat executable aliases. Check actual app IDs and icons
in SSD titles, dock and recent apps in a live session.

For packaging, build release and run `DESTDIR=/absolute/staging sh
packaging/install.sh`. Verify the binary, Roboto weights, Material Icons,
`codepoints` and session entry. Run the staged binary from an unrelated cwd.
The `installed_resources_follow_xdg_and_relocated_binary_without_checkout` test
loads fonts and canonical icon names from a temporary installation, without
supplying checkout or system-font roots. A real graphical session after removing
the checkout remains a separate manual check.

Appearance-write regressions hold the worker behind a deterministic gate while
queuing 1000 requests, then verify coalescing, ordering and shutdown draining.
They also check stale results, invalid TOML preservation, retry after repair,
and the Retry hit target in the fixed shade header at 320/1280 px while scrolling
or viewing notifications. The error remains until the latest write succeeds.
Run `cargo test --locked write_appearance_error_previews -- --ignored --nocapture`
to generate four light/dark, compact/wide error previews in `target/visual-audit`.
In a real session, deny config writes, change the theme, restore permissions and
press Retry; then restart and verify the final preference. Repeat rapid changes
followed immediately by normal logout. Abrupt process termination cannot drain
an unfinished background save.

`cargo test --locked measure_theme_input_preparation -- --ignored --nocapture`
times the theme input handler with no live windows or disk writes. It does not
measure rendering/presentation or guarantee a smooth first frame; scene assembly
still prepares cold visible layers. The timing helper is excluded from CI.
