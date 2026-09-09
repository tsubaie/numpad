#!/usr/bin/env python3
"""Write SHA256SUMS for release assets, excluding local backups/documents."""
import argparse
import hashlib
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("directory", type=Path)
args = parser.parse_args()
patterns = ("*.deb", "*.rpm", "*.pkg.tar.zst", "*.tar.gz", "*.dmg", "*.zip")
files = sorted({path for pattern in patterns for path in args.directory.glob(pattern)})
if not files:
    parser.error("No release assets found")
lines = []
for path in files:
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    lines.append(f"{digest}  {path.name}\n")
(args.directory / "SHA256SUMS").write_text("".join(lines), encoding="ascii")
print(f"Wrote checksums for {len(files)} assets")
