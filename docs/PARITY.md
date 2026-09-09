# CalcTape compatibility

NumPad is a native, independent implementation of the observed calculation-tape workflow, not a binary-compatible or pixel-identical port. The behavioral reference and evidence boundaries are in [CALCTAPE-SPEC.md](CALCTAPE-SPEC.md).

## Implemented

| Area | NumPad behavior |
|---|---|
| Typed arithmetic | Operators start new operand rows; Enter/equals creates a separator and derived sum after multiple operands |
| Paste | Literal tape input, deliberately distinct from per-key input; trailing inline expression-like text is a comment |
| Evaluation | Top-to-bottom tape operations; standard precedence within assignment expressions; live correction of earlier values |
| Calculations | Independent blocks, chained subtotals, caret-dependent result and completed-block grand total |
| Text | Standalone notes, inline comments, multiline selection, clipboard, undo/redo |
| Percentages | Relative addition/deduction, multiplication/division and annotations; following operations run sequentially |
| Variables | Case-insensitive assignments and references, assignment expressions, named generated sums, dependency errors |
| Functions | sqrt, abs, round, sin, cos, tan, ln, log and exp; trigonometry in radians |
| Precision | Decimal arithmetic with 60 significant internal digits, configurable 0–12 display places, 32 integer-digit bound |
| Number display | Decimal point/comma and fixed three-digit grouping |
| Memory | Current operand/selection M+/M−, recall, clear and visible stored value |
| Tax | One shared tax name/rate and automatic-calculation setting for add/remove tax; button rates derived directly from settings |
| Appearance | One light theme and Catppuccin Mocha dark theme, proportional/monospaced font, ruled paper, zoom, native desktop sidebar |
| Files | Versioned .numpad JSON, text import/export, PDF and Excel export, native file dialogs |
| Recovery | Local autosave, save-on-close, session/preferences/memory restoration |
| Help | Built-in guide and three undoable example tapes |
| Limits | 500-row editing limit and bounded input; oversized input rejected with a notice instead of silently truncated |

## Intentional native adaptations and outstanding differences

- **Requested arithmetic change (1.2):** tape rows now execute strictly top-to-bottom. `10`, `+2`, `*3` gives 36. Assignments such as `x=10+2*3` still give 16. Old documents are recalculated using the new tape rule. This intentionally differs from CalcTape.
- **Requested UI simplification (1.2):** Export lives under the hamburger menu; theme and language live only in Settings. The sidebar help/promotional card is removed. Standard line icons replace undo/redo/backspace glyphs. Tax has one shared settings dialog.

- **No proprietary interoperability:** CalcTape `.calc` files and compressed share-link/QR payloads are not supported. NumPad's `.numpad` format is openly readable JSON. No fabricated compatibility is implied by changing an extension.
- **Desktop layout:** Iced widgets reproduce the paper-tape arrangement but not the original browser's exact CSS, font metrics, animations or responsive mobile keypad. The sidebar scrolls in short windows.
- **Language coverage (1.3):** English only. The incomplete language selector has been removed; old language preferences are safely ignored.
- **Theme differences (requested design change):** one light theme and Catppuccin Mocha dark mode replace the reference's palette selection. NumPad uses the supplied app logo and its own visual styling. Tax defaults remain 15% rather than a country-dependent table.
- **Secondary interactions:** reference news/store links, five guided tours, timed block selection, long-press subtotal menu and repeated-value variable suggestions are not reproduced. The guide and ordinary selection/copy provide the essential editing workflow.
- **Limit handling:** an over-limit paste is rejected intact; there is no truncation-confirmation dialog. Existing over-500-line documents can load, subject to byte limits. Undo is limited to 300 snapshots.
- **Grammar extensions/bounds:** NumPad can recognize explicit `=name` subtotal rows in pasted tapes; the reference only established naming through a generated subtotal. NumPad's exact rounding, numeric bounds, Unicode identifiers and scientific approximation algorithms are implementation choices where reference behavior is unresolved.
- **Exports:** PDF layout and Excel columns are NumPad-specific. Excel stores calculated values, not formulas; long numbers are text to avoid Excel precision loss. Installed fonts are embedded in PDFs where available; complex-script shaping is not certified.
- **Accessibility:** native text editing supports keyboard navigation, but full screen-reader and IME parity has not been certified. Ruled-paper scroll alignment under every font/zoom/long-document combination is not certified.

## Platform status

Windows x64 is built and exercised locally. Linux and macOS build definitions and packaging files are included, but those platforms have not been executed on this Windows machine. Their native CI jobs are ready to run after this source is put in a GitHub repository. macOS signing/notarization and Linux distribution-specific packaging remain distribution work, not completed verification.

This matrix is the compatibility contract for this delivery. The project should not be advertised as an exact 1:1 clone until the outstanding differences have been resolved and tested.
