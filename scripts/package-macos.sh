#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --locked
mkdir -p release/NumPad.app/Contents/MacOS
cp target/release/numpad release/NumPad.app/Contents/MacOS/NumPad
cp packaging/Info.plist release/NumPad.app/Contents/Info.plist
printf '%s\n' 'Created release/NumPad.app (unsigned).'
