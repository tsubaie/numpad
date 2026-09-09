"""Packaging contract tests; native package tools are mocked here, real installs run in CI."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("packager", ROOT / "scripts/package.py")
PACKAGER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PACKAGER)


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="numpad-package-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.binary = self.root / "numpad"
        self.binary.write_bytes(b"test executable")
        self.output = self.root / "dist"
        self.output.mkdir()
        self.work = self.root / "work"
        self.work.mkdir()

    def test_tag_must_match_cargo_version(self):
        with patch.dict(os.environ, {"GITHUB_REF": "refs/heads/main"}):
            version = PACKAGER.version()
        with patch.dict(os.environ, {"GITHUB_REF": f"refs/tags/v{version}"}):
            self.assertEqual(PACKAGER.version(), version)
        with patch.dict(os.environ, {"GITHUB_REF": "refs/tags/v999.0.0"}):
            with self.assertRaises(ValueError):
                PACKAGER.version()

    def test_linux_asset_names_and_package_dependencies(self):
        commands = []
        configs = []

        def capture(command, **_kwargs):
            commands.append(command)
            configs.append(json.loads(Path(command[command.index("--config") + 1]).read_text()))

        with patch.object(PACKAGER.subprocess, "run", side_effect=capture):
            PACKAGER.linux_packages(self.binary, self.output, self.work, "1.3.0")
        self.assertEqual(len(commands), 3)
        names = {Path(command[-1]).name for command in commands}
        self.assertEqual(names, {"numpad_1.3.0-1_amd64.deb",
                                "numpad-1.3.0-1.x86_64.rpm",
                                "numpad-1.3.0-1-x86_64.pkg.tar.zst"})
        rpm_dependencies = configs[0]["overrides"]["rpm"]["depends"]
        self.assertIn("libwayland-client", rpm_dependencies)
        self.assertNotIn("wayland-libs", rpm_dependencies)
        with tarfile.open(self.output / "numpad-1.3.0-linux-x86_64.tar.gz") as archive:
            self.assertTrue(archive.getmember("numpad-1.3.0/numpad").mode & 0o111)
            self.assertIn("numpad-1.3.0/numpad.desktop", archive.getnames())
            self.assertIn("numpad-1.3.0/LICENSE", archive.getnames())

    def test_windows_zip_contains_docs_and_screenshots(self):
        PACKAGER.windows_package(self.binary, self.output, self.work, "1.3.0")
        with zipfile.ZipFile(self.output / "NumPad-1.3.0-windows-x64.zip") as archive:
            for name in ("NumPad.exe", "README.md", "CONTRIBUTING.md", "LICENSE",
                         "docs/images/numpad-dark.jpg", "docs/images/numpad-light.jpg"):
                self.assertIn(name, archive.namelist())
            self.assertEqual(archive.read("NumPad.exe"), b"test executable")

    def test_checksums_exclude_backups_and_private_documents(self):
        (self.output / "numpad_1.3.0-1_amd64.deb").write_bytes(b"package")
        (self.output / "old-backup.exe").write_bytes(b"backup")
        (self.output / "session.numpad").write_bytes(b"private")
        subprocess.run([sys.executable, str(ROOT / "scripts/checksums.py"), str(self.output)], check=True)
        manifest = (self.output / "SHA256SUMS").read_text()
        self.assertIn("numpad_1.3.0-1_amd64.deb", manifest)
        self.assertNotIn("backup", manifest)
        self.assertNotIn("session", manifest)


if __name__ == "__main__":
    unittest.main()
