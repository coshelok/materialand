# Materialand code audit and rewrite plan

## Scope

This review covered the project manifest, README, packaging entry, font notice, and all 13 Rust modules in `src/`. It was a source audit against the checked-out GitHub tree; the environment used for this review could not clone the repository or run Rust locally. Consequently, the findings below are source-level findings and the branch has **not** been build-tested or exercised in a live Wayland session.

Materialand is a compositor prototype, so the aim is not to replace working Smithay protocol machinery with custom implementations. The aim is to make the application-owned code smaller, make coordinate and event-routing rules explicit, and let Smithay own protocol details wherever possible.

## High-confidence defects addressed in this branch

- **Pointer frames were never sent, and scroll events were discarded.** Input forwarded `motion` and `button` through Smithay but did not send `pointer.frame()`. Smithay's [Anvil input handler](https://github.com/Smithay/smithay/blob/v0.7.0/anvil/src/input_handler.rs) ends forwarded pointer events with a frame. This can make client input appear stale or unresponsive. Motion, forwarded presses, releases, and scroll events now terminate the frame; both nested and DRM/libinput backends route scroll input.
- **Clicks could be intercepted by a toplevel with no surface under the pointer.** A toplevel could be mapped before receiving its first buffer, and the click path did not consistently follow the same content/surface-under test used by rendering. Hit testing now skips windows without content and continues to lower windows when a non-fullscreen surface/input-region hit misses.
- **A nested compositor could issue host power actions.** The shell exposed `systemctl poweroff/reboot` even in `--nested`, which is a development window rather than an owning session. Those operations are now ignored in nested mode; logging out only closes Materialand.
- **Child processes could bypass the chosen Wayland socket.** `WAYLAND_SOCKET` takes precedence over `WAYLAND_DISPLAY`, and was inherited untouched. Child processes now remove it.
- **Multiple compositor instances could collide on one socket.** Both backends used the fixed `materialand-0` name. Each instance now binds a PID-scoped socket name and launches clients with that actual name.
- **Runtime directory validation only checked for an absolute path.** An invalid or unsafe `XDG_RUNTIME_DIR` could be accepted and then fail socket creation or expose Wayland sockets. Startup now checks directory type, owner UID, and group/other permissions before reusing it.
- **Nested or secondary sessions could reuse a host desktop process through D-Bus activation.** Merely changing `WAYLAND_DISPLAY` does not change the environment of an already-running app. The compositor now tries to start a private D-Bus session and removes inherited bus-starter metadata. If `dbus-daemon` cannot be started, the host bus address is replaced with an unreachable Unix-socket address; it is not left unset, because some clients may auto-discover the host bus. This still cannot prevent applications from forwarding requests through their own shared-profile IPC.
- **DRM refresh was advertised in the wrong units and lost fractional rates.** `wl_output.mode.refresh` uses millihertz, whereas the old code passed the integer hertz value. The compositor now calculates millihertz directly from DRM clock/timing totals and accounts for interlaced, double-scan and vertical-scan modes instead of rounding to whole Hz first.
- **Window surface stacking could be inverted inside a single window.** Each window's surface-tree element group is reversed before the whole scene list is reversed for rendering.

## Structure and complexity findings

### 1. Shell is doing too many jobs

`src/shell.rs` is roughly 3,170 lines. It mixes shell state, animation state, bitmap caching, layout geometry, painting, and hit-test calculations. That makes visual changes capable of breaking hit testing and makes cache invalidation hard to review.

**Target structure:**
- `shell/model.rs`: visible shell state and actions.
- `shell/layout.rs`: pure geometry, layout and hit-test functions shared by drawing and input.
- `shell/animation.rs`: springs/tweens and animation lifecycle.
- `shell/render.rs`: Material 3 painting and buffer cache policy.
- Keep rendering and hit testing coupled through shared layout data, not duplicated formulae.

### 2. State mixes protocol glue with policy

`src/state.rs` owns the Smithay protocol handler implementations and also decides window activation, saved geometry, maximize/fullscreen/minimize behaviour, popup constraints, animation lifecycle and app metadata. These are legitimate responsibilities, but they are interleaved in one global state type.

**Target structure:**
- a small protocol-facing `CompositorState` that implements Smithay handlers;
- a `WindowManager` with focused window, stacking, geometry and lifecycle;
- a `SurfaceHitTest`/coordinate helper that is the single authority for surface origins and input coordinates;
- protocol handlers translate requests into window-manager operations instead of duplicating state mutations.

### 3. Input routing is manual and repeated

`src/input.rs` contains keyboard shortcuts, shell hit testing, client surface hit testing, pointer event delivery, move/resize state, process spawning and system actions. Several client-click branches each recreate motion and button events, so a missing `pointer.frame()` escaped across the code.

**Target structure:**
- one shell-first hit-test pass that yields a typed target (`ShellControl`, `WindowDecoration`, `ClientSurface`, or `Desktop`);
- one function that forwards a client pointer event and always closes its frame;
- shell actions handled by a separate dispatcher;
- session and process actions passed to explicit services rather than calling commands from input code.

### 4. The two backends have different event-loop policy

`src/main.rs` owns the Winit nested loop and `src/session.rs` owns the calloop/DRM/libseat lifecycle and loop. They separately handle socket acceptance, client dispatch, redraw scheduling, output state and frame callbacks. This naturally creates differences between `--nested` and hardware mode.

**Target structure:**
- shared client dispatch and compositor tick logic;
- backend-specific code limited to device events, framebuffer acquisition/submission and presentation timing;
- a single explicit coordinate space for compositor logical coordinates, output physical coordinates and buffer coordinates;
- shared integration tests exercised against both backends where possible.

### 5. Configuration and launching are conflated

`src/config.rs` combines XDG desktop-entry scanning, an `Exec` tokenizer, keyboard settings and compositor shortcuts. The custom desktop-entry parser is deliberately small and should either be clearly scoped as a supported subset or replaced with a standards-aware parser. Client launch policy should be separate from desktop-file parsing and should make nested-session isolation explicit.

### 6. Output/session setup needs explicit failure and recovery contracts

`src/session.rs` does GPU discovery, connector/mode selection, libseat pause/resume, calloop registration, KMS setup and rendering. These operations have distinct lifetimes. A more modular backend state machine would make resume, hot-unplug and partial initialisation easier to test than a set of loosely related optional fields.

## Rewrite sequence

This is intentionally staged; rewriting all of a compositor before validating input and protocol semantics would discard a substantial amount of working Smithay integration and make regressions difficult to isolate.

1. **Input and session safety** (this branch): close pointer frames; avoid false surface hits; use isolated sockets and D-Bus; prohibit host power operations in nested mode.
2. **Coordinate contract**: document and test the conversion between output logical coordinates, physical framebuffer coordinates, window geometry and surface-local coordinates. Reproduce the cursor offset in nested mode before changing renderer transforms.
3. **Window manager extraction**: centralize focus, stacking, geometry, popup, maximize/fullscreen/minimize and closing state; add pure tests for transitions.
4. **Shell decomposition**: split layout/animation/rendering without changing appearance in one commit; make every control's visual bounds and hit bounds come from the same layout object.
5. **Backend convergence**: unify client dispatch and redraw policy for Winit and DRM while preserving backend-specific presentation.
6. **Desktop integration and protocol coverage**: robust app launching, XDG activation/portals, keyboard/pointer frames, drag and drop, output-management and optional XWayland where desired.

## Still requiring manual verification

- Reproduce cursor offset and validate the final coordinate-transform policy in a live `--nested` window; this branch does not yet claim to have fixed that symptom.
- Test apps that reuse an already-running process or share profiles (Firefox, VS Code/Electron, browsers, file managers). Private D-Bus reduces activation leakage but is not a sandbox and shared IPC may still cross session boundaries.
- Confirm that required portal services and file dialogs work with the private D-Bus session. This isolation may expose assumptions in desktop services that expect the user-manager bus.
- Run `cargo fmt --check`, `cargo check --all-targets`, `cargo clippy --all-targets`, and `cargo test` on Linux with Smithay's native dependencies; test both `--nested` and DRM/TTY mode.
- Test actual pointer presses, release, menus, popups, transparent/input-excluded regions, move/resize, client decorations and multiple overlapping windows.

## Scope honesty

This branch is a first repair/refactor pass, not a completed clean-room rewrite. No changes have been merged to `main`; review and run the validation steps above before relying on it for a daily-driver session.
