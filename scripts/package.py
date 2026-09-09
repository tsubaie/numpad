#!/usr/bin/env python3
"""Package a built native executable. Requires Python 3.11+ and nFPM on Linux."""
import argparse
import json
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def version():
    value = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]
    if not re.fullmatch(r"\d+\.\d+\.\d+", value):
        raise ValueError("Release versions must use MAJOR.MINOR.PATCH")
    tag = os.environ.get("GITHUB_REF", "")
    if tag.startswith("refs/tags/") and tag != f"refs/tags/v{value}":
        raise ValueError(f"Tag {tag} does not match Cargo.toml version {value}")
    return value


def copy_docs(destination):
    destination.mkdir(parents=True, exist_ok=True)
    for name in ("README.md", "CONTRIBUTING.md", "LICENSE"):
        shutil.copy2(ROOT / name, destination / name)
    shutil.copytree(ROOT / "docs", destination / "docs", dirs_exist_ok=True)


def archive_tree(source, destination):
    with tarfile.open(destination, "w:gz") as archive:
        archive.add(source, arcname=source.name)


def linux_packages(binary, output, work, ver):
    bundle = work / f"numpad-{ver}"
    bundle.mkdir()
    shutil.copy2(binary, bundle / "numpad")
    (bundle / "numpad").chmod(0o755)
    shutil.copy2(ROOT / "docs/images/icon.png", bundle / "numpad.png")
    shutil.copy2(ROOT / "packaging/numpad.desktop", bundle / "numpad.desktop")
    copy_docs(bundle)
    archive_tree(bundle, output / f"numpad-{ver}-linux-x86_64.tar.gz")
    config = {
        "name": "numpad", "arch": "amd64", "platform": "linux",
        "version": ver, "release": "1", "section": "utils",
        "maintainer": "NumPad contributors <22681130+tsubaie@users.noreply.github.com>",
        "description": "An offline calculation tape with notes and live totals",
        "homepage": "https://github.com/tsubaie/numpad", "license": "MIT",
        "contents": [
            {"src": str(binary), "dst": "/usr/bin/numpad", "file_info": {"mode": 0o755}},
            {"src": str(ROOT / "packaging/numpad.desktop"), "dst": "/usr/share/applications/numpad.desktop"},
            {"src": str(ROOT / "docs/images/icon.png"), "dst": "/usr/share/icons/hicolor/256x256/apps/numpad.png"},
            {"src": str(ROOT / "LICENSE"), "dst": "/usr/share/licenses/numpad/LICENSE"},
            {"src": str(ROOT / "README.md"), "dst": "/usr/share/doc/numpad/README.md"},
        ],
        "overrides": {
            "deb": {"depends": ["libc6 (>= 2.35)", "libgcc-s1", "libxkbcommon0",
                               "libxkbcommon-x11-0", "libwayland-client0", "libx11-6",
                               "libxcb1", "libxcursor1", "libxi6", "libxrandr2", "fontconfig"],
                    "recommends": ["xdg-desktop-portal"]},
            "rpm": {"depends": ["glibc >= 2.35", "libgcc", "libxkbcommon",
                               "libxkbcommon-x11", "libwayland-client", "libX11", "libxcb",
                               "libXcursor", "libXi", "libXrandr", "fontconfig"],
                    "recommends": ["xdg-desktop-portal"]},
            "archlinux": {"depends": ["glibc>=2.35", "gcc-libs", "libxkbcommon",
                                     "libxkbcommon-x11", "wayland", "libx11", "libxcb",
                                     "libxcursor", "libxi", "libxrandr", "fontconfig"]},
        },
        "archlinux": {"packager": "NumPad contributors"},
    }
    config_path = work / "nfpm.json"
    config_path.write_text(json.dumps(config, indent=2))
    names = {"deb": f"numpad_{ver}-1_amd64.deb",
             "rpm": f"numpad-{ver}-1.x86_64.rpm",
             "archlinux": f"numpad-{ver}-1-x86_64.pkg.tar.zst"}
    for kind, name in names.items():
        subprocess.run(["nfpm", "package", "--config", str(config_path),
                        "--packager", kind, "--target", str(output / name)], check=True)


def mac_packages(binary, output, work, ver, arch):
    bundle = work / f"numpad-{ver}"
    contents = bundle / "NumPad.app/Contents"
    (contents / "MacOS").mkdir(parents=True)
    resources = contents / "Resources"
    resources.mkdir()
    shutil.copy2(binary, contents / "MacOS/NumPad")
    (contents / "MacOS/NumPad").chmod(0o755)
    with (ROOT / "packaging/Info.plist").open("rb") as source:
        info = plistlib.load(source)
    info.update(CFBundleVersion=ver, CFBundleShortVersionString=ver,
                CFBundleIconFile="NumPad", LSMinimumSystemVersion="11.0")
    with (contents / "Info.plist").open("wb") as target:
        plistlib.dump(info, target)
    iconset = work / "NumPad.iconset"
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            name = f"icon_{size}x{size}{'@2x' if scale == 2 else ''}.png"
            subprocess.run(["sips", "-z", str(size * scale), str(size * scale),
                            str(ROOT / "docs/images/icon.png"), "--out", str(iconset / name)],
                           check=True, stdout=subprocess.DEVNULL)
    subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(resources / "NumPad.icns")], check=True)
    # Ad-hoc signing is not Developer ID signing or Apple notarization.
    subprocess.run(["codesign", "--force", "--deep", "--sign", "-", str(bundle / "NumPad.app")], check=True)
    subprocess.run(["codesign", "--verify", "--deep", "--strict", str(bundle / "NumPad.app")], check=True)
    copy_docs(bundle)
    archive_tree(bundle, output / f"numpad-{ver}-macos-{arch}.tar.gz")
    image_root = work / "dmg"
    image_root.mkdir()
    shutil.copytree(bundle / "NumPad.app", image_root / "NumPad.app")
    (image_root / "Applications").symlink_to("/Applications")
    subprocess.run(["hdiutil", "create", "-volname", "NumPad", "-srcfolder", str(image_root),
                    "-ov", "-format", "UDZO", str(output / f"numpad-{ver}-macos-{arch}.dmg")], check=True)


def windows_package(binary, output, work, ver):
    bundle = work / "NumPad"
    copy_docs(bundle)
    shutil.copy2(binary, bundle / "NumPad.exe")
    with zipfile.ZipFile(output / f"NumPad-{ver}-windows-x64.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(bundle.rglob("*")):
            if path.is_file():
                archive.write(path, path.relative_to(bundle))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, default=ROOT / "release")
    parser.add_argument("--platform", choices=("linux", "macos", "windows"), required=True)
    parser.add_argument("--arch", choices=("x86_64", "arm64"), default="x86_64")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    ver = version()
    if args.platform != "macos" and args.arch != "x86_64":
        parser.error("Linux and Windows packages currently support x86_64 only")
    if args.platform == "macos" and platform.system() != "Darwin":
        parser.error("macOS packaging requires a native macOS host")
    with tempfile.TemporaryDirectory(prefix="numpad-package-") as temporary:
        work = Path(temporary)
        if args.platform == "linux":
            linux_packages(binary, output, work, ver)
        elif args.platform == "macos":
            mac_packages(binary, output, work, ver, args.arch)
        else:
            windows_package(binary, output, work, ver)
    print(f"Packaged NumPad {ver} for {args.platform}/{args.arch} in {output}")


if __name__ == "__main__":
    main()
