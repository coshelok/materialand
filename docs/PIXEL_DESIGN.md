# Pixel visual pass

The user paused the engineering backlog on 2026-10-23 and requested a visual
redesign toward Google Pixel. Do not resume TODO priorities instead of this task.

Working reference: Pixel / Material You, adapted to a wide desktop. The user's
latest direction specifically calls for soft shade contours and springy movement
rather than the previous restrained motion. Reference material:
https://store.google.com/magazine/pixel-recommended-settings?hl=en-US
https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/

Implement in compositor code, not a separate mock website: deliberate wallpaper,
bounded app grids, functional date/calendar, Pixel-like app-search capsule,
readable quick controls and a separate appearance page. Preserve real app logos,
native client decorations, saved shortcut identities and matching input geometry.
No fake weather, Google services, disconnected toggles or demonstration-only UI.

Review light/dark at 320×640 and 1280×800, plus compact landscape; generate
previews from actual shell painting. These are synthetic shell previews, not
live app or frame-pacing verification. Keep existing engineering fixes intact.

## Implemented, 2026-10-26

- Procedural layered wallpaper uses dynamic primary/secondary/tertiary colors in
  light and dark schemes; no external image download or new runtime dependency.
- Home and All apps have centered maximum widths. A real date opens the calendar
  above the home grid on taller screens; compact landscape preserves grid space.
  Search text is left aligned. Saved cell identities and drag/drop remain intact.
- Original app artwork is fitted inside consistent circular containers; fallback
  glyphs still use theme colors. Raster logos are neither recolored nor cropped.
- Quick controls use capsules, larger text on wide tiles, and a stacked compact
  layout. Appearance variants moved into a separate Wallpaper & style page with
  its own scrolling, cache key, back action, Escape and status-bar navigation.
- Notification summaries/body text are larger. The status-strip backing follows
  fullscreen drawer/shade progress, avoiding a contrasting scrim strip.
- Recents use an understated identity header with focus indicated by title color;
  live client card geometry and actions are unchanged. SSD close controls use a
  neutral idle color and error accent on hover/press. CSD stays client-owned.

## Verification and continuation

The existing engineering backlog remains paused. Keep this visual direction as
current work if the user asks to continue. Do not claim pixel-perfect Android or
restyling of application-owned toolbars: GTK/Qt/Electron content and real-session
motion still need separate visual assessment.

`cargo test --locked --offline` passed 223 tests (13 explicitly ignored). The new
input regression checks opening/back/reset and palette targeting at 320×640,
640×480 and 1280×800 without firing hidden quick controls. Home resize regression
continues checking all 95 shortcuts remain unique and reachable. Formatting and
Clippy with warnings denied passed. The 48-image preview sweep also passed;
manual inspection covered narrow and wide light/dark home, shade, style and
drawer, landscape home and wide recents. A separate installed-app preview
loaded original artwork for 28 of 32 discovered applications (the rest use
fallback glyphs).

Preview command:
`cargo test --locked --offline write_tablet_design_previews -- --ignored --nocapture`

It paints home, drawer, quick settings, style, notifications and recents at four
sizes (320×640, 640×480, 1280×800, 1920×1080), in light and dark: 48 PNGs under
`target/visual-audit`. Earlier images are retained in `before-pixel/`. These use
fixture apps/service data through the actual shell painter; they are not captures
of a live Wayland session. Release binary is built at `target/release/materialand`;
no installation, compositor restart, commit or publication was requested.

Final check: release build succeeded after the desktop-page overlap fix; the
focused style-page input regression, Clippy (`--all-targets -D warnings`), format
check and `git diff --check` all passed. The engineering TODO was not modified.

## Soft contours and spring motion, 2026-10-27

User priority: the shade looks angular and motion feels linear; continue this
visual work, leaving the engineering TODO paused.

Implemented:
- Fullscreen tablet shade and drawer now have inset rounded contours (8 px inset
  at 320 px width, up to 16 px; default radius 40 px). The wide shade retains two
  distinct cards. A dim wallpaper scrim replaces the opaque full-output slab.
- GPU clipping combines the rounded sheet boundary and circular theme reveal in
  one shader. Header, scrolling body and app tiles share the moving contour;
  cached textures are reused. Edges are antialiased. Shader failure logs a warning
  and falls back to ordinary layers.
- Sheet spring defaults are stiffness 380, damping 0.72. Native-size content
  slides with a rebound softly capped at 16 logical px, including tall displays.
  Retargeting preserves motion; reduced motion still snaps. Gesture-following,
  app maximize/minimize animation and client-owned decorations are not changed.
- Rounded input bounds match the contour. The outside-click precheck now accepts
  both shade columns; previously a right-column notification click could close
  the whole shade. Desktop-mode shade input retains its own 32 px radius.
- Updated only the two sheet-motion values in the user's existing config. Backup:
  `/home/coshelok/.config/materialand/config.toml.before-expressive-20261027`.
  No compositor restart or installation performed.

Verification: 224 unit tests pass, 14 explicitly ignored. The explicit EGL
reveal test passes with rounded-only, circle-only, combined and unmasked cases,
cropped/scaled textures, both framebuffer orientations, alpha and antialiased
edges; the next plain layer is unaffected. New regressions cover rebound at
320×640, 1280×800 and 3840×2160, reversal, native content proportions, reduced
motion, corner hit testing and both notification columns.

Preview commands:
```sh
cargo test --release --locked --offline write_expressive_motion_previews -- --ignored --nocapture
cargo test --locked --offline write_tablet_design_previews -- --ignored --nocapture
```
The first needs ffmpeg and writes `target/visual-audit/expressive-{shade,launcher}.mp4`,
selected PNG frames and trajectory CSVs. It samples the actual shell spring at
60 Hz into 30 fps video, with open, close, interrupted close/reopen, then close.
These are synthetic painter previews, not live Wayland captures or timing tests.
The second refreshes the 48-image resolution/theme/page sweep.

Completed: the 48-image sweep passed; inspected narrow light and wide light/dark
shade contours and drawer frames. Both motion videos pass complete ffmpeg decode,
each 640×480, 108 frames, 3.6 seconds. Sampled first-opening overshoot is 13.09 px
for the shade and 12.36 px for the drawer. The exporter crops the grid-rounded
wallpaper cache to the output size before encoding. Release-profile preview
generation passed (3.17 seconds excluding compilation); debug painting is much
slower and should not be interpreted as compositor performance.

Release binary built at `target/release/materialand`; `--check-config` and `--help`
pass. Formatting, Clippy all-targets with warnings denied, and `git diff --check`
pass. A comparison against the saved config backup confirms exactly two changed
values: sheet stiffness and damping. The changes take effect when the new binary
starts; no running session was restarted. Real-session appearance and frame
pacing still require assessment on the user's display. Engineering TODO untouched.

Release preparation: the user reported that the shell is usable after personal
testing and selected v0.6.0 instead of the proposed beta. Cargo package and lockfile
now use 0.6.0; `docs/RELEASE_NOTES.md` describes the regular v0.6.0 release relative
to the local v0.5.0 release commit `5c89b3b` (no local release tags). This report
does not establish compatibility across different hardware or client toolkits.
