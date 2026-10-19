# Materialand v0.3.0

Materialand v0.3.0 focuses on immersive animations and a configurable Material 3 desktop. This release improves frame pacing, moves circular theme reveals to the GPU and centralizes the shell's design settings in one documented configuration file.

## Immersive animations

- Direct sessions select the highest advertised progressive refresh rate at the display's preferred resolution, keeping the desktop resolution intact. The output initialization log now reports the selected refresh rate.
- Animation updates follow KMS page flips. While a frame is waiting to be displayed, redraw requests are coalesced without advancing animations through a competing timer.
- Nested mode follows the host monitor's reported refresh rate instead of using a fixed 60 Hz limit, and updates it when the window moves between displays. If the host reports no valid rate, it falls back to 60 Hz.
- Improved animation timing after idle and ensured the final settled frame is drawn. Dropped frames do not artificially stretch configured animation durations.

## GPU theme transitions and rendering

- Circular light/dark reveals now clip cached textures with a GLES shader, replacing per-frame CPU masks and uploads of newly masked buffers.
- The reveal circle stays anchored to the screen while panels scale, windows grow and content is clipped. Shader state is isolated from subsequent draws; application surfaces retain their own colors.
- Home-screen and panel caches retain both theme endpoints, avoiding repeated rasterization when alternating between light and dark layers during a transition.
- Fixed scaled overlays sampling a cropped source rectangle during opening animations. Panels now scale their complete native texture.
- A crossfade is used if the reveal shader cannot be compiled.

## Centralized design configuration

- Appearance, Material palette, color-role overrides, layout, spacing, corner radii, shadows, typography, cursor styling, generated wallpaper and animation tuning now live in `~/.config/materialand/config.toml`, or `$XDG_CONFIG_HOME/materialand/config.toml`.
- New configurations include all 315 parameters, with comments explaining their purpose, units and relevant constraints. Partial files remain supported and inherit omitted defaults.
- Layout, rendering, input hit areas and window placement share the same validated settings. Manual design changes apply after a session restart without rebuilding Materialand.
- Quick settings save theme, palette and reduced motion to the same file using atomic replacement, preserving unrelated values and comments.
- Existing appearance preferences migrate automatically when the new configuration file is created. The legacy preference file is retained.
- Invalid configurations are preserved and reported; the session falls back to built-in defaults, and UI saves refuse to overwrite invalid input.
- Added configuration tools: `--init-config`, `--check-config`, `--print-default-config` and optional `--compact-config`. Explicit compaction retains custom values and creates an exact backup; normal initialization and UI saves keep full files intact.

## Touchpad scrolling

- Added `[input.touchpad] invert_scroll` to reverse horizontal and vertical finger scrolling in applications and the launcher.
- Preserved high-resolution scroll values, direction metadata and finger-stop events. Mouse-wheel direction and three-finger launcher gestures remain independent.

## Validation

- 162 automated tests pass, covering configuration, migration, input, layout, animation timing and rendering contracts.
- An additional offscreen EGL pixel test passes, checking shader compilation, circular clipping, alpha blending, scaling, source cropping, framebuffer orientation and shader isolation.
- Formatting, Clippy and the release build pass. Documentation now covers design configuration and high-refresh validation.

Animations target the active display's refresh rate; actual presentation performance depends on hardware and, in nested mode, the host compositor. Physical display FPS was not established by the offscreen tests.
