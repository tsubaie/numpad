# Builds and releases

## CI and caching

The Native builds workflow validates Rust formatting, runs Clippy and tests, builds optimized binaries, and packages Linux x86_64, Windows x86_64, and both macOS architectures. Linux also runs offline installer tests and installs the real packages in fresh Debian, Fedora, and Arch containers before running headless export checks.

Rust registry downloads and compiled dependencies are cached by runner, target, toolchain, and dependency inputs. Workspace crates are cached too. Main-branch builds populate the cache; pull requests and version tags can restore it. nFPM is pinned and separately cached. A first build or changed toolchain still needs to compile dependencies.

Documentation-only pushes skip native builds. Superseded branch/PR runs are cancelled; version-tag runs are not. Ordinary build assets remain available under Actions for 14 days.

## Publish a version

1. Update the package version in Cargo.toml and regenerate Cargo.lock. Keep the visible app version and documentation current.
2. Update packaging/RELEASE-NOTES.md with changes, supported architectures, and limitations.
3. Commit and push to main. Wait for native build and package checks to pass.
4. Tag that exact commit and push the tag:

~~~sh
git tag -a v1.3.0 -m "NumPad 1.3.0"
git push origin v1.3.0
~~~

Use the actual release version, not necessarily the example. Packaging rejects a tag that does not match Cargo.toml. Do not move a published version tag.

The release job runs only after every build/package job succeeds. It collects all assets, generates SHA256SUMS, creates a draft, uploads the complete set, then publishes it. It refuses to overwrite an already-published release. A failed draft can be retried; a published correction needs a new version.

A manual workflow run against an existing version tag can retry that tag. A manual run against main builds artifacts but does not publish.

## Asset contract

For version X.Y.Z, the installer expects these names:

| Target | Asset |
| :--- | :--- |
| Debian/Ubuntu x86_64 | numpad_X.Y.Z-1_amd64.deb |
| Fedora x86_64 | numpad-X.Y.Z-1.x86_64.rpm |
| Arch x86_64 | numpad-X.Y.Z-1-x86_64.pkg.tar.zst |
| Linux x86_64, per-user install | numpad-X.Y.Z-linux-x86_64.tar.gz |
| macOS Apple Silicon | numpad-X.Y.Z-macos-arm64.tar.gz and .dmg |
| macOS Intel | numpad-X.Y.Z-macos-x86_64.tar.gz and .dmg |
| Windows x86_64 | NumPad-X.Y.Z-windows-x64.zip |
| All assets | SHA256SUMS |

Linux binaries use an Ubuntu 22.04/glibc 2.35 baseline. Packages declare runtime libraries. Wayland file dialogs need a desktop portal and a backend appropriate to the desktop. Alpine/musl and Linux ARM are not supported by these prebuilt assets.

macOS packages are ad-hoc signed, not Developer ID-signed or notarized. Windows binaries and Linux packages are not publisher-signed. Checksums detect corruption relative to the release manifest; they are not independent publisher authentication. The installer does not disable operating-system security checks.

## Local packaging

After building a native executable, choose the current OS (Python 3.11+):

~~~sh
python scripts/package.py --platform linux --binary target/release/numpad --output dist
python scripts/package.py --platform macos --arch arm64 --binary target/release/numpad --output dist
python scripts/package.py --platform windows --binary target/release/numpad.exe --output dist
python scripts/checksums.py dist
~~~

Linux also requires nFPM 2.47.0: run sh scripts/setup-nfpm.sh, then add .tools/nfpm to PATH. macOS packaging uses the system sips, iconutil, codesign, and hdiutil tools. The Windows and macOS wrappers build and invoke the shared packager.

Use a clean output directory per release. Never upload the development release/ directory wholesale: it can contain old builds and private session backups.

## Quick installer

The root install.sh downloads only from this project's GitHub releases and checks the selected asset against SHA256SUMS before installation. It does not compile source or modify calculation documents.

- Linux selects apt, dnf, or pacman when available; otherwise it installs the tarball into ~/.local.
- NUMPAD_INSTALL=tarball forces a per-user Linux install. Runtime libraries remain the user's responsibility.
- macOS chooses Intel or Apple Silicon, including Rosetta detection, and installs in ~/Applications/NumPad.app. Updates preserve the previous app as a timestamped sibling backup.
- NUMPAD_VERSION=X.Y.Z selects a release; otherwise the latest stable release is used.
- Re-running the installer updates the app. It does not register an automatic updater or a package repository.

Remove native Linux packages through the package manager. For a per-user Linux install, remove only ~/.local/bin/numpad, ~/.local/share/applications/numpad.desktop, and ~/.local/share/icons/hicolor/256x256/apps/numpad.png. On macOS, move the app and unwanted app backups to Trash. Session data and saved documents remain separate.
