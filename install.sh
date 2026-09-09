#!/bin/sh
# NumPad quick installer: Linux x86_64 and macOS Intel/Apple Silicon.
# Read this script before piping it to sh. No source compilation is performed.
set -eu

say() { printf '%s\n' "$*"; }
fail() { say "NumPad installer: $*" >&2; exit 1; }
fetch() { curl --proto '=https' --tlsv1.2 --retry 3 -fsSL "$1" -o "$2"; }
as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"
    elif command -v sudo >/dev/null 2>&1; then
        # Read prompts from the user's terminal, not the piped installer source.
        { sudo "$@"; } </dev/tty
    else fail "Root or sudo is required. Use NUMPAD_INSTALL=tarball for a per-user Linux install."
    fi
}
checksum() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d ' ' -f1
    elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | cut -d ' ' -f1
    else fail "Install sha256sum or shasum before continuing."
    fi
}
cleanup() {
    case "${temporary:-}" in
        */numpad-install.*) [ ! -d "$temporary" ] || rm -rf -- "$temporary" ;;
    esac
}
main() {
    repository=tsubaie/numpad
    command -v curl >/dev/null 2>&1 || fail "curl is required."
    system="$(uname -s)"
    machine="$(uname -m)"
    kind="${NUMPAD_INSTALL:-auto}"
    case "$system/$machine" in
        Linux/x86_64|Linux/amd64) architecture=x86_64 ;;
        Darwin/arm64|Darwin/aarch64) architecture=arm64 ;;
        Darwin/x86_64)
            architecture=x86_64
            # Detect Apple Silicon when the shell is running under Rosetta.
            if [ "$(sysctl -n sysctl.proc_translated 2>/dev/null || true)" = 1 ]; then architecture=arm64; fi ;;
        *) fail "Prebuilt downloads support Linux x86_64 and macOS Intel/Apple Silicon. See https://github.com/$repository#build-from-source" ;;
    esac
    if [ "$system" = Darwin ]; then
        case "$kind" in auto|macos) kind=macos ;; *) fail "On macOS, NUMPAD_INSTALL must be auto or macos." ;; esac
        [ "$(id -u)" -ne 0 ] || fail "Run the macOS installer as your normal user, without sudo."
    elif [ "$kind" = auto ]; then
        if command -v pacman >/dev/null 2>&1; then kind=arch
        elif command -v apt-get >/dev/null 2>&1 && command -v dpkg >/dev/null 2>&1; then kind=deb
        elif command -v dnf >/dev/null 2>&1; then kind=rpm
        else kind=tarball
        fi
    fi
    case "$kind" in deb|rpm|arch|tarball|macos) ;; *) fail "NUMPAD_INSTALL must be auto, deb, rpm, arch, tarball, or macos." ;; esac
    if [ "$system" = Linux ] && [ "$kind" = macos ]; then fail "A macOS app cannot be installed on Linux."; fi
    temporary="$(mktemp -d "${TMPDIR:-/tmp}/numpad-install.XXXXXXXX")"
    trap cleanup EXIT
    trap 'exit 1' HUP INT TERM
    ver="${NUMPAD_VERSION:-}"
    ver="${ver#v}"
    if [ -z "$ver" ]; then
        fetch "https://api.github.com/repos/$repository/releases/latest" "$temporary/release.json" ||
            fail "No published release could be fetched. Check https://github.com/$repository/releases"
        ver="$(sed -n 's/.*"tag_name": *"v\([0-9][0-9.]*\)".*/\1/p' "$temporary/release.json" | head -n 1)"
    fi
    printf '%s\n' "$ver" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$' ||
        fail "Expected a release version such as 1.3.0."
    case "$kind" in
        deb) asset="numpad_${ver}-1_amd64.deb" ;;
        rpm) asset="numpad-${ver}-1.x86_64.rpm" ;;
        arch) asset="numpad-${ver}-1-x86_64.pkg.tar.zst" ;;
        tarball) asset="numpad-${ver}-linux-x86_64.tar.gz" ;;
        macos) asset="numpad-${ver}-macos-${architecture}.tar.gz" ;;
    esac
    base="https://github.com/$repository/releases/download/v$ver"
    say "Downloading NumPad $ver ($asset)"
    fetch "$base/$asset" "$temporary/$asset" || fail "Download failed: $asset"
    fetch "$base/SHA256SUMS" "$temporary/SHA256SUMS" || fail "Release checksums are unavailable."
    expected="$(awk -v name="$asset" '$2 == name {print $1}' "$temporary/SHA256SUMS")"
    printf '%s\n' "$expected" | grep -Eq '^[0-9a-f]{64}$' ||
        fail "Missing or ambiguous checksum for $asset."
    actual="$(checksum "$temporary/$asset")"
    [ "$actual" = "$expected" ] || fail "Checksum mismatch; nothing was installed."
    say "Checksum verified."
    case "$kind" in
        deb) as_root apt-get install -y "$temporary/$asset" ;;
        rpm) as_root dnf install -y "$temporary/$asset" ;;
        arch) as_root pacman -U --noconfirm "$temporary/$asset" ;;
        tarball|macos)
            # Reject unexpected archive roots and traversal before extraction.
            tar -tzf "$temporary/$asset" > "$temporary/entries"
            awk -v root="numpad-$ver" '
                $0 !~ ("^" root "(/|$)") || $0 ~ /(^|\/)\.\.(\/|$)/ {bad=1}
                END {exit bad}
            ' "$temporary/entries" || fail "Unexpected archive layout."
            tar -xzf "$temporary/$asset" -C "$temporary"
            bundle="$temporary/numpad-$ver"
            if [ "$kind" = macos ]; then
                source_app="$bundle/NumPad.app"
                [ -x "$source_app/Contents/MacOS/NumPad" ] || fail "App executable is missing."
                codesign --verify --deep --strict "$source_app" || fail "App code-signature verification failed."
                mkdir -p "$HOME/Applications"
                destination="$HOME/Applications/NumPad.app"
                stage="$(mktemp -d "$HOME/Applications/.numpad-install.XXXXXXXX")"
                if ! cp -R "$source_app" "$stage/NumPad.app"; then
                    fail "Could not stage the app. Your existing app is unchanged; staging is at $stage."
                fi
                if [ -e "$destination" ] || [ -L "$destination" ]; then
                    backup="$HOME/Applications/NumPad.app.backup-$(date +%Y%m%d%H%M%S)-$$"
                    mv "$destination" "$backup"
                    say "Previous app preserved at $backup"
                fi
                if ! mv "$stage/NumPad.app" "$destination"; then
                    if [ -n "${backup:-}" ]; then mv "$backup" "$destination"; fi
                    fail "Could not install the staged app."
                fi
                rmdir "$stage"
                say "Installed in $destination. Open it from Finder."
                say "This build is ad-hoc signed, not Apple-notarized. macOS may ask you to approve opening it."
            else
                [ -x "$bundle/numpad" ] || fail "Executable is missing."
                prefix="$HOME/.local"
                mkdir -p "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/256x256/apps"
                stage="$(mktemp "$prefix/bin/.numpad-install.XXXXXXXX")"
                install -m755 "$bundle/numpad" "$stage"
                mv -f "$stage" "$prefix/bin/numpad"
                install -m644 "$bundle/numpad.png" "$prefix/share/icons/hicolor/256x256/apps/numpad.png"
                # Escape desktop-entry quoted Exec syntax, including literal percent signs.
                # Dollar signs and backticks are literal characters here, not shell expansions.
                # shellcheck disable=SC2016
                desktop_binary="$(printf '%s' "$prefix/bin/numpad" | sed 's/\\/\\\\/g; s/"/\\"/g; s/\$/\\$/g; s/`/\\`/g; s/%/%%/g')"
                sed '/^Exec=/d' "$bundle/numpad.desktop" > "$prefix/share/applications/numpad.desktop"
                printf 'Exec="%s" %%f\n' "$desktop_binary" >> "$prefix/share/applications/numpad.desktop"
                if command -v update-desktop-database >/dev/null 2>&1; then
                    update-desktop-database "$prefix/share/applications" || true
                fi
                say "Installed in $prefix/bin/numpad. Add $prefix/bin to PATH if needed."
                say "Runtime libraries and a desktop portal must be provided by your Linux distribution."
            fi ;;
    esac
    say "NumPad $ver is installed. Your existing calculation documents were not changed."
}
main "$@"
