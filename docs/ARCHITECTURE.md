# Ownership and rendering contracts

```mermaid
flowchart LR
    App[Application runtime] --> Nested[Winit backend]
    App --> DRM[DRM backend]
    Nested --> Desktop[Desktop state]
    DRM --> Desktop
    Input[Input target selection] --> Desktop
    Protocol[Wayland adapters] --> Desktop
    Desktop --> Windows[Window manager]
    Desktop --> Output[Output model]
    Desktop --> Shell[Shell components]
    Windows --> Scene[Scene groups]
    Shell --> Scene
    Scene --> Nested
    Scene --> DRM
```

## Windows and coordinates

`WindowManager` owns the mapped Smithay space, registered window metadata/properties, focus and resize transactions. Creating an XDG toplevel registers a window; the initial empty commit triggers configuration. Its first committed buffer maps and focuses it. Committing a null buffer unmaps it. A minimized window remains registered while absent from the mapped space.

`SurfaceSpace` is the coordinate authority. A window location refers to its XDG geometry, while the surface origin is that location minus the geometry offset inside the client buffer. Rendering and hit testing use the same origin. Surface-tree hit offsets are added to the global origin before delivery to Smithay. Popup surfaces are hit-tested without first clipping to their parent's geometry.

Input selects one target in scene order: shell overlays/chrome, a client surface, a decoration, or the desktop. Scroll shares this selection. Client grabs retain additional button events. Consumed button/key presses are paired with consumed releases; physical keycodes identify swallowed keys even when modifiers/layouts change. Seat suspension cancels grabs and releases tracked input state.

Resize requests configure a size while retaining the committed window placement. A `ResizePlacement` moves an anchored edge only after the client acknowledges and commits the requested state. Position calculations use the client's actual geometry and respect its size limits.

## Scene and outputs

`Layers` accepts groups in back-to-front order. Each group is already front-to-back. Final assembly reverses groups, never the surface elements inside an individual window. Decorations sit behind the complete client surface tree within the same window group. Popup/subsurface order therefore survives composition of overlapping windows.

Window entrances uniformly transform the entire surface/decorations group on the GPU while clients retain their native protocol size. The origin is the launch icon when a matching launch is pending, otherwise the window center. Entrance time starts after its first frame is presented. A focused launching window is composited above the outgoing launcher; panels and menus stay in front. Clicks on the moving frame finish the entrance and are consumed so controls at invisible final coordinates cannot activate. Once settled, surface coordinates and normal input delivery apply directly. Client cursor surfaces retain their own buffer scale and hotspot; named cursors have a raster fallback.

The output model describes an upright, scale-1 logical desktop. Winit's framebuffer orientation is handled by its own damage tracker, independently of protocol output metadata. DRM selects the highest progressive millihertz refresh at the preferred resolution. Nested mode tracks its host monitor's reported rate. `set_output_mode` updates output metadata, desktop size, pointer bounds and maximized/fullscreen placement together.

Radial theme reveals clip cached shell textures with a GLES fragment shader. Draw-time source/destination mapping keeps the circle in screen coordinates during clipping, panel scaling and window entrances. A failed shader compilation falls back to a crossfade. The shader override is scoped to one draw; client surfaces retain their own colors. Home and sheet caches retain both palette endpoints, so switching between them does not rasterize or upload the same pixels every frame. Scaled overlays sample their full native source texture instead of treating the destination size as a source crop.

DMA-BUF import requests are completed only after the active renderer validates the buffer. A changed GPU format set replaces the advertised global while preserving disabled old globals for existing clients.

## Direct-session lifetime

The DRM backend owns the seat, libinput/udev sources, pending GPU initialization and the active output. Input source registration succeeds as a unit. Pause removes timers/device sources and drops scanout; resume rebuilds renderer/output state from the seat. Connector-change events keep an unchanged output alive and recreate it when the selected connector/mode changes.

`FrameState` permits one submitted frame. Rendering synchronizes its fence, queues once, and waits for vblank before acknowledging scanout and releasing client frame callbacks. While a flip is pending, redraw requests only mark the scene dirty: they neither arm an animation timer nor advance springs. The page-flip callback advances and renders the next pose immediately. A no-damage frame may release callbacks for content already presented and uses a refresh-derived timer when more animation work remains. Idle wakeups maintain the clock. Both backends paint the final settled pose, and the animation clock limits its first step after idle to one display interval.

The listening socket, output and GPU are separate resources. Dropping/reopening a GPU preserves Wayland clients and desktop state. A private session bus has the compositor process lifetime and isolates normal D-Bus activation from a parent desktop.

`config/default.toml` is the source of built-in design values. Before fonts, devices and windows are initialized, a partial user `config.toml` is merged and validated into one immutable typed design snapshot. Layout, rendering, input geometry, window placement, typography, Material roles and motion constructors consume that snapshot. Runtime caches therefore require no filesystem reads or configuration locks.

Light/dark mode, palette and reduced motion save to the same TOML document. Preference writes reread the user file, validate it, change only those keys and preserve other values and comments. The legacy `appearance` file migrates when the new file is absent. Appearance, dock pins and home shortcuts share atomic file replacement under the XDG configuration directory. See [Design configuration](DESIGN.md) for the schema and update contract.

## Boundaries retained from the original implementation

Smithay continues to own protocol validation, buffer/surface-tree handling, keyboard state and low-level device APIs. Material color computation, raster drawing and widget appearance were split into dedicated components rather than replaced with another graphics toolkit. Existing visual and utility tests were retained alongside new contract tests.
