# Materialand v0.2.0

## Dock and application launcher

- Added spring animations for dock expansion, contraction, icon movement and appearance. Hit regions follow the animated geometry; reduced motion snaps directly to the final layout.
- Kept running windows in creation order so focusing or minimizing a window does not reorder the dock.
- Added **Pin to dock** and **Unpin from dock** through a shared right-click menu in the dock and launcher. Choices persist across sessions, including an intentionally empty list of pins.
- Running pinned applications occupy their existing launcher slots. Closing the application restores its launcher instead of removing the pin. Additional windows remain accessible as separate entries.
- Saved pin identities atomically in the user's configuration directory. Failed writes preserve the previous selection and display an error in the menu.
- Added smooth, bounded scrolling to All apps with mouse-wheel, touchpad and Page Up/Page Down/Home/End controls. Search stays fixed; new searches reset the scroll position.
- Added a scroll indicator and clipped partially visible application tiles and press feedback to the grid viewport. Picking uses the current scroll position and panel transform.

## Appearance and window integration

- Introduced a pastel cornflower-blue default palette, floating rounded bars, tonal app medallions, larger quick-setting tiles and a broad brightness slider.
- Added more expressive panel and press springs while retaining bounded opacity transitions and reduced-motion behavior.
- Preserved the dark scheme, custom palette seeds and all nine palette variants. `MATERIALAND_THEME=dark` selects the dark startup appearance.
- Honored client decoration requests instead of forcing a second system titlebar. Server decoration painting now leaves client content transparent and removes the light body outline.
- Replaced the assumed monitor dimensions with the active DRM connector's millimeter measurements. Nested displays and missing measurements remain unknown; `--check` reports physical dimensions when available.

Compilation, regression tests and raster previews are checked locally. Interactive GPU/client validation remains dependent on the target session.

## Display controls and clock

- Replaced the brightness dimming overlay with real panel backlight control, initialized from the current hardware level. Updates run outside the render loop, follow external brightness changes and report permission failures without pretending the adjustment succeeded.
- Replaced the orange Night light overlay with a smooth, gentle DRM gamma-ramp white balance. Black stays black; disabling Night light returns to neutral color, and releasing the output restores the previous LUT.
- Display controls show Unavailable in nested mode or when the output lacks the required hardware support.
- Added a clock panel with live seconds, full local date, timezone and a Monday-first monthly calendar. Browse months with the arrows or keyboard, return with Today/Home and dismiss with Escape. The panel follows the shell’s spring animation and reduced-motion setting.
