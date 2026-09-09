# Contributing to NumPad

Thanks for helping improve NumPad. Small, focused contributions are welcome, especially reproducible bug reports, calculation regression tests, and native platform testing.

## Local development

Follow the prerequisites in [README.md](README.md), then run `cargo run --locked`. For an isolated session, set `NUMPAD_DATA_DIR` to a temporary directory before launching the app. Do not use personal calculation documents in screenshots or test fixtures.

Before submitting a pull request, run:

```sh
cargo fmt --check
cargo test --release --locked
cargo clippy --all-targets --locked -- -D warnings
```

For export changes, also run the compiled executable with `--export-demo ./test-results`, then inspect the generated PDF, XLSX, and native document. Keep generated binaries and personal documents out of commits.

## Reporting a bug

Include your operating system, NumPad version or commit, exact input and keystrokes, expected result, and actual result. Say whether the input was typed or pasted: typing operators creates tape rows, while paste inserts literal text. Use a minimal fictional example with no private data.

For UI issues, include the theme, window size or display scaling, and a screenshot if useful. Native build success does not replace checking the app on the affected desktop.

## Making a change

- Keep pull requests focused and explain the user-visible behavior they change.
- Add regression tests for arithmetic, editing, or persistence changes.
- Preserve the distinction between sequential tape operations and standard-precedence assignment expressions.
- Check both light and dark themes when changing the interface.
- Discuss major features or document-format changes in an issue before undertaking a large implementation.

## AI-assisted contributions

AI-assisted contributions are welcome and follow the same review and quality standards as any other contribution. The person submitting the pull request remains responsible for the entire change.

- **Understand and review the code.** Read the generated diff, verify its behavior, and be prepared to explain the implementation. Do not submit unreviewed output or unrelated generated refactors.
- **Be transparent.** Briefly note substantial AI assistance in the pull request and describe what you reviewed and tested. Sharing private prompts, chat transcripts, or account details is not required.
- **Protect private data.** Use fictional calculation tapes. Never include credentials, personal documents, session backups, or private screenshots in prompts, fixtures, commits, or issues.
- **Verify claims.** Run the applicable checks above and report the commands and results. Clearly distinguish checks that passed from checks you could not run; generated tests and platform claims are not evidence until verified.
- **Respect ownership and licenses.** Verify the provenance and licensing of introduced code and assets. Do not copy proprietary implementations or remove required license notices.
- **Keep the scope focused.** Read the existing implementation and tests first. Preserve unrelated work, explain new dependencies, and discuss major behavior or format changes before implementing them.

For AI-assisted pull requests, include a short summary of the assistance used, the human review performed, validation results, and any remaining limitations. The contributor should submit and stand behind the final change.

## Coding conventions

### Rust style and structure

- Use the stable Rust toolchain and edition declared in `Cargo.toml`. Follow `cargo fmt` rather than introducing a separate formatting style, and keep Clippy free of warnings.
- Use `snake_case` for modules, functions, and variables; `UpperCamelCase` for types and enum variants; and `SCREAMING_SNAKE_CASE` for constants. Choose descriptive names, particularly for calculation state and editor transitions.
- Keep changes in the appropriate module: decimal operations in `math.rs`, tape evaluation in `engine.rs`, editing in `editor.rs`, persistence and exports in `storage.rs`, shared visual styling in `appearance.rs`, and application wiring in `main.rs`.
- Prefer small, explicit functions and existing project abstractions. Add comments to explain non-obvious rules and invariants, not to narrate individual statements.
- Propagate recoverable failures with `Result` and actionable messages. Do not introduce `unwrap()` or `expect()` on user input, documents, or fallible runtime I/O; assertions in tests are appropriate.
- Avoid new dependencies or `unsafe` code unless there is a clear need. Explain the tradeoff and document safety invariants when applicable.

### Calculation and document behavior

- Use the existing decimal `Number` type and arithmetic helpers for calculator values. Do not introduce binary floating-point shortcuts into decimal calculations. Keep display rounding separate from internal precision.
- Preserve sequential tape evaluation and standard-precedence assignment expressions. Test both when modifying parsing or operator behavior.
- Keep typing and literal paste behavior distinct. Preserve undo/redo, caret behavior, named values, subtotals, percentage rules, and three-digit number grouping.
- Parse expressions as data; never execute tape contents as code. Preserve input-size and arithmetic bounds and report invalid input without crashing.
- Treat native document changes as compatibility changes: version the format when necessary, test supported older documents, and recalculate derived totals on load. Preserve atomic writes and local autosave recovery.
- Keep the app offline by default. Network services, telemetry, or new collection of user data require explicit project discussion, not an incidental dependency or implementation shortcut.

### Interface and tests

- Reuse the shared theme colors and existing Iced components. Check both light and dark themes, keyboard operation, and the supported minimum window size when changing layout.
- Keep calculation logic testable without a window. Add focused, deterministic regression tests near the affected module or in `src/tests.rs`, using descriptive behavior-based test names.
- For bug fixes, include the smallest input that reproduces the failure and assert the corrected result. Cover relevant errors and boundaries, not only the happy path; do not weaken existing tests to make a change pass.
- Update user-facing documentation when behavior or shortcuts change. Screenshots must show the real app with non-private demo data. Report desktop testing separately from compilation and unit-test results.

See [the architecture overview](docs/ARCHITECTURE.md) for more context. Contributions are made under the project's [MIT License](LICENSE).
