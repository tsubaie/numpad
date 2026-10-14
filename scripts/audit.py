#!/usr/bin/env python3
"""Audit the enabled dependency graph, including platform-specific dependencies.

Cargo.lock also includes disabled optional dependencies. Report those advisories,
but only gate on packages reachable through Cargo's actual feature resolution.
"""
import json
import subprocess
import sys


def findings(report, active):
    issues = [('vulnerability', item) for item in report['vulnerabilities']['list']]
    issues += [(kind, item) for kind, items in report.get('warnings', {}).items() for item in items]
    failures = []
    for kind, item in issues:
        package = item['package']
        enabled = (package['name'], package['version']) in active
        advisory = item.get('advisory') or {}
        print(f"{'ENABLED' if enabled else 'DISABLED'} {kind}: {package['name']} {package['version']} {advisory.get('id', '')} {advisory.get('title', '')}")
        if enabled and kind in ('vulnerability', 'unsound', 'yanked'):
            failures.append(item)
        # The inherited font parser is a maintenance warning, not an exemption
        # from future vulnerability/unsoundness findings. Track it in SECURITY.md.
        if enabled and kind == 'unmaintained' and advisory.get('id') != 'RUSTSEC-2026-0192':
            failures.append(item)
    return failures


def main():
    # metadata can retain optional package edges even when their feature is off;
    # cargo tree applies the feature/target filtering used by the actual build.
    tree = subprocess.check_output(['cargo', 'tree', '--locked', '--target', 'all', '--prefix', 'none', '--edges', 'normal,build', '--format', '{p}'], text=True)
    active = {(parts[0], parts[1][1:]) for line in tree.splitlines() if len(parts := line.split()) >= 2 and parts[1].startswith('v')}
    if not active:
        sys.exit('Cargo returned an empty dependency graph')
    audit = subprocess.run(['cargo', 'audit', '--json', *sys.argv[1:]], capture_output=True, text=True)
    if audit.returncode not in (0, 1):
        sys.exit(audit.stderr or 'Dependency audit failed')
    try:
        report = json.loads(audit.stdout)
        failed = findings(report, active)
    except (ValueError, KeyError) as error:
        sys.exit(f'Invalid audit report: {error}; {audit.stderr}')
    if failed:
        sys.exit(f'{len(failed)} enabled dependency findings require attention')
    print('No blocking findings in the enabled dependency graph.')


if __name__ == '__main__':
    main()
