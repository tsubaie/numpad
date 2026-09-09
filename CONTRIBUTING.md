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

See [the architecture overview](docs/ARCHITECTURE.md) for module boundaries. Contributions are made under the project's [MIT License](LICENSE).
