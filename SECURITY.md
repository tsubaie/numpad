# Security and dependency maintenance

Calculations and session documents stay local. Network access occurs only for
explicit update checks and installation. Documents are data, never executable
expressions. Do not put private calculations or credentials in public reports.

For a sensitive issue, use GitHub's private vulnerability reporting when enabled,
or contact the repository owner privately. Public reports should use fictional
reproductions without exploitable private data.

## Dependency checks

CI and the weekly schedule run `python3 scripts/audit.py` using cargo-audit 0.22.2.
The script reports all lockfile advisories and blocks vulnerabilities, unsoundness,
and yanked packages in the enabled dependency tree across all targets. Enabling a
previously disabled optional dependency automatically brings it under that gate.
New unmaintained dependencies also require review.

The known maintenance warning RUSTSEC-2026-0192 remains visible for ttf-parser
0.25.1 through Iced's fontdb/owned_ttf_parser dependencies. NumPad's PDF exporter
uses maintained skrifa instead. This narrow maintenance exception does **not**
exempt the package from vulnerability or unsoundness findings. Remove it when
Iced's font stack migrates; review the remaining paths with each dependency update.

The lockfile's lru 0.16.4 advisory RUSTSEC-2026-0253 belongs to the disabled wgpu
renderer. Do not enable that renderer until its dependency is fixed. The audit
gate detects it automatically if it becomes enabled.

Dependabot checks Cargo dependencies and pinned GitHub Actions weekly. The tested
toolchain is pinned in rust-toolchain.toml and the workflow; update both together,
run the full checks, and review behavior before adopting a compiler update.

## Release identity

Release jobs generate GitHub build-provenance attestations for the packages and
checksum manifest. After publication, verify an asset with:

```sh
gh attestation verify PATH_TO_ASSET --repo tsubaie/numpad
```

The existing installer verifies download checksums, not provenance attestations.
Attestation verification is a separate consumer check. SHA256SUMS fetched from the
same release is a corruption check, not independent publisher authentication.

Developer ID signing and Apple notarization, Windows Authenticode signing, and
Linux package signing still require publisher identities/keys. No signing keys
are included in this repository, and no release is represented as publisher-signed
without those steps. Do not disable OS trust checks to work around unsigned builds.

## Documents and persistence

Text is limited to 1 MB and 500 rows, including imported and restored tapes.
Workspaces permit up to 64 tabs and 8 MB of text; their serialized input is bounded
before parsing. Decimal scales are validated before rounding or expansion.
Rejected workspace recovery files are preserved beside the original before a new
session can replace it. If preservation fails, workspace writes stay disabled and
the app reports the error; save individual tapes before resolving the filesystem issue.
Unix saves default to mode 0600 and preserve existing permission bits. This is not
a claim that arbitrary platform ACLs or extended attributes are preserved.
Manual saves are ordered within an application process. Concurrent independent
instances editing the same explicit file still require user coordination.
