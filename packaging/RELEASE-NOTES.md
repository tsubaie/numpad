## NumPad 1.4.0

A native calculation tape for Windows, Linux, and macOS.

### What's new

- Check for the latest stable release and install updates from Settings → About. Downloads are verified against release checksums; Windows saves your session before replacing and restarting the app.
- Faster editing with fewer text allocations, bounded undo memory, and background workspace autosave.
- Cached theme detection and scientific constants reduce repeated work.
- Smaller release binaries through a lightweight PDF exporter, font subsetting, simplified icons, and optimized linking. The measured Linux development build shrank from 17.52 MiB to 10.81 MiB (38.3%); sizes vary by platform.
- A compact, outlined update button, a simpler burger menu, and an author link to www.ta.sa.
- Expanded editor, updater, recovery, package, and GUI regression checks.

### Features

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
