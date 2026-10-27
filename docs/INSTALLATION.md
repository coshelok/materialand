# Build and install

[Back to the overview](../README.md)

## Build and run

Install Rust, `pkg-config` and development libraries for DRM, EGL, GBM, GLES, libinput, libseat, udev, Wayland, xkbcommon, GdkPixbuf, librsvg (2.52 or newer) and Cairo. The [CI workflow](../.github/workflows/ci.yml) lists the packages used on Ubuntu.

```sh
cargo build --release --locked
./target/release/materialand --nested  # Run inside your current desktop
```

To start a desktop session, run as your normal user from an active local TTY with logind or seatd:

```sh
./target/release/materialand
```

Use `./target/release/materialand --check` to inspect DRM connectors before launching.

## SDDM installation

Run from the repository root after building:

```sh
sudo sh packaging/install.sh
```

This installs the binary into `/usr/local/bin`, its fonts and icon mapping into
`/usr/local/share/materialand/fonts`, and the session entry into
`/usr/share/wayland-sessions`. Select **Materialand** in SDDM. After rebuilding,
run the installer again. The installed session does not require the source checkout.
Packagers can set `PREFIX`, `SESSIONDIR` and `DESTDIR` (all absolute paths);
`DESTDIR` stages files without changing their runtime paths. A custom binary prefix
must be on the display manager's PATH.
