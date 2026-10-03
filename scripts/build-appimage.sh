#!/bin/bash
# Generates the Linux AppImage on demand (bundling is disabled by default in tauri.conf.json).
# Passing --bundles forces bundling even though bundle.active is false.
# "Text file busy" (ETXTBSY) can happen when a file the bundler just wrote/patched is
# executed or replaced while still in use, so retry once before giving up.
set -uo pipefail

cd "$(dirname "$0")/.."
export PATH="$HOME/.cargo/bin:$PATH"

if npm run tauri build -- --bundles appimage; then
    exit 0
fi

echo "AppImage bundling failed, retrying once..." >&2
sleep 2
npm run tauri build -- --bundles appimage
