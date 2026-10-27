#!/bin/sh
# Run after cargo build --release. DESTDIR supports packaging/staging without root.
set -eu

project_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
prefix=${PREFIX:-/usr/local}
session_dir=${SESSIONDIR:-/usr/share/wayland-sessions}
stage=${DESTDIR:-}

case "$prefix" in /*) ;; *) echo 'PREFIX must be absolute' >&2; exit 1 ;; esac
case "$session_dir" in /*) ;; *) echo 'SESSIONDIR must be absolute' >&2; exit 1 ;; esac
case "$stage" in ''|/*) ;; *) echo 'DESTDIR must be absolute' >&2; exit 1 ;; esac

# Check the complete payload before modifying the installation.
test -x "$project_dir/target/release/materialand"
for file in Roboto-Regular.ttf Roboto-Medium.ttf Roboto-Bold.ttf MaterialIcons-Regular.ttf codepoints README.md; do
    test -r "$project_dir/assets/fonts/$file"
done
test -r "$project_dir/packaging/materialand.desktop"

install -Dm755 "$project_dir/target/release/materialand" "$stage$prefix/bin/materialand"
for file in Roboto-Regular.ttf Roboto-Medium.ttf Roboto-Bold.ttf MaterialIcons-Regular.ttf codepoints README.md; do
    install -Dm644 "$project_dir/assets/fonts/$file" "$stage$prefix/share/materialand/fonts/$file"
done
install -Dm644 "$project_dir/packaging/materialand.desktop" "$stage$session_dir/materialand.desktop"
