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
2. Open two overlapping clients, including one with client-side decorations and one using server-side decorations. Check content, shadows, title bars, focus and stacking.
3. Open menus/submenus that extend outside the parent window. Move and click inside the protruding part; scroll and dismiss the menu.
4. Check arrow/text/resize cursors, client cursor surfaces and hotspot alignment.
5. Move/resize each edge and corner, including fast resize then immediate release. Check client minimum/maximum sizes and opposite-edge anchoring.
6. Maximize, fullscreen, minimize, restore, close and remap clients. Resize the host window while clients are maximized/fullscreen.
7. Exercise launcher search, dock, quick settings, theme/palette transitions, brightness and reduced motion. Shell overlays must consume clicks/scroll over their bounds.
8. Copy/paste text between two clients, and check applications using D-Bus activation and portal dialogs.

## Direct session

1. Use `--check` to inspect the intended DRM card/connector before entering the session.
2. Start from SDDM or an active local TTY, then repeat the window/input checks above.
3. Verify the advertised refresh on both integer and fractional-rate modes. Look for stalls/tearing during client and shell animations.
4. Switch to another VT while keys/buttons are held, then return. Input state should recover without stuck modifiers or grabs.
5. Disconnect/reconnect the active monitor; verify connector recovery and window/cursor bounds. Only one output is driven at a time.
6. Close the compositor normally and verify that the display manager/console regains the device.

Record GPU, driver, connector/mode, client, action and logs with any failure. These checks remain pending until run on the actual graphical environment.
