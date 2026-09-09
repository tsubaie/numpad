# NumPad architecture

NumPad is an offline Rust application with Iced 0.14. Source is built independently from observable CalcTape behavior. No webview, Electron runtime, accounts or network service is required.

## Boundaries

- `math.rs`: decimal arithmetic and bounded scientific operations. Display rounding is separate from internal precision.
- `engine.rs`: line classification, assignment expressions, variable scope, precedence, percentages, subtotal calculation, structured errors and formatting.
- `editor.rs`: tape editing state and keyboard transitions, selection, generated rows, caret preservation, undo/redo and paste behavior. It can be tested without opening a window.
- `storage.rs`: versioned native JSON documents, local session recovery, atomic writes, text/PDF/XLSX exports.
- `main.rs`: Iced view, input dispatch, menus/dialogs, themes, memory, custom keys, file operations and startup.

Computed totals are re-derived from operands rather than trusted when opening a document. Documents store editable text plus preferences in a versioned `.numpad` format; formatted `.txt` tapes are importable. No proprietary `.calc` interoperability is implied.

All arithmetic input is parsed; nothing is evaluated as code. Decimal work is bounded so unreasonable exponents and document sizes cannot create unbounded allocations. Variable keys are case-insensitive. File writes use a temporary sibling and rename/replacement with error reporting. Autosave is local and preserves the last valid on-disk session when serialization or write fails.

Windows is built and exercised on the current machine. Linux and macOS share the same source and receive native build instructions and CI jobs. Their installer/signing behavior must be checked on those operating systems.
