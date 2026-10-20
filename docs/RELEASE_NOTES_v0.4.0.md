# Materialand v0.4.0

Materialand introduces a redesigned tablet-style desktop, combining Android 15-inspired navigation with Material 3 Expressive colors, shapes and spring animations.

## Desktop and navigation

- Redesigned home screen with a two-part clock widget, centered date, page indicators and an app-search capsule.
- Full-screen app drawer with an adaptive grid, search and scrolling.
- Drag applications from the drawer onto home, reorder shortcuts and move them between pages. Shortcut positions persist across sessions.
- Floating pastel navigation capsule and transparent status strip showing time, notifications, battery and connection status.
- Optional desktop mode restores the traditional topbar and application dock.

## Recent apps and split screen

- Live app overview with large GPU-rounded previews, application titles and smaller neighboring cards.
- Restore, close or select applications for split screen directly from their previews.
- Softer spring transitions between applications and the overview.
- Swipe cards upward to close them; dismissal continues offscreen while neighboring cards slide into place. Applications that cancel or ignore closing return to the overview.
- Split screen supports resizing both windows with a shared divider.
- Fixed second-app selection, divider hit detection and resizing that displaced individual windows.

## System shade and notifications

- Full-screen system shade with adaptive quick-control columns, clock, date and hardware brightness control.
- Live battery percentage and charging status.
- Wi-Fi controls and network selection through NetworkManager.
- Bluetooth power controls and paired-device connections through BlueZ.
- Session-local desktop notifications with individual dismissal and Clear all.
- System queries run outside the rendering thread to keep the interface responsive.

## Input, rendering and configuration

- Three fingers up/down open and close the app drawer; four fingers up open recent apps; four fingers down open the system shade.
- Reverse gestures dismiss the current panel without opening another overlay.
- Touchpad tap-to-click and tap-and-drag are enabled by default.
- Rounded live previews support transformed and scaled client buffers; cached decorations reduce repeated rendering during animations.
- Window placement respects the status strip; fullscreen applications use the entire display.
- Added commented settings for navigation, overview motion, split screen, shade layout, widgets, grids and touchpad behavior.
- Updated documentation and configuration validation.

## Validation and current limits

Verified with 192 automated tests and two offscreen EGL rendering tests. These checks do not measure physical display performance or replace testing with real clients in a direct session.

Materialand remains experimental: native Wayland applications and a single output at scale 1 are supported. XWayland and multi-monitor layouts are not yet implemented. Hardware brightness, Night light and touchpad gestures require a direct session and compatible hardware. New secured Wi-Fi connections and Bluetooth pairing use external settings tools.
