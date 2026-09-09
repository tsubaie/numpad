# Verification record

## Version 1.3 clarification and alignment

The desktop window was visually checked: tape and keypad card bottoms align, tax buttons display the configured 14%, the adjacent link reads Tax settings, Copy block is absent, and the font toolbar label describes its mode. Indian grouping and its control were subsequently removed; a regression test verifies that even legacy Indian preferences render 1,000,000.00. The default tax rate is tested as 15%. A new regression test checks tax-label normalization/localization and safe removal of legacy language preferences. The suite now contains 48 passing tests. Rustfmt and warning-free clippy checks pass.

## Version 1.2 behavior and interface update

47 regression tests pass. Tests explicitly verify 10 + 2 × 3 on separate tape rows produces 36, assignment expressions retain standard precedence, percentages can precede multiplication without a subtotal, tax add/remove share one rate and invert correctly, and the old mismatched tax name/formula migrates to one consistent configuration. Rustfmt and clippy with warnings denied pass. Historical version 1.0 and 1.1 evidence below describes the previous builds.

## Version 1.1 design update

The supplied logo is embedded in the header and native window icon, and a multi-resolution Windows executable icon resource is generated at build time. Both the light theme and Catppuccin Mocha dark theme were visually inspected in the running Windows app. Number, operator, memory and custom-key labels are centered on both axes. The old palette selector is absent. Live tape edits and the existing memory value were retained during theme changes.

43 regression tests pass, including a new legacy-preference migration test. Rustfmt and clippy with warnings denied pass. Windows release build succeeds. The original `Logo.png` is unchanged. See `DESIGN.md` for the intentional visual changes; the historical version 1.0 measurements below refer to the previous executable.

## Version 1.0 baseline

Date: 2026-09-09. Host: Windows x64, Rust 1.98.1, MSVC toolchain. Dependencies are locked in `Cargo.lock`.

Final result: **42 tests passed, zero failures; rustfmt passed; clippy passed with warnings denied.** The optimized Windows executable built successfully for `x86_64-pc-windows-msvc` using Iced's tiny-skia software renderer.

Delivered executable: `NumPad.exe` in the local project folder.
Size: 9,210,368 bytes. SHA-256: `10358B76DCB1784C79999B877A62846BC3428A51F371AA9911DFB820C372A451`.

## Reference investigation

The public CalcTape Web application was used interactively before the native editor was implemented. The manual and both wide/narrow layouts were inspected. Probes distinguished character typing from paste, tested comments, precedence, subtotals, percentages, variables, scientific functions, memory and locale behavior. Findings and remaining uncertainties are recorded in `CALCTAPE-SPEC.md`.

## Automated coverage

The Rust suite covers decimal precision, precedence, chained and independent sums, percentage operations/errors, invalid dependencies, names and assignment expressions, scientific functions, locale/grouping, numeric bounds, whole-block results, keystroke splitting, literal paste, single-value Enter, subtotal continuation, clear/undo/redo, UTF-8 deletion, correction of earlier values, row limits, naming totals, locale-safe undo, and native-document serialization.

Reproduce with:

```sh
cargo fmt --check
cargo test --release --locked
cargo clippy --release --locked -- -D warnings
cargo build --release --locked
```

## Native interaction checks

On Windows, physical number/operator key events for `10+2*3` created three operand rows and displayed 16. Enter inserted the separator and derived 16 subtotal. Literal paste preserved the reference's different interpretation. Memory addition on the subtotal stored 16 without moving the tape caret. The rendered paper, highlighting, keypad and theme controls were inspected visually.

Native window automation was occasionally interrupted by user interaction/minimization. A failed or interrupted automated action is not counted as passing evidence. In particular, file-format tests do not by themselves certify the native Save/Open dialog workflow.

The final delivered executable was launched and visually inspected. It restored the current 1,000 + 15% = 1,150 tape, High Contrast preference and memory 16. The right sidebar now scrolls instead of making its lower content unreachable in shorter windows. Observed process working set after launch was approximately 38 MB; this is a single measurement, not a performance guarantee. Ctrl+Shift+S opened a native Save As dialog. Saving through that dialog was not completed: the UI automation helper reported an accessibility-property error and the dialog subsequently closed. No modal was left open at handoff.

## Export checks

The `--export-demo` command produced a native JSON document, PDF and XLSX. The PDF was parsed using pypdf and rendered using Poppler; comments, calculated total 223.20 and page layout were checked. The workbook was reopened with openpyxl and its cells/values were checked. Native JSON roundtrip is additionally covered by a unit test.

Exports were repeated using the delivered executable. Its PDF again contained 223.20 and its workbook reopened with nine rows and five columns.

## Unverified

Linux/macOS execution, signed installers, exhaustive keyboard/IME/accessibility combinations, every hidden CalcTape engine edge case, and proprietary share formats are not verified. CI configuration is not presented as evidence of successful non-Windows builds.
