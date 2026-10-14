# Reliability and platform verification

## Automated checks

```sh
cargo fmt --check
cargo test --release --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --no-default-features --lib --locked
python3 -m unittest discover -s tests -p 'test_*.py' -v
python3 scripts/audit.py
xvfb-run -a -s '-screen 0 1280x1200x24' cargo test --release --locked --features e2e --test e2e
cargo bench --no-default-features --locked --bench engine
```

The UI suite needs Xvfb, xdotool, ImageMagick (`import`), and system fonts. Use
isolated NUMPAD_DATA_DIR/XDG directories for manual testing. Never use personal
documents in screenshots or fixtures.

The core invariant suite generates decimal values and UTF-8 edit sequences and
checks precision-preserving formatting, undo/redo, malformed exponents, document
limits (including undo and generated-total expansion), rejected recovery backups,
and private atomic replacement. Save-queue tests check coalescing and
waiting for manual writes before final workspace persistence.
Selection regressions cover Unicode word bounds, triple-click line selection,
copy/replacement/deletion, and preserving native selection during synchronization.
The desktop suite also checks that spaces immediately move the subtotal caret.
Comment styling is checked in both themes, including an installed slanted-font
fallback when the default monospace family has no italic face.
Layout regressions exercise 420×580, 464×1112, 700×580 and 900×1000 windows,
check that controls do not squeeze the tape, and inspect rendered pixels to
ensure long green comments cannot paint outside the tape viewport.

For coverage-guided fuzzing, install cargo-fuzz and a nightly toolchain, then run:

```sh
cargo +nightly fuzz run inputs -- -max_len=8192 -max_total_time=60
```

The fuzz target exercises numbers, expressions, tape parsing and document
validation. Keep fuzz artifacts private until triaged; turn each confirmed failure
into a small deterministic regression test. Fuzz and application lockfiles are
separate; refresh them together when changing dependencies.

The October 2026 hardening validation compiled this fuzz target and ran the
deterministic invariant suite. It did not run a coverage-guided fuzzing campaign.

## Native platform acceptance

An X11 test pass does not certify Windows, macOS, Wayland, assistive technology,
or native file dialogs. Before release, check each supported platform:

| Area | Acceptance |
| --- | --- |
| Open/save | Unicode paths, read-only targets, cancellation, disk-full errors, rapid repeated saves, editing while saving, close while saving |
| Keyboard | Tab/Shift+Tab, visible focus, shortcuts, keyboard-only menus/settings, selection and clipboard |
| IME | Composition, commit, cancel and backspace using an actual platform IME; Arabic/CJK notes and mixed-direction selections |
| Accessibility | Inspect accessible names, roles, focus order and announcements with the native screen reader; record unsupported Iced behavior explicitly |
| Visuals | Light/dark/system, minimum window, large zoom/DPI, long tapes, scroll/caret alignment |
| Recovery | Forced termination, malformed workspace, multiple tabs, failed final save |
| Updates | Native updater, file locking/rollback, publisher identity and OS prompts |
| PDF | Embedded font, extracted text and visual output for Latin and other intended scripts |

PDF fallback tries the supported installed fonts before using the ASCII fallback.
Font coverage is not shaping: complex-script/RTL PDF output remains uncertified.
The editor's native complex-text shaping stays enabled. Do not infer screen-reader
or IME support from unit tests that manipulate UTF-8 strings.
