#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
profile=debug
case "${1:-}" in
  --release) profile=release ;;
  "") ;;
  *) echo 'Usage: build-sidecar.sh [--release]' >&2; exit 2 ;;
esac
host=$(rustc -vV | while IFS=' ' read -r key value; do
  if [ "$key" = host: ]; then printf '%s' "$value"; fi
done)
target=${TAURI_ENV_TARGET_TRIPLE:-$host}
suffix=
case "$target" in *windows*) suffix=.exe ;; esac
# Host builds skip --target so they reuse target/<profile> instead of a second full dep build.
if [ "$target" = "$host" ]; then
  cargo build -p unelos-cli ${1:+"$1"}
  out="${CARGO_TARGET_DIR:-target}/$profile"
else
  cargo build -p unelos-cli ${1:+"$1"} --target "$target"
  out="${CARGO_TARGET_DIR:-target}/$target/$profile"
fi
mkdir -p src-tauri/binaries
cp "$out/unelos$suffix" "src-tauri/binaries/unelos-$target$suffix"
