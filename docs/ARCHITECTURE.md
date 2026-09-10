# NumPad architecture

NumPad is an offline Rust application with Iced 0.14. Source is built independently from observable CalcTape behavior. No webview, Electron runtime, accounts or network service is required.

## Boundaries

- `math.rs`: decimal arithmetic and bounded scientific operations. Display rounding is separate from internal precision.
- `engine.rs`: line classification, assignment expressions, variable scope, precedence, percentages, subtotal calculation, structured errors and formatting.
- `editor.rs`: tape editing state and keyboard transitions, selection, generated rows, caret preservation, undo/redo and paste behavior. It can be tested without opening a window.
- `storage.rs`: versioned native JSON documents, local session recovery, atomic writes, text/PDF/XLSX exports.
- `main.rs`: Iced view, input dispatch, menus, memory, clipboard, file operations, and startup.
- `tabs.rs`: independent tape state, tab lifecycle, and atomic workspace recovery.
- `dialogs.rs`: tabbed Settings and guide, transactional preferences, About, and zoom tests.
- `appearance.rs` and `system_theme.rs`: shared widget/highlighter colors and read-only system/Omarchy theme integration.
- `e2e.rs`: feature-gated, read-only layout/state/screenshot probes for the real-app driver in `tests/e2e.rs`. Not included in normal builds.

Computed totals are re-derived from operands rather than trusted when opening a document. Documents store editable text plus preferences in a versioned `.numpad` format; formatted `.txt` tapes are importable. No proprietary `.calc` interoperability is implied.

All arithmetic input is parsed; nothing is evaluated as code. Decimal work is bounded so unreasonable exponents and document sizes cannot create unbounded allocations. Variable keys are case-insensitive. File writes use a temporary sibling and rename/replacement with error reporting. Autosave is local and preserves the last valid on-disk session when serialization or write fails.

CI builds Windows, Linux, and macOS. A separate Linux/Xvfb job drives the app with real keyboard and mouse events and checks crash recovery in isolated data directories. This does not establish interaction coverage for Wayland, Windows, or macOS; native dialogs, installer/signing behavior, and platform-specific interactions still need checking on those systems.
