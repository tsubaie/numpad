"""Offline checks for the terminal wrapper; never launches a real installer."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

WRAPPER = Path(__file__).resolve().parents[1] / "scripts/update-unix.sh"


@unittest.skipIf(os.name == "nt", "Unix terminal wrapper")
class UpdateWrapperTests(unittest.TestCase):
    def run_wrapper(self, status, canceled=False):
        with tempfile.TemporaryDirectory(prefix="numpad-update-test-") as directory:
            root = Path(directory)
            installer = root / "installer with spaces.sh"
            result = root / "result"
            capture = root / "environment"
            installer.write_text(
                '#!/bin/sh\nprintf "%s|%s" "$NUMPAD_VERSION" "$NUMPAD_INSTALL" '
                f'> "$NUMPAD_TEST_CAPTURE"\nexit {status}\n'
            )
            if canceled:
                result.with_suffix(".cancel").write_text("canceled")
            completed = subprocess.run(
                ["sh", str(WRAPPER), str(installer), str(result), "1.3.2", "tarball"],
                env={**os.environ, "NUMPAD_TEST_CAPTURE": str(capture)},
                input="\n", text=True, capture_output=True, timeout=5,
            )
            if canceled:
                self.assertNotEqual(completed.returncode, 0)
                self.assertFalse(capture.exists())
            else:
                self.assertEqual(completed.returncode, status)
                self.assertEqual(result.read_text(), str(status))
                self.assertEqual(capture.read_text(), "1.3.2|tarball")
                self.assertTrue(result.with_suffix(".started").exists())

    def test_success_forwards_version_and_install_kind(self):
        self.run_wrapper(0)

    def test_failure_reports_the_installer_status(self):
        self.run_wrapper(5)

    def test_late_terminal_does_not_run_a_canceled_update(self):
        self.run_wrapper(0, canceled=True)
