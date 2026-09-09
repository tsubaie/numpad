#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
arch="$(uname -m)"
case "$arch" in arm64|x86_64) ;; *) echo "Unsupported macOS architecture: $arch" >&2; exit 1 ;; esac
python3 scripts/package.py --platform macos --arch "$arch" --binary target/release/numpad --output release
