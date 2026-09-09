#!/bin/sh
# Download a pinned packaging tool and verify its upstream checksum.
set -eu
version=2.47.0
destination="$(pwd)/.tools/nfpm"
mkdir -p "$destination"
if [ -x "$destination/nfpm" ]; then exit 0; fi
temporary="$(mktemp -d)"
trap 'rm -rf -- "$temporary"' EXIT
trap 'exit 1' HUP INT TERM
asset="nfpm_VERSION_Linux_x86_64.tar.gz"
asset="$(printf '%s' "$asset" | sed "s/VERSION/$version/")"
base="https://github.com/goreleaser/nfpm/releases/download/v$version"
curl --proto '=https' --tlsv1.2 -fsSL "$base/$asset" -o "$temporary/$asset"
curl --proto '=https' --tlsv1.2 -fsSL "$base/checksums.txt" -o "$temporary/checksums.txt"
expected="$(awk -v name="$asset" '$2 == name {print $1}' "$temporary/checksums.txt")"
actual="$(sha256sum "$temporary/$asset" | cut -d ' ' -f1)"
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
    echo "nFPM checksum mismatch" >&2
    exit 1
fi
tar -xzf "$temporary/$asset" -C "$destination" nfpm
