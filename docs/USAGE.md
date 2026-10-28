# Using Materialand

[Back to the overview](../README.md) · [Configuration](CONFIGURATION.md)

## Keyboard and gestures

| Input | Action |
| --- | --- |
| Tap `Super` (Win), or `Super+H` | Show home and minimize open windows |
| Home search or `Super+P` | Open the launcher; type to search, Enter to launch, Escape to close |
| Three-finger swipe up | Open the launcher; close the shade when it is open |
| Three-finger swipe down | Open the shade; close the launcher when it is open |
| Four-finger swipe up | Open recent apps |
| Four-finger swipe down | Close recent apps |
| `Super+Tab` / `Super+S` | Toggle recent apps / system shade |
| `Super+BackSpace` | Leave split screen and restore both windows |
| `Super+Enter` | Open a terminal |
| `Super+D` | Switch light/dark theme |
| `Super+Q` | Close the focused window |
| Clock in the status strip or shade | Open date, time and calendar |
| Bell / Wi-Fi / Bluetooth in the status strip | Open notifications / networks / paired devices |

## Layout and navigation

The default tablet layout uses a compact Android-style status strip at the top (time, notifications, radios and battery), with no bottom navigation buttons or reserved navigation strip. A bare Super press returns home on release; using Super in a shortcut does not trigger Home. App windows reserve the status-strip height; ordinary fullscreen applications hide it. Escape dismisses the current overlay.

## Gesture ownership

Gestures belong to the panel visible when they start. Three fingers down closes an open launcher without opening the shade; three fingers up closes the shade without opening the launcher or recents. Four fingers down closes recents. Other global swipes inside a panel are ignored. A closing animation cannot turn the same action into another overlay. Global swipe directions do not change with scroll inversion.

Three-finger shade/drawer gestures and four-finger recents gestures track movement
before release. Hold still to hold the panel, or reverse direction to bring it back.
Release settles it using distance and recent velocity; a pause clears the fling.
Cancelling the gesture restores its starting state. With Reduce motion enabled,
the action happens at the end of the gesture without intermediate motion.

The shade reveals downward from its top edge: header, brightness, tile rows, then
notification content. These phases follow expansion progress, including reversal;
text and icons keep their native size.

## Touchpad input

Touchpad tapping, tap-and-drag and drag lock are enabled by default in direct sessions. One-finger tap clicks; two-finger tap opens a context menu. Tap, then touch again and move to drag icons, windows, recent-app cards, split dividers or sliders without pressing the pad. Drag lock lets you reposition the finger during a drag; tap again to finish. Two-finger scrolling is explicitly selected when supported, and accidental input while typing is suppressed.

Launcher and shade lists follow two-finger motion directly, then coast on release. Touch the pad again or click to stop; reduced motion disables coasting. A scroll remains attached to its original list, including the two shade columns. Home pages turn once per horizontal two-finger gesture. Native applications retain their own scrolling and inertia. These controls act at the pointer; the touchpad is not mapped to absolute screen coordinates. Nested sessions use the host's tapping/device settings and depend on which gesture events it forwards.

## Recent apps and split screen

On wide, sufficiently tall outputs, recent apps use a two-row tablet grid; compact outputs use a carousel. Scroll or use arrow keys, click a preview to restore it, and use its close button or drag it upward to close it. The card continues upward after release; once it leaves the screen, the app receives a normal close request and neighboring cards slide into the gap. A short drag returns smoothly. Applications may still ask to save work; a card returns if its app ignores or cancels closing. Click the split icon under one preview, then click another preview or its split icon to complete the pair. Click the first icon again to cancel. If the apps need more room, the first selection stays active so you can choose another app. Drag the divider or either adjacent inner window edge to resize both panes; right-click the divider or maximize either app to leave split screen.

## Home shortcuts

Right-click an app to add/remove its home shortcut. Drag an icon to any free cell, or drag it from the launcher onto home. Empty cells stay empty; dropping onto another shortcut swaps the two. Hold an icon near the right edge to create another page. Positions persist in `workspace.json`; existing home shortcuts migrate from `home-apps`, which is kept intact. On taller displays, the date above the shortcut grid opens the calendar. The grid is centered and limited in width; saved empty cells and shortcut positions are preserved.

## Quick settings and notifications

Wi-Fi and Bluetooth tiles show actual service state. Click a tile to toggle the radio; its chevron opens available Wi-Fi networks or paired Bluetooth devices. The shade also shows battery percentage and charging state. On wide outputs, quick settings and notifications sit side by side with independent scrolling. On compact outputs, the status-strip bell opens the notification page. Notifications support individual dismissal and Clear all. See [System controls](SYSTEM_CONTROLS.md) for dependencies and connection setup.

## App icons

App icons come from the `Icon` field of installed `.desktop` entries, including absolute paths and installed icon themes. Home, launcher, dock, server-side titlebars and recent apps share these images; missing or unreadable icons retain their Material fallback. PNG and SVG decode directly at startup and are cached, so scrolling and animations do not reload image files. Legacy image formats use GdkPixbuf.
