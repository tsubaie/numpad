# NumPad

A native, offline calculation tape written in **Rust + Iced 0.14**. It runs without Electron, a browser, an account, or a server.

## Run on Windows

The local portable package includes **NumPad.exe**, which needs no installer. For a source checkout from GitHub, follow the build instructions below; executables and personal calculation files are not tracked in the repository.

Type `10+2*3` and press Enter. Each operator starts a new tape line; the result is 36. Add comments after numbers, edit earlier values, use percentages and named values, and save your work as a `.numpad` document. A blank line or a standalone note starts an independent calculation.

## Documentation

- [CalcTape behavior and grammar specification](docs/CALCTAPE-SPEC.md): observed rules, public documentation, grammar, editor transitions, visual layout, limits and unresolved behavior.
- [Architecture](docs/ARCHITECTURE.md): arithmetic, editor and persistence boundaries.
- [Compatibility matrix](docs/PARITY.md): what NumPad implements and where it differs.
- [Verification](docs/VERIFICATION.md): build/test evidence and platform limitations.
- [Design](docs/DESIGN.md): supplied logo, two-theme appearance and centered keypad in version 1.1.

The in-app Guide includes examples for percentages, variables and subtotals. Undo restores your tape after loading an example.

## Build and test

Install a stable Rust toolchain. Windows requires the Visual Studio C++ build tools and Windows SDK; macOS requires Xcode command-line tools. On Debian/Ubuntu, install `build-essential pkg-config libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libx11-dev libxi-dev libxcursor-dev libxrandr-dev libgl1-mesa-dev`.

```sh
cargo test --release --locked
cargo build --release --locked
cargo run --release --locked
```

The executable is `target/release/numpad.exe` on Windows, or `target/release/numpad` on Linux/macOS. Native build jobs for all three platforms are defined in `.github/workflows/build.yml`. Windows has been tested locally; see the verification record and GitHub Actions for platform-specific build results.

On Windows, run `scripts/package.ps1` to build and produce a portable ZIP. On macOS, run `scripts/package-macos.sh` to create an unsigned `.app` bundle. Signing/notarization requires your own distribution identity.

## Files and recovery

Version 1.2 evaluates tape rows top-to-bottom; assignment expressions retain standard precedence. Existing tapes will recalculate with the new tape rule. New saves use document version 2; version 1 documents still open. Original files are not overwritten unless you explicitly save them.

Tax buttons share one name/rate configuration. “+ VAT (rate%)” adds tax and “− VAT (rate%)” removes included tax using the reciprocal factor. Export is in the hamburger menu; appearance is in Settings (the app is English-only).

Ctrl/Cmd+S saves a versioned JSON `.numpad` document. Ctrl/Cmd+O opens that format or a text tape. Native documents preserve numeric settings, view preferences, shared tax settings and memory. Values displayed on generated sum lines are recalculated on load. PDF, Excel and text export are available from the Export menu.

The session is autosaved locally under the platform's application-data directory. On Windows this is normally `%LOCALAPPDATA%\NumPad\NumPad\data\session.numpad` (the exact directory follows the Rust `directories` crate). Set `NUMPAD_DATA_DIR` to an explicit folder to isolate a test session or make storage portable. No calculations are sent over the network.

PDF export embeds an available installed Consolas, DejaVu Sans Mono or Courier New font. When no suitable font is found, ASCII tapes use the PDF standard Courier font; non-ASCII export reports the missing font instead of silently dropping text. Complex-script PDF shaping has not been certified.

XLSX exports contain values rather than executable spreadsheet formulas. Values exceeding Excel's 15-digit precision are preserved as text.

## Shortcuts

| Shortcut | Action |
|---|---|
| Enter / `=` | Close a multi-operand calculation |
| Ctrl/Cmd+Z / Y | Undo / redo |
| Ctrl/Cmd+N | Clear tape, undoable |
| Ctrl/Cmd+O | Open |
| Ctrl/Cmd+S / Shift+S | Save / Save as |
| Ctrl/Cmd+Shift+L | Toggle ruled paper |
| Ctrl/Cmd+Shift+plus/minus/0 | Zoom / reset |
| F1 | Guide |

## Headless export verification

```sh
numpad --export-demo ./test-results
```

This writes `sample.pdf`, `sample.xlsx` and `sample.numpad` into the specified directory.

## Attribution

NumPad is an independent implementation based on use of the public CalcTape Web application and its manual. It does not bundle CalcTape source, branding or proprietary codecs. The original reference is <https://calctape.app/en>. Open-source dependencies retain their respective licenses; versions are recorded in `Cargo.lock`.
