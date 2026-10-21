#!/usr/bin/env bash
# Builds the PCM package the spike was installed from (Linux): the plugin.json,
# the spike binary as an exec action, a probe that records what KiCad passes to
# an exec plugin, and an icon.
set -euo pipefail
spike_dir=$(dirname "$(realpath "$0")")
out=${1:-$spike_dir/target/traxide-spike-pcm.zip}
staging=$(mktemp -d)
trap 'rm -rf "$staging"' EXIT

cargo build --quiet --release --manifest-path "$spike_dir/Cargo.toml"
cp -r "$spike_dir/package/." "$staging/"
cp "$spike_dir/target/release/traxide-kicad-ipc-spike" "$staging/plugins/"
mkdir -p "$staging/resources"
convert -size 24x24 xc:'#2a7' "$staging/plugins/icon.png"
cp "$staging/plugins/icon.png" "$staging/resources/icon.png"
rm -f "$out"
(cd "$staging" && zip -qrX "$out" metadata.json plugins resources)
echo "$out"
