# Performance and binary-size work

## October 2026 reliability and performance pass

Measured on Linux x86_64 with rustc 1.99.0 on 2026-10-14, against commit
`9948248`. These are local measurements, not guarantees for other machines.

The paper canvas now covers only the visible viewport. Unchanged system-palette
polls no longer send redraw messages, and autosave/status ticks stop when idle.
Scientific functions use a bounded cache keyed by function and exact normalized
argument. Arithmetic validation checks scales before expensive operations without
copying coefficients on the common path. Operand formatting retains tiny decimal
values instead of applying the result display's 12-place rounding limit.

The core is exposed as a library. The benchmark uses the same fresh editor state
for each edit sample, five warmups, and median/p95 reporting. Both versions below
used the same benchmark harness, compiler, dependencies, and release profile
(speed level 3, full LTO, one codegen unit). The baseline core was extracted from
`9948248`; measurements ran after compilation completed. GUI, disk I/O, and editor
fixture construction are excluded. Run the current benchmark with:

```sh
cargo bench --no-default-features --locked --bench engine
```

| Workload | Before calculate median | After calculate median / p95 | Before edit median | After edit median / p95 |
| --- | ---: | ---: | ---: | ---: |
| 20 arithmetic rows | 0.012 ms | 0.011 / 0.017 ms | 0.012 ms | 0.011 / 0.017 ms |
| 500 arithmetic rows | 0.273 ms | 0.264 / 0.296 ms | 0.278 ms | 0.267 / 0.291 ms |
| 500 rows / 950 KB notes | 0.437 ms | 0.427 / 0.454 ms | 0.791 ms | 0.745 / 0.788 ms |
| 500 scientific rows, repeated argument | 16.218 ms | 0.382 / 0.393 ms | 16.179 ms | 0.382 / 0.395 ms |
| 500 scientific rows, distinct arguments | 19.824 ms | 0.876 / 0.933 ms | 19.852 ms | 0.819 / 0.966 ms |

The table measures warm caches. With an empty function cache, the updated repeated
argument workload takes 0.687 ms median; 500 distinct arguments take **20.564 ms**
(27.594 ms p95), versus 20.941 ms median before. A new scientific document can
still exceed a 16.7 ms frame budget; cached recalculation is the large improvement.
Small arithmetic/note differences are within normal local timing variation.

The normal release executable, without E2E probes, was also measured in Xvfb at
1280×1024. After two seconds to settle, process CPU time was sampled for three
seconds. The updated process accumulated no CPU ticks during these samples;
this means below the sampling resolution, not literally zero work forever.

| Visible tape | Before idle CPU, one core | After sampled idle CPU | Before / after RSS |
| --- | ---: | ---: | ---: |
| Short tape | 6.00% | 0.00% | 23,640 / 21,884 KiB |
| 500 arithmetic rows | 28.00% | 0.00% | 26,840 / 24,008 KiB |
| 500 note rows | 20.67% | 0.00% | 26,044 / 23,868 KiB |

Visible-window startup samples ranged from 40–150 ms after the changes; this pass
does not establish a startup improvement. Executable size is essentially unchanged:
11,286,000 → **11,301,616 bytes** (10.76 → 10.78 MiB).

Validation includes 92 release Rust tests, 17 Python tests, formatting, Clippy,
ShellCheck, workflow linting, core-only tests, and compilation of the fuzz target.
The complete X11 desktop suite passed editing/clipboard, update checks, menus,
themes, compact layout, zoom, tabs, crash recovery, and last-tab close protection.
PDF syntax/streams, font embedding, XLSX ZIP/XML, and native-document exports were
checked. The dependency audit uses a current local advisory database; the local
offline run excludes registry yanked checks, while CI enables them. See
[quality and platform acceptance](QUALITY.md) and [security limits](../SECURITY.md).

## Historical September 2026 measurements

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
