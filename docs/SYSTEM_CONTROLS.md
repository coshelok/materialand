# System controls

Open the shade with four fingers down, `Super+S`, the Settings navigation control, or the top status strip. Swipe four fingers up to dismiss it; this does not open recent apps. The header opens the clock/calendar and session menu. The body contains radio toggles, display/appearance controls and palettes; it scrolls on small outputs.

## Battery

Charge and charging status are read from `/sys/class/power_supply`. Systems without a readable battery show **Battery unavailable**. Multiple readable batteries use their average percentage. Monitoring runs on a worker thread and refreshes periodically.

## Wi-Fi

Materialand uses NetworkManager through `nmcli`. NetworkManager must manage a Wi-Fi interface and permit the current user to control it. The tile shows the actual radio and connection state; clicking its main area toggles Wi-Fi. Its chevron opens the access-point list and selecting an entry requests a connection. Previously saved connections and open networks can connect directly.

For new secured networks, open connection settings from the list heading. Install `nm-connection-editor`, or use the `nmtui` fallback in the configured terminal. Materialand does not store Wi-Fi passwords. Connection errors remain visible; rejected commands do not leave a falsely enabled switch.

Network lists reuse NetworkManager's scan results without forcing a new scan every polling cycle. See the official [nmcli manual](https://networkmanager.pages.freedesktop.org/NetworkManager/NetworkManager/nmcli.html).

## Bluetooth

Materialand queries BlueZ over the system bus through `busctl`. It discovers adapter paths dynamically and reads their actual Powered state. The tile toggles adapter power; the chevron lists paired devices, which can be connected or disconnected by clicking them.

To discover and pair new devices, install `blueman-manager` and open pairing settings from the list heading. Materialand currently delegates new pairing to that application. The integration uses BlueZ's [Adapter API](https://github.com/bluez/bluez/blob/master/doc/org.bluez.Adapter.rst) and Device API.

## Service behavior

All service queries and commands run outside the compositor thread, with timeouts. Controls show **Working…** during an outstanding action and then reflect a new service reading. Missing services/hardware show **Unavailable**. Service permissions and hardware switches remain authoritative; Materialand does not invoke sudo or modify service policies.

Wi-Fi/Bluetooth controls affect the user's actual machine, including when Materialand runs nested. Hardware brightness and Night light retain their direct-session-only behavior.

## Status strip and notifications

In tablet mode the top status strip shows time, battery and actual Wi-Fi/Bluetooth state. Tap the clock for date/time, a radio icon for its device list, the battery/empty strip for quick settings, or the bell for notifications. An indicator beside the bell appears only when messages exist. The strip reserves space above normal/maximized/split windows and is hidden by ordinary fullscreen content; explicitly opened shell overlays can show it again.

Materialand hosts `org.freedesktop.Notifications` on its private session bus. Applications launched in this session can send plain-text messages; the list supports replacement, explicit expiry, individual dismissal and Clear all. Default messages remain until dismissed. Dismissal and expiry emit the standard close signal. The service follows the [desktop notification protocol](https://specifications.freedesktop.org/notification/latest/protocol.html), advertising text and persistence; actions, images and sounds are not advertised. Messages are kept in memory for this session, with a maximum of 64 current entries.

If the private bus or notification service cannot start, the list reports that it is unavailable. The host desktop's notification service is not replaced in nested mode.
