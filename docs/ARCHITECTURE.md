# NumPad architecture

NumPad is a native Rust desktop application with Iced 0.14. Source is built independently from observable CalcTape behavior. No webview or Electron runtime is required.

## Boundaries

- `lib.rs` exposes the UI-independent core; `limits.rs` centralizes document and workspace bounds. `math.rs`: decimal arithmetic and bounded scientific operations, with a bounded cache keyed by function and exact argument. Display rounding is separate from internal precision.
- `engine.rs`: line classification, assignment expressions, variable scope, precedence, percentages, subtotal calculation, structured errors and formatting.
- `editor.rs`: tape editing state and keyboard transitions, selection, generated rows, caret preservation, undo/redo and paste behavior. It can be tested without opening a window.
- `storage.rs`: versioned native JSON documents, unique temporary files and atomic writes, text/XLSX exports.
- `pdf_export.rs`: text-only PDF writing, searchable Unicode mappings, and embedded TrueType font subsets.
- `tape_content.rs`: keeps the Iced text buffer synchronized through UTF-8-safe range and line patches.
- `main.rs`: application state, input dispatch, memory, clipboard, and startup. `view.rs` owns widget construction; `file_tasks.rs` orders manual saves and exports.
- `tabs.rs`: independent tape state, tab lifecycle, shared immutable snapshots of inactive documents, and serialized background workspace saves. Revision checks prevent older completions from clearing newer edits; final close writes wait for earlier saves.
- `dialogs.rs`: tabbed Settings and guide, transactional preferences, About/update controls, and zoom tests.
- `updates.rs`: on-demand stable release checks using system curl, explicit installer launch, and update state. `scripts/update-windows.ps1` verifies checksums, waits for a saved-session commit and process exit, then replaces/restarts with rollback. No update request contains tape data.
- `appearance.rs` and `system_theme.rs`: shared widget/highlighter colors and read-only system/Omarchy theme integration.
- `e2e.rs`: feature-gated, read-only layout/state/screenshot probes for the real-app driver in `tests/e2e.rs`. Not included in normal builds.

Computed totals are re-derived from operands rather than trusted when opening a document. Documents store editable text plus preferences in a versioned `.numpad` format; formatted `.txt` tapes are importable. No proprietary `.calc` interoperability is implied.

All arithmetic input is parsed; nothing is evaluated as code. Decimal work is bounded so unreasonable exponents and document sizes cannot create unbounded allocations. Variable keys are case-insensitive. File writes use a temporary sibling and rename/replacement with error reporting. Autosave is local and preserves the last valid on-disk session when serialization or write fails.

CI builds Windows, Linux, and macOS. A separate Linux/Xvfb job drives the app with real keyboard and mouse events and checks crash recovery in isolated data directories. This does not establish interaction coverage for Wayland, Windows, or macOS; native dialogs, installer/signing behavior, and platform-specific interactions still need checking on those systems.

Editor history retains at most 300 steps and 8 MiB across undo/redo per tab, with a shared 32 MiB workspace budget divided among open tabs. Rejected/no-op edits preserve redo. Generated total formatting updates raw rows without another arithmetic pass; operand reformatting still re-evaluates. The engine continues to evaluate the full bounded tape, avoiding dependency-invalidation complexity for the current 500-line limit. Scientific decimal constants are initialized once. A 1,024-entry thread-local FIFO cache reuses scientific function results for exact normalized arguments; new arguments still require evaluation.

The release profile keeps speed optimization, full LTO, one codegen unit, and unwinding so blocking-worker panics can be surfaced as errors. RFD shares Iced's Tokio runtime, and canvas paths replace the general-purpose SVG icon backend. See [measurements](PERFORMANCE.md).

The paper canvas covers the visible viewport rather than the full tape height. System-palette polling emits UI messages only for actual changes, and autosave/status ticks stop when idle. Manual save dialogs and writes run through one bounded FIFO queue; repeated queued Ctrl+S requests retain the newest snapshot. Closing waits for this queue before flushing the workspace.
