# Configuration

Set environment variables before launching Materialand. Appearance and keyboard settings are read at startup; theme, palette and reduced motion can also be changed in quick settings.

## Appearance

| Variable | Value |
| --- | --- |
| `MATERIALAND_THEME` | Initial mode: `light` (default) or `dark`, used when no saved choice exists |
| `MATERIALAND_SEED` | Palette seed, for example `'#6b85b3'`; takes priority over image extraction |
| `MATERIALAND_WALLPAPER` | Image path used for palette extraction |
| `MATERIALAND_VARIANT` | Initial palette: `tonalspot`, `vibrant`, `expressive`, `content`, `fidelity`, `rainbow`, `fruitsalad`, `neutral` or `monochrome` |
| `MATERIALAND_FONT_DIR` | First font search directory; bundled and system fonts are fallbacks |

```sh
MATERIALAND_THEME=dark MATERIALAND_SEED='#6b85b3' ./target/release/materialand
```

Changing light/dark mode or the palette saves both choices immediately to `$XDG_CONFIG_HOME/materialand/appearance` (normally `~/.config/materialand/appearance`). The next session applies them before its first frame, including after an unclean exit. Saved choices take priority over the initial mode and variant above; removing this file resets them to startup defaults. The seed still comes from the environment or image extraction.

## Keyboard

| Variable | Example |
| --- | --- |
| `MATERIALAND_XKB_LAYOUT` | `us,ru` |
| `MATERIALAND_XKB_OPTIONS` | `grp:alt_shift_toggle` |
| `MATERIALAND_XKB_VARIANT` | XKB variant names corresponding to the chosen layouts |
| `MATERIALAND_XKB_MODEL` | `pc105` |
| `MATERIALAND_XKB_RULES` | `evdev` |

Settings fall back to the corresponding `XKB_DEFAULT_*` variables, then the system keyboard configuration. Multiple layouts use Alt+Shift unless another `grp:` option is set. Super shortcuts use Latin key positions across layouts.

## Session and display

| Variable | Value |
| --- | --- |
| `MATERIALAND_NESTED` | `1` defaults to nested mode; `--nested` is also available |
| `MATERIALAND_DRM_DEVICE` | Preferred DRM device, for example `/dev/dri/card1` |

Use `materialand --check` to inspect connected outputs, modes and reported physical dimensions. Direct mode needs an active local seat through logind or seatd.

Brightness controls the backlight associated with the active connector, with a 2% minimum. It uses direct access when permitted, otherwise `busctl` and logind. Night light adjusts the output's color channels through its gamma ramps. Both controls show **Unavailable** in nested mode or when the required output support is missing. Three-finger launcher gestures use libinput in direct sessions.

## Saved applications

Files live in `$XDG_CONFIG_HOME/materialand/`, or `~/.config/materialand/` when `XDG_CONFIG_HOME` is unset.

| File | Contents |
| --- | --- |
| `home-apps` | Home screen shortcut identities, in display order |
| `pinned-apps` | Dock pin identities, in display order |

Manage these through an application's context menu. Home shortcuts and dock pins are independent; empty files preserve an intentionally empty selection. Missing application identities remain saved for reinstalls. Unpinning the terminal leaves `Super+Enter` available.
