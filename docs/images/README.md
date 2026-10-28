# README preview assets

These images come from Materialand's own shell painter. They use fixture apps and
service data, so they must be described as synthetic previews rather than live
Wayland screenshots or evidence of hardware compatibility.

| Asset | Source under `target/visual-audit/` |
| --- | --- |
| `home-light.png` | `materialand-tablet-light-1280-home.png` |
| `shade-dark.png` | `materialand-tablet-dark-1280-shade.png` |
| `shade-motion.gif` | `expressive-shade.mp4`, converted to a looping GIF |

Regenerate the sources from the repository root:

```sh
cargo test --release --locked --offline write_tablet_design_previews -- --ignored --nocapture
cargo test --release --locked --offline write_expressive_motion_previews -- --ignored --nocapture
```

Copy the two PNG files to this directory. Generate the GIF with ffmpeg:

```sh
ffmpeg -v error -i target/visual-audit/expressive-shade.mp4 \
  -filter_complex '[0:v]split[a][b];[a]palettegen=max_colors=128[p];[b][p]paletteuse=dither=bayer:bayer_scale=3' \
  -loop 0 -y docs/images/shade-motion.gif
```

The GIF demonstrates top-down expansion, staged content opacity, bounded rebound,
and reversal during closing. The motion generator also writes
`expressive-shade-gesture.mp4`: a scripted drag, hold and reverse using the shell's
progress API. Actual input events are covered separately by gesture regressions.
It is not a frame-pacing benchmark. Keep the static previews available so the
README remains useful without playing the animation.
