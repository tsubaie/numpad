# Performance and binary-size work

Measured on Linux x86_64 with rustc 1.98.1 on 2026-09-10. These results describe this machine and compiler; they are not promises for other platforms. The baseline was the existing stripped `target/release/numpad` (18,375,424 bytes / 17.52 MiB).

## Changes

- The Iced buffer is synchronized with UTF-8-safe patches. Ordinary edits and generated totals patch their individual lines; line insertion/deletion patches the changed range. Cursor-only events no longer reconstruct the widget's entire text.
- Editing modifies the string in place. Recalculation no longer clones every parsed line or allocates an owned copy of every raw row. Updating generated totals does not trigger a second arithmetic pass; reformatting operands still does.
- Undo and redo share an 8 MiB/300-step limit per tab. Rejected edits preserve redo and do not create history or dirty the document. Custom calculator actions retain a single undo step even when history is full.
- Autosave serialization and durable writes run in a blocking worker, with one workspace write in flight. Revision checks retain newer dirty state. Closing waits for earlier saves before writing the final snapshot, including an empty session when the last tab is discarded. Inactive tabs share immutable document snapshots rather than copying their text on every autosave.
- Manual file reads, saves, and exports also run in blocking workers. Atomic writes have unique temporary filenames so concurrent operations do not share a temporary file.
- Omarchy paths and parsed colors are cached. Polling only runs in System mode and detects file creation, removal, and theme symlink changes.
- Scientific decimal constants, including ln(10), are initialized once. Decimal precision, arithmetic semantics, and complex-text shaping are retained.
- RFD uses the existing Tokio backend. Canvas paths replace SVG UI icons. The text-only PDF exporter uses pdf-writer, ttf-parser, and subsetter instead of printpdf's broader PDF/SVG stack. It preserves the A4 layout, pagination, searchable Unicode maps, and embedded font subsets; ASCII export still works without an installed font.

The engine still evaluates the full bounded tape. A dependency graph or rope is not introduced: ordinary 500-line calculations are already sub-millisecond, and keeping full evaluation avoids changing variable/block dependency behavior. X11 and Wayland support remain enabled.

## Release-profile experiment

The following builds used the same intermediate implementation (including the updater), so the profile comparison isolates compiler settings. Sizes are uncompressed executable bytes, not package downloads.

| Configuration | Bytes | MiB |
| --- | ---: | ---: |
| Updated implementation, speed level 3 / Thin LTO | 14,191,288 | 13.53 |
| Size level `s` / Thin LTO | 11,410,096 | 10.88 |
| Size level `z` / Thin LTO | 11,384,864 | 10.86 |
| Speed level 3 / full LTO / one codegen unit | 11,313,376 | 10.79 |
| Same full-LTO configuration, panic abort | 10,075,440 | 9.61 |

The selected release configuration is speed level 3, full LTO, one codegen unit, stripping, and **panic unwinding**. Full LTO beat the size levels without disabling vectorization. Panic abort was measured but not selected: Tokio's blocking-worker panic isolation and error reporting should not become process termination. Full LTO increases compilation/link time. One unconstrained experimental build was killed; subsequent builds used `-j 4`.

The completed executable is **11,334,112 bytes (10.81 MiB)**, a **38.3% reduction** from the original binary, including the new on-demand update feature.

## Calculation/edit microbenchmark

Run `cargo bench --locked --bench engine`. It exercises the real math, engine, and editor modules without opening a window. Each edit appends a character at the end of the first line. This is a diagnostic microbenchmark, not a GUI latency measurement; font layout, rendering, IPC, and disk I/O are excluded. Values below are one local run, rounded to milliseconds per operation.

| Input | Before: calculate | After: calculate | Before: edit | After: edit |
| --- | ---: | ---: | ---: | ---: |
| 20 arithmetic lines | 0.015 | 0.010 | 0.018 | 0.010 |
| 500 arithmetic lines | 0.399 | 0.259 | 0.432 | 0.264 |
| 500 lines / approximately 950 KB of notes | 0.760 | 0.767 | 0.943 | 0.505 |
| 100 scientific assignments | 3.647 | 3.038 | 3.544 | 2.923 |

The unchanged note-parsing result illustrates the measurement limit: not every path improved, and tiny timing differences should not be interpreted as a regression without repeated profiling. The large-note edit path benefits from fewer copies and bounded history.

## Verification

- 76 release unit tests (including UTF-8 synchronization, history bounds, theme-cache invalidation, and save revision races), formatting, and Clippy with warnings denied.
- Linux/Xvfb interaction tests: editing, clipboard, undo/redo, tabs, themes, menus, zoom, scrolling, real autosave/crash recovery, and last-tab close/discard.
- About controls exercised with a child-only curl shim: no automatic request, available/current versions, and offline failure. The real GitHub endpoint also returned a valid stable v1.3.1 response.
- 15 offline Python installer/package/wrapper tests; seven PowerShell helper tests for success, bad/duplicate checksums, missing executables, replacement rollback, invalid versions, and canceled session saves. PowerShell tests ran in Microsoft's Linux container with networking disabled and mocked downloads/process launches; they are also registered in the Windows CI job.
- Export smoke tests for native documents, Excel, and PDF. qpdf found no syntax/stream errors. A separate two-page Unicode PDF was rendered and checked with pdftotext and pdffonts: the TrueType font was embedded/subsetted and Unicode text remained searchable.

Actual installation was not performed on this workstation. Native Windows/macOS update behavior, Windows file locking, and real Wayland/X11 portal dialogs still need platform testing before publishing a release. Existing `.numpad` and workspace formats remain compatible.
