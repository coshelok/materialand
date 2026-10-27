Font assets installed by `packaging/install.sh`. The runtime searches the
configured `[fonts] directory` first (`MATERIALAND_FONT_DIR` supplies its startup
default), then Materialand's installed resource directories, development assets
and system font directories for:

* a UI font: Roboto, Noto Sans, DejaVu Sans or Liberation Sans;
* `MaterialIcons-Regular.ttf` plus the matching `codepoints` mapping file in
  the same directory — only then are the canonical Material icon names used,
  otherwise the built-in Unicode icon fallback applies.

Installed resources live in `share/materialand/fonts` under the binary's prefix.
The runtime also searches `$XDG_DATA_HOME/materialand/fonts` (default
`~/.local/share/materialand/fonts`) and each `$XDG_DATA_DIRS/materialand/fonts`
(default `/usr/local/share` and `/usr/share`). Installed resources take precedence
over the build checkout, so the checkout can be removed after installation.

The files currently here are Roboto in three weights, Material Icons and the
`codepoints` map. Both families are licensed under Apache-2.0. The compositor
can still run without them by falling back to system fonts and Unicode icons.
