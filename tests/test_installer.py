"""Offline installer tests. Downloads and package managers are mocked."""
import hashlib
import io
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
SH = shutil.which("sh")


@unittest.skipUnless(SH, "POSIX sh is required")
class InstallerTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="numpad-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.home = self.root / "user home"
        self.home.mkdir()
        self.fixtures = self.root / "fixtures"
        self.fixtures.mkdir()
        self.log = self.root / "commands"
        self.env = dict(os.environ, HOME=self.home.as_posix(),
                        TMPDIR=self.root.as_posix(),
                        PATH=str(self.bin) + os.pathsep + os.environ["PATH"],
                        NUMPAD_VERSION="1.3.0", NUMPAD_INSTALL="tarball",
                        TEST_OS="Linux", TEST_ARCH="x86_64", TEST_UID="0",
                        TEST_FIXTURES=self.fixtures.as_posix(), TEST_LOG=self.log.as_posix())
        self.command("uname", 'case "$1" in -s) echo "$TEST_OS";; -m) echo "$TEST_ARCH";; esac')
        self.command("id", 'echo "$TEST_UID"')
        self.command("curl", 'for value in "$@"; do case "$value" in https://*) url="$value";; esac; done\n'
                             'while [ "$1" != "-o" ]; do shift; done\n'
                             'cp "$TEST_FIXTURES/${url##*/}" "$2"')
        for manager in ("apt-get", "dnf", "pacman"):
            self.command(manager, 'printf "%s\\n" "$0 $*" >> "$TEST_LOG"')
        self.command("sysctl", 'exit 1')
        self.command("codesign", 'exit 0')
        self.command("update-desktop-database", 'exit 0')

    def command(self, name, body):
        path = self.bin / name
        path.write_text("#!/bin/sh\nset -eu\n" + body + "\n", encoding="utf-8")
        path.chmod(0o755)

    def archive(self, name, mac=False, traversal=False):
        with tarfile.open(self.fixtures / name, "w:gz") as archive:
            files = {"numpad-1.3.0/numpad": "#!/bin/sh\nexit 0\n",
                     "numpad-1.3.0/numpad.png": "image",
                     "numpad-1.3.0/numpad.desktop": "[Desktop Entry]\nType=Application\nExec=numpad %f\n"}
            if mac:
                files = {"numpad-1.3.0/NumPad.app/Contents/MacOS/NumPad": "#!/bin/sh\nexit 0\n"}
            if traversal:
                files = {"../outside": "must not be extracted"}
            for path, text in files.items():
                content = text.encode()
                entry = tarfile.TarInfo(path)
                entry.size = len(content)
                entry.mode = 0o755 if path.endswith(("numpad", "NumPad")) else 0o644
                archive.addfile(entry, io.BytesIO(content))
        self.manifest(name)

    def manifest(self, name, bad=False):
        digest = hashlib.sha256((self.fixtures / name).read_bytes()).hexdigest()
        if bad:
            digest = "0" * 64
        (self.fixtures / "SHA256SUMS").write_text(f"{digest}  {name}\n")

    def run_installer(self, **updates):
        env = dict(self.env, **updates)
        return subprocess.run([SH, str(ROOT / "install.sh")], env=env,
                              text=True, capture_output=True, timeout=30)

    def test_linux_tarball_install_and_update(self):
        self.archive("numpad-1.3.0-linux-x86_64.tar.gz")
        for _ in range(2):
            result = self.run_installer()
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertTrue((self.home / ".local/bin/numpad").is_file())
        desktop = (self.home / ".local/share/applications/numpad.desktop").read_text()
        self.assertIn('Exec="', desktop)
        self.assertIn("user home", desktop)

    def test_latest_release_resolution(self):
        self.archive("numpad-1.3.0-linux-x86_64.tar.gz")
        (self.fixtures / "latest").write_text('{"tag_name": "v1.3.0"}')
        result = self.run_installer(NUMPAD_VERSION="")
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_checksum_failure_changes_nothing(self):
        name = "numpad-1.3.0-linux-x86_64.tar.gz"
        self.archive(name)
        self.manifest(name, bad=True)
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Checksum mismatch", result.stderr)
        self.assertFalse((self.home / ".local").exists())

    def test_missing_checksum_changes_nothing(self):
        self.archive("numpad-1.3.0-linux-x86_64.tar.gz")
        (self.fixtures / "SHA256SUMS").write_text("")
        self.assertNotEqual(self.run_installer().returncode, 0)
        self.assertFalse((self.home / ".local").exists())

    def test_archive_traversal_is_rejected(self):
        self.archive("numpad-1.3.0-linux-x86_64.tar.gz", traversal=True)
        self.assertNotEqual(self.run_installer().returncode, 0)
        self.assertFalse((self.root / "outside").exists())

    def test_native_package_selection(self):
        for kind, name, manager in (
            ("deb", "numpad_1.3.0-1_amd64.deb", "apt-get"),
            ("rpm", "numpad-1.3.0-1.x86_64.rpm", "dnf"),
            ("arch", "numpad-1.3.0-1-x86_64.pkg.tar.zst", "pacman"),
        ):
            with self.subTest(kind=kind):
                (self.fixtures / name).write_bytes(b"mock package")
                self.manifest(name)
                result = self.run_installer(NUMPAD_INSTALL=kind)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertIn(manager, self.log.read_text())
                self.assertIn(name, self.log.read_text())

    def test_macos_architectures_and_backup(self):
        for arch in ("x86_64", "arm64"):
            with self.subTest(arch=arch):
                self.archive(f"numpad-1.3.0-macos-{arch}.tar.gz", mac=True)
                result = self.run_installer(TEST_OS="Darwin", TEST_ARCH=arch,
                                            TEST_UID="1000", NUMPAD_INSTALL="auto")
                self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue((self.home / "Applications/NumPad.app/Contents/MacOS/NumPad").exists())
        self.assertEqual(len(list((self.home / "Applications").glob("NumPad.app.backup-*"))), 1)

    def test_unsupported_platform_and_invalid_version(self):
        for updates in ({"TEST_ARCH": "aarch64"}, {"TEST_OS": "FreeBSD"},
                        {"NUMPAD_VERSION": "../bad"}, {"NUMPAD_INSTALL": "unknown"}):
            with self.subTest(updates=updates):
                self.assertNotEqual(self.run_installer(**updates).returncode, 0)
        self.assertFalse(self.log.exists())


if __name__ == "__main__":
    unittest.main()
