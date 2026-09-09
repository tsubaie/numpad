## NumPad 1.3.0

A native, offline calculation tape for Windows, Linux, and macOS.

- Editable calculations with notes, named values, percentages, memory, and live totals.
- Top-to-bottom tape arithmetic; assignments use standard precedence.
- Catppuccin Mocha dark and light themes.
- Shared tax settings with a configurable 15% default.
- Local session recovery, native documents, and text/PDF/Excel export.

### Downloads

Choose .deb for Debian/Ubuntu, .rpm for Fedora, .pkg.tar.zst for Arch, or the Linux tarball for a per-user install. Linux and Windows assets are x86_64. macOS has separate Intel (x86_64) and Apple Silicon (arm64) builds, each as a DMG and app tarball.

Windows: extract the ZIP and run NumPad.exe. macOS: open the matching DMG and drag NumPad to Applications.

Quick install on Linux or macOS:

~~~sh
curl -fsSL https://raw.githubusercontent.com/tsubaie/numpad/main/install.sh | sh
~~~

Read the installer before running it. Downloads are checked against SHA256SUMS. Linux native packages may require sudo; macOS installs for the current user.

### Requirements and limitations

- Linux: glibc 2.35+ and the declared desktop libraries. Wayland file dialogs need a working desktop portal/backend.
- macOS: 11.0+ and the matching architecture. Apps are ad-hoc signed, not Apple-notarized; macOS may require approval to open them.
- Windows: a 64-bit Windows desktop. Binaries are not publisher-signed.
- Build and headless export checks do not replace interactive testing on every desktop.
- Checksums provide download integrity, not independent publisher authentication.
