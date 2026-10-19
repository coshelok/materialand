# Materialand v0.2.5

## Tablet-style home screen

- Sessions now start on the home screen, with a large clock, local date and personal app shortcuts. The terminal remains available through Super+Enter and its dock shortcut.
- Add and remove home shortcuts through application context menus in the launcher, dock and home screen. Home shortcuts are persisted independently of dock pins; missing application identities are retained for reinstalls.
- App grids spread across pages with spring-driven horizontal motion, shared rendering/input geometry and reduced-motion support.
- The top bar’s leading Home button and Super+H minimize open windows and reveal the home screen. Home works while the launcher is open and ends active compositor move/resize operations.
- Launching an application from home or the launcher restores a known running window before starting another process.
- Context menus provide both dock and home actions, with Up/Down selection and Enter activation.

## Touchpad launcher gesture

- Three-finger upward touchpad swipes open the launcher; downward swipes close it. This works in direct DRM sessions, including over fullscreen applications.
- Gesture recognition checks direction, travel distance, finger count, cancellation and device identity. Incomplete, cancelled, horizontal and four-finger swipes do not trigger it. Input pause/device removal clears pending gesture state.

## Window placement

- Normal and maximized windows reserve space for the top bar, including compositor-drawn titlebars.
- Placement is reconciled after commits and decoration changes, output resize, move/resize and restore operations, preventing inaccessible titlebars behind shell chrome.
- Mode requests before the first buffer no longer map an unfinished window or bypass its initial focus.
- Initial client configuration advertises usable bounds. Maximized content size is corrected when server-side decorations become active.
- True fullscreen uses the entire output. Shell bars follow the topmost visible window, so another window above a fullscreen application can use normal desktop controls.

## App launch animation and visual cleanup

- New windows grow smoothly from the icon selected in the home screen, dock or launcher. Restoring a minimized app uses the same transition; launches without a known icon use a centered entrance.
- Entrances wait for the first presented frame, preserving the animation during slow client startup or an idle compositor. Client content, decorations, subsurfaces and popups share one GPU transform with their native protocol sizes preserved.
- The launching window appears above the outgoing launcher so the transition remains visible. Reduced motion skips the animation.
- Removed an extra frame of waiting at the start of window entrances, while keeping idle-time protection for slow starts.
- Removed the All apps button and empty-state text from the home screen.
- Removed the calendar’s Today button and icon. The Home key returns to the current month.

## Remembered appearance

- Light/dark mode and the selected Material palette are saved together as soon as either changes and restored before the next session's first frame.
- Appearance preferences use atomic file replacement; missing or invalid settings fall back to startup defaults without preventing the session from starting.
