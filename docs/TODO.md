# Materialand user-facing issues

This is the persistent work list requested by the user on 2026-10-22.
After a context reset, read this file and `git diff` before continuing the current
task. Original request: collect every audit finding likely to frustrate an
ordinary user or lead to an issue, then start fixing them. Fixes are authorized;
publishing GitHub issues, commits, messages, and releases was not requested.

## Rules and priorities

- Prioritize correct window/input behavior and preservation of user work, then
  smoothness, visual consistency, usability, and broader compatibility.
- `[ ]` pending; `[~]` in progress; `[x]` implemented and verified as described.
- "Based on code inspection" does not mean a defect has been observed in a live session.
- Major features (XWayland, monitors, IME, portals) remain explicit tasks;
  partial stubs must not be presented as completed implementations.
- For each fix, record the cause, changes, verification, and remaining limitations.
- Do not replace the user's task with architectural rewrites for their own sake.

## P0 — incorrect behavior and loss of control

- [~] **U01. A window reappears after its close animation.** Based on code inspection:
  `take_closed_windows` removes the fade-out before `send_close`; the live window
  becomes opaque again. Test a slow client, rejected close, save dialog, repeated
  close, and reduced motion. Never hide a save dialog or leave an invisible
  window holding keyboard focus.
- [~] **U02. An app takes focus without user interaction.** The activation handler
  ignores token data. Validate seat/serial/source/age, consume tokens, and remove
  expired ones; legitimate activation must keep working.
- [~] **U03. Clicks or keyboard input reach an invisible or moving window.** Check
  opening, closing, split, recents, popups, and VT switching; rendered and input
  coordinates must agree, and consumed presses must have matching releases.
- [~] **U04. The interface stalls when changing themes or moving shortcuts.** Possible
  cause: reading/validation/`fsync` and cache warming on the main thread. Measure;
  move slow writes off the thread while preserving ordering, errors, and data integrity.
- [x] **U05. Settings save failures are invisible.** Theme/palette errors currently
  go to the log. Show a clear status and a retry action; never silently overwrite
  a corrupted user configuration file.

## P1 — windows and motion

- [x] **U06. Animations slow down when frames are missed.** Entrance caps each step
  at 1/30 s, Spring at 0.05 s. Preserve elapsed motion time after the first frame;
  protection against long idle periods must not stretch ordinary animations.
- [ ] **U07. Minimizing happens abruptly.** Add a consistent minimize/restore/Home
  transition; preserve the client, focus, and access to saved work.
- [ ] **U08. Maximize/fullscreen changes geometry abruptly.** Add an interruptible
  transition synchronized with configure/commit; test returning from fullscreen.
- [~] **U09. The shade, launcher, and recents do not follow finger movement.**
  Continuous progress, distance/velocity settling, cancellation and ownership are
  implemented with CPU regressions. Real touchpad tracking and presentation remain
  to be checked in a direct session; see the 2026-10-28 handoff.
- [ ] **U10. Split stretches text and window controls.** Risk identified in code:
  nonuniform scaling before the new buffer arrives. Reproduce with a slow client;
  coordinate transitions and resizing without distortion or incorrect clicks.
- [ ] **U11. Window corners and shadows are inconsistent with recents.** Define a
  policy for CSD/SSD and normal/maximized/split/fullscreen states; preserve protruding
  popups and transparency, and avoid duplicate borders/shadows. Do not force borders
  on all CSD clients.
- [~] **U12. Server-side window buttons do not respond to hover/press.** Add states,
  trigger on release inside the original button, and cancel on exit/input reset.
- [ ] **U13. Resizing or changing a title causes stutters.** SSD decorations cover
  the entire window with a large transparent area. Measure and split the titlebar
  and shadow into small cacheable elements; test narrow windows and long titles.
- [ ] **U14. The shell is dark, but apps remain light.** Propagate theme preferences
  through proper session-local Settings portal/toolkit integration; do not change
  the parent desktop's settings when running nested.
- [x] **U15. The same app has different icons in the launcher, titlebar, and recents.**
  Use shared app identification and original artwork, with a fallback when missing.
- [ ] **U16. Rapid opening, closing, and theme reversals cause jumps.** Check
  interruptions and reversals of every transition; preserve current pose/velocity,
  the final frame, and reduced motion. Repeated actions must not accumulate animations.

## P1 — responsiveness and background operations

- [ ] **U17. Nested mode consumes CPU/battery while idle.** Replace polling at the
  display refresh rate with event/deadline waits without delaying clients or animations.
- [ ] **U18. Notifications and toggle results appear late.** Worker channels should
  wake the event loop instead of waiting for an idle tick that can take a second.
- [ ] **U19. Bluetooth delays Wi-Fi and vice versa.** A shared worker and external
  commands serialize operations. Separate queues/updates, consider persistent D-Bus,
  and show actual state, errors, and a clear busy indication.
- [ ] **U20. Cannot connect to a new secured Wi-Fi network or a new Bluetooth device.**
  The current interface is limited to nmcli without password entry and paired
  Bluetooth devices. Complete flows for secrets, pairing, and cancellation are needed.
- [ ] **U21. Startup is slow with a large app catalog or heavy icons.** Measure startup;
  bound decoding and resource loading, show fallbacks, and avoid blocking the first
  usable frame.
- [ ] **U22. Memory growth and stuttering at high resolutions.** Measure CPU/GPU caches,
  repeated tile rasterization on press, hidden layers, and damage; set cache budgets.
  Do not claim performance gains without measurements.

## P2 — everyday usability and visual refinement

- [ ] **U23. Newly installed apps do not appear until restart.** Refresh the catalog
  and icons when XDG application directories change.
- [ ] **U24. Incorrect or extra menu entries; names are not in the user's language.**
  Support localized Name, OnlyShowIn/NotShowIn, nested desktop IDs, and correct XDG
  precedence; preserve working Exec/TryExec/Hidden behavior.
- [ ] **U25. Icons have inconsistent visual sizes and weights.** Define a shared
  optical area and padding, preserve original logos, and check real app catalogs.
- [ ] **U26. Shade labels turn into ellipses.** Confirmed in 320 px previews.
  Use adaptive layouts and wrapping; test long Russian labels and text scaling.
- [ ] **U27. Text, emoji, and complex scripts render incorrectly.** Add shaping,
  bidi, fallback fonts, shell string localization, and correct Unicode search editing.
  Rendering currently operates on individual `char` values.
- [ ] **U28. The main shade is overloaded with appearance settings.** Move detailed
  palette selection to a separate page; preserve accessibility and back navigation.
- [ ] **U29. Home and launcher are too similar; large catalogs are awkward to use.**
  Preserve free Home cells; improve search/navigation/grouping without imposing a new UX.
- [ ] **U30. An app fails to launch, but the user does not know why.** Child process
  stdout/stderr are currently suppressed. Provide a clear launch error and accessible
  diagnostics without exposing private data.
- [ ] **U31. Notifications cannot be opened or acted on.** The server currently
  advertises only body/persistence. Implement actions and activation consistently
  with U02; do not advertise a capability before it works.

## P2/P3 — installation and expected desktop capabilities

- [x] **U32. Fonts and icons disappear after installation on another computer.** Add
  an installer and resource paths outside the checkout; test launching from an
  arbitrary working directory without the sources.
- [ ] **U33. The diagnostic version does not match the release.** Release commit
  v0.5.0, Cargo v0.4.0. Use a single version source and a verifiable build/install process.
- [ ] **U34. Everything is tiny on HiDPI; a second monitor does not work.** Implement
  scaling and multiple outputs with consistent input/render/window coordinates;
  a separate work phase.
- [ ] **U35. Legacy/X11 apps do not launch.** Add XWayland and window/input integration
  as a separate phase; native Wayland is currently a documented limitation.
- [ ] **U36. The tablet interface cannot be controlled with a touchscreen.** Touch
  input, ownership/cancellation, an on-screen keyboard, and text-input/input-method;
  a separate phase.
- [ ] **U37. The session cannot be securely locked.** Implement session locking with
  correct crash/VT-switch behavior; do not substitute a decorative panel for a lock.
- [ ] **U38. Screen sharing, recording, and some portal flows do not work.** Test file
  picking and activation on a private bus; implement the required portals and capture
  with clear user choice/permission; a separate phase.
- [ ] **U39. Regressions are discovered only after users update.** Enable integration
  Wayland/D-Bus/EGL tests in suitable CI jobs and visual regression checks at reference
  sizes/themes; maintain a separate manual GPU/VT/hotplug/real-app matrix.
- [ ] **U40. Hardware/nested-mode limitations are unclear.** Make disabled/unavailable/
  error states consistent for brightness, Night light, radios, and gestures; show
  a brief reason and an available action without fake toggles.

## Verification matrix

- GTK CSD, Qt, Electron/browser, SSD terminal, transparent terminal, video.
- Long titles, popups outside parent bounds, subsurfaces, save dialogs, slow clients,
  rejected close requests, and multiple windows from one app.
- Light/dark, compact/wide, normal/maximized/fullscreen/split/recents.
- 30/60/144+ Hz, irregular intervals/missed frames, reduced motion.
- Real DRM/VT/hotplug and nested sessions; mockups do not replace these checks.

## Continuation / handoff

2026-10-28: the user resumed U09 and specifically requested sequential shade reveal.
Implemented continuous three-finger shade/drawer and four-finger recents tracking
through direct-session libinput events. Holding freezes progress; reversal stays
with the same panel. Release uses distance and recent velocity (discarded after a
100 ms pause); cancellation restores the starting open/closed state. Keyboard,
pointer, axis input, seat reset and output resize cancel an active drag. Reduced
motion retains the threshold action without intermediate animation.

Fullscreen tablet shade content now stays at native size while a rounded contour
expands downward. Header, brightness, tile rows and notifications fade in by
expansion phase; closing reverses the same phases. Hidden content cannot receive
clicks. Cached rasters are reused; settled frames retain their original draw count.
Implementation: `src/input/gestures.rs`, `src/shell/shade_reveal.rs`, shell/overview
motion APIs and input interruption hooks. U07/U08 remain subsequent work.

Verification: 229 tests pass, 14 explicitly ignored; Clippy all-targets with warnings
denied and the explicit offscreen EGL reveal test pass. Regressions cover hold,
reversal, cancellation, foreign-device events, velocity versus paused release,
interruptions, reduced motion, resize, cache reuse, native proportions and hit
bounds. Three synthetic motion previews cover shade/drawer spring motion and a
scripted shade drag/hold/reversal (`target/visual-audit/expressive-*.mp4`); README
GIF refreshed. These are painter previews, not live touchpad captures or FPS
measurements. U09 remains `[~]` pending the direct-session checks in
`docs/VALIDATION.md`. Do not repeat implementation on context recovery.
Release binary rebuilt at `target/release/materialand`; `--help` and
`--check-config` pass. Formatting and `git diff --check` pass; all three motion
videos pass complete ffmpeg decoding. No session restart or installation performed.

2026-10-23: U01/U02/U12 are implemented, pending execution of the new protocol tests
in a normal environment. U06 is verified by CPU tests. U15 is implemented and checked
with CPU/pixel tests and a static preview of actual installed icons. U32 is verified
through isolated font loading and staged installation. U03 fixes an old release
reaching a new overlay; the full transformed-input matrix is still needed.
U03 also accounts for transformed hits on content outside xdg geometry; the new
protocol test still requires a normal environment. U04/U05: appearance writes moved
to a worker, with errors/Retry implemented in the shade header. Unnecessary prewarming
was removed from theme handlers after measurement. Next for U04: first-frame cost
of a new theme and workspace/pin writes; U18 covers waking the loop when background
writes complete. U03 still needs full popup/grab/split verification in a session.
Do not repeat completed implementation work. Other major phases have not started.
The initial working tree was clean. Initial audit checks: fmt/Clippy passed,
208 unit tests passed, 7 ignored. Two additional offscreen EGL tests passed.
Wayland tests hit `Operation not permitted` on socket sendmsg; private D-Bus does
not start in the sandbox. The preview generator passed (82.94 s); its images in
`target/visual-audit` use synthetic clients and a fixture app catalog.

Before implementing more changes, read the relevant code/tests: the earlier audit
contains hypotheses and observations, not an unconditional solution specification.
Do not hide unsaved work for the sake of an attractive close animation. Record the
results of each completed task below.

## Work log

- Created this list and the root `AGENTS.md`, directing future work here.
- U01: removed speculative fade-out of a live client. Close is sent immediately,
  and the window stays accessible until unmap/destroy. Added a protocol test for
  rejection, repeated close, save dialogs, and reduced motion; execution requires
  a normal environment. Snapshot animation of a confirmed disappearance remains
  part of U16.
- U06: removed step caps from active analytic springs/entrance/ticker; the first
  frame after idle remains bounded. Regressions for 10–360 Hz, irregular intervals,
  and long pauses pass. Full suite after U01/U06: 210 passed, 8 ignored.
- U02: added a history of actually delivered input, owner/seat/serial/focus checks,
  a 5 s lifetime, bounded single-use tokens, and reset on focus changes, VT switches,
  or entering an overlay. Two unit tests pass; the new client protocol test compiles
  but needs execution outside the sandbox. Changes remain under overall review.
- U39: added a separate CI job for ignored Wayland/D-Bus/EGL tests, excluding local
  preview generators. Remote CI has not run; visual regression is not implemented yet.
- U12: added hover/press states for SSD buttons, with actions only on release over
  the original button of the same window, and cancellation on exit/overlay/reset/
  destruction. Two CPU tests pass; the protocol test for cancellation, overlay,
  and exactly one close compiles. After U12: fmt/Clippy passed, 214 passed, 10 ignored.
- U15: shared resolver for dock/SSD/recents, with exact desktop ID taking precedence
  over name/command matches; original icons preserve aspect ratio/alpha, with the
  existing fallback when missing. Launcher uses the same painter. Two CPU/pixel
  tests pass. The installed-icon generator passed: 28/32 icons; inspected
  `target/visual-audit/installed-app-identities.png`. These are real icons on
  synthetic cards, not a session capture.
- U32: added `packaging/install.sh` (PREFIX/SESSIONDIR/DESTDIR), resource lookup in
  XDG data directories and beside the installed binary before falling back to the
  checkout. The installer bundles Roboto, Material Icons, and codepoints together.
  Added a loading check from an isolated installation tree without source/system-font paths.
- U32: the loading test passed; staged installation into `/usr/local` and a custom
  prefix containing a space passed from `/tmp`: all resource files and the desktop
  entry were compared, and the installed binary runs `--help`. A real session without
  the checkout has not been run. CI now also checks staging; remote CI has not run.
- U03: opening an overlay could route the release of an already held button to the
  new launcher's scrim after clearing press ownership. That release is now consumed
  once; VT reset clears the flag so the next click is not lost. The new CPU regression
  passed; the U12 protocol test uses the actual overlay-reset path.
- Before pausing at the user's request, built the release: `target/release/materialand`;
  fmt/Clippy, 218 CPU tests, and 2 EGL tests passed at that point, with 10 ignored.
  Later checks and rebuilds are recorded at the end of this log.
- U03: the inverse presentation transform uses the same rounded anchors/offsets as
  rendering. Protruding popups/subsurfaces/CSD are checked in source coordinates so
  clicks during entrance/split do not pass to the window underneath. The CPU transform
  regression passed; the new protocol scenario with content outside xdg geometry
  compiles, but execution in the restricted environment is unconfirmed. Full scaled
  input delivery was not added: clicks are still consumed during motion.
- U04/U05: the bounded appearance worker keeps only the latest pending write,
  serializes parse/validation/fsync/rename outside the UI, and waits for the final
  write on normal shutdown. Stale results are ignored. The shade shows the failure
  reason and Retry, preserving invalid configuration files; status clears only after
  the current write is confirmed. Tests passed for blocked writes/1000 requests/
  ordering/shutdown, stale failures, invalid TOML and recovery, plus Retry hit testing
  at 320/1280, including notifications/scrolling. Generated four
  `appearance-error-{light,dark}-{320,1280}.png` images and inspected compact light.
  The error is visible in the shade; there is no separate toast. Worker results are
  still collected on tick; a dedicated wakeup remains U18. Workspace/pin writes are
  still synchronous, so U04 is not fully complete.
- U04: removed synchronous prewarming from theme/variant handlers: it painted the
  topbar/dock hidden in tablet mode and layers before actual frame construction.
  Explicit `measure_theme_input_preparation` measurement, debug CPU, 1280×800,
  without windows or disk writes: before the change, first switches took 1.37–1.41 s
  (home), 532–537 ms (shade), and warmed calls about 1 ms; afterward, 0.81–0.85 ms
  across all six calls. This measures input handler cost, NOT total frame latency
  or physical FPS. Visible layers are still prepared in `build_ui`; cold rasterization
  remains a separate U04/U22 concern. The profiling test is excluded from integration CI.
- Results of this pass: fmt/Clippy and `git diff --check` passed; 222 tests passed,
  13 ignored (protocol/D-Bus/EGL tests, preview generators, and CPU timing). Two EGL
  tests passed separately. The existing configured_session test now waits for the
  worker before reading the saved file; functional save checks are preserved.
- Built the updated release after U03/U04/U05 (`cargo build --release --locked
  --offline`, 2m15s); ran `target/release/materialand --help`. No system installation
  or graphical session launch was performed.
