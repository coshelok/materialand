# MateriaLand v0.6.0 — softer shapes and spring animations

MateriaLand moves closer to the visual character of Pixel and Material You:
rounded panels, spring motion, a refreshed home screen, and more consistent
app icons. The visual updates also include fixes for window management and
saving settings.

Changes since v0.5.0.

## Appearance

- **Softer shade and app drawer contours.** Edge-to-edge rectangular panels
  are replaced with rounded corners, small screen insets, and a dimmed backdrop.
  On wide displays, quick settings and notifications appear as separate cards.
  Antialiased corners remain intact during scrolling and circular theme reveals.
- **Refreshed home screen.** Layered wallpaper uses the current palette colors.
  Home and app drawer grids are centered and limited in width. On sufficiently
  tall displays, tapping the date above the shortcuts opens the calendar.
- **More readable shade.** Quick settings use capsule shapes, improved typography,
  and a compact layout for narrow screens. Notification text is larger.
  Palette options have moved to a dedicated **Wallpaper & style** page.
- **Consistent app icons.** Original artwork is used on the home screen, in the
  app drawer, dock, server-side window titlebars, and recent apps. Aspect ratio
  and transparency are preserved; missing artwork falls back to Material icons.
  Grid icons sit inside circular containers.
- **Refined interface details.** Search text is left-aligned, and the status-bar
  background follows panel expansion. Recent-app card headers are simpler,
  and the window close button has a more subdued appearance when idle.

## Animations

- The shade and app drawer open with a gentle spring rebound. Overshoot is
  limited to 16 logical pixels, including on large displays.
- Reopening a panel while it is closing preserves its current position and
  velocity. Panels move without stretching text or icons.
- Animation timing now accounts for elapsed time across missed frames instead
  of discarding part of it and artificially slowing movement.
- **Reduce motion** still snaps animations to their final state.

## Fixes and responsiveness

- A close request no longer hides a window that is still running. If an app
  shows a save dialog or declines to close, its window remains accessible.
- Window activation requests are now validated against recent user input,
  its owner, focus, and the expiration of a single-use token.
- Server-side window buttons respond to hover and press. Actions occur on
  release over the original button; moving outside its bounds cancels the press.
- Fixed mouse-button release handling when an overlay opens above a window.
  Improved hit testing for content extending beyond moving window bounds.
- Shade and app drawer hit regions account for rounded corners. Clicking the
  right-hand notification column no longer counts as clicking outside the shade.
- Theme, palette, and Reduce motion settings are saved in the background.
  Rapid changes retain the latest choice; normal session shutdown waits for
  the write to finish. Save failures appear in the shade with **Retry**;
  invalid configuration files are never silently replaced.
- Removed unnecessary synchronous preparation of hidden elements during
  theme changes.

## Installation and upgrading

Added `packaging/install.sh` to install the binary, Roboto fonts, Material Icons,
the icon codepoint mapping, and the session entry. Packaging supports `PREFIX`,
`SESSIONDIR`, and `DESTDIR`; resources can be loaded from the installation
directory without the source checkout.

**Existing configuration files retain explicitly set values.** To enable the
new panel motion, update these values in the existing `[motion]` section of
`~/.config/materialand/config.toml` (or the corresponding file under
`$XDG_CONFIG_HOME/materialand/`):

```toml
sheet_stiffness = 380.0
sheet_damping = 0.72
```

The previous defaults were `300.0` and `1.0`. Validate the configuration with
`materialand --check-config`, then restart the session with the new binary.
Rebound is disabled while Reduce motion is enabled.

## Validation and known limitations

- 224 tests passed locally; 14 tests and generators require explicit execution.
  Additional checks covered GPU rounded clipping and theme transitions through
  offscreen EGL, formatting, Clippy, and the release build.
- Generated 48 static previews across four resolutions in light and dark themes,
  plus videos of panel opening and animation reversal. These are synthetic
  checks, not measurements of smoothness on a physical display.
- Added dedicated CI checks for Wayland, D-Bus, EGL, and resource installation.
  Successful execution on GitHub for this release has not yet been confirmed;
  some protocol scenarios cannot run in the restricted local environment.
- This release does not add panels that track finger movement during swipes
  or new minimize/maximize transitions. Application content and client-side
  titlebars remain styled by the applications themselves.
