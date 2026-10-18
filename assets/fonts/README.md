Optional font assets. The runtime searches `MATERIALAND_FONT_DIR` first, then
this directory (and then the system font directories) for:

* a UI font: Roboto, Noto Sans, DejaVu Sans or Liberation Sans;
* `MaterialIcons-Regular.ttf` plus the matching `codepoints` mapping file in
  the same directory — only then are the canonical Material icon names used,
  otherwise the built-in Unicode icon fallback applies.

The files currently here (Roboto in three weights, Material Icons and the
`codepoints` map) are the optional copies the compositor picks up
automatically. Both families are licensed under Apache-2.0. The compositor
still runs without them by falling back to system fonts and Unicode icons.
