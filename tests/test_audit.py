import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('audit', Path(__file__).resolve().parents[1] / 'scripts/audit.py')
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class AuditTests(unittest.TestCase):
    def test_disabled_optional_advisory_becomes_blocking_when_enabled(self):
        report = {'vulnerabilities': {'list': []}, 'warnings': {'unsound': [
            {'package': {'name': 'lru', 'version': '0.16.4'}, 'advisory': {'id': 'RUSTSEC-2026-0253'}}]}}
        active = set()
        self.assertFalse(audit.findings(report, active))
        active.add(('lru', '0.16.4'))
        self.assertTrue(audit.findings(report, active))

    def test_font_maintenance_exception_never_hides_vulnerabilities(self):
        item = {'package': {'name': 'ttf-parser', 'version': '0.25.1'}, 'advisory': {'id': 'RUSTSEC-2026-0192'}}
        active = {('ttf-parser', '0.25.1')}
        report = {'vulnerabilities': {'list': []}, 'warnings': {'unmaintained': [item]}}
        self.assertFalse(audit.findings(report, active))
        report['vulnerabilities']['list'].append(item)
        self.assertTrue(audit.findings(report, active))
