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

The rewrite was developed in an execution environment without a usable seat/GPU session; local Unix-socket/TCP IPC operations are restricted. Compilation and pure tests can be checked there. Interactive Wayland protocol exchanges, output pixels and real KMS presentation are not established by those tests.

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
15. Select dark mode and Vibrant, then restart the session and check both are restored on the first frame. Switch back to light and Tonal spot and repeat. Test with a temporary XDG_CONFIG_HOME as well. Saved choices should override startup defaults; a missing or invalid appearance file must not prevent startup.
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
