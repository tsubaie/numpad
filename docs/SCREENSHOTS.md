# Reproducing README screenshots

The README uses unedited captures of the real Iced application in an isolated
Linux/X11 display. No personal session, desktop capture, or generated mockup is
used. The fictional data is in [demo-workspace.json](demo-workspace.json).

Install ImageMagick (including its X11 `import` command) and the build and UI test dependencies listed in the [README](../README.md),
then run from the repository root:

```sh
xvfb-run -a -s '-screen 0 1280x1024x24' env NUMPAD_DOC_SCREENSHOTS=1 \
  cargo test --locked --features e2e --test e2e
```

This is a **capture-only mode**, not a successful regression-test run. It opens
the demo workspace, checks the total, resizes the real window to 1180 × 840,
and navigates through Dark, Settings, About, Light, and the guide. It writes
five PNG files to the printed `test-results/e2e/<run>/` directory. Font rendering
can vary with the installed system fonts.

Inspect each capture for readable content, accurate arithmetic, and absence of
personal data before copying the five `numpad-*.png` files to `docs/images/`.
Keep the main dark/light images visible in the README and the supporting images
inside the expandable gallery. Do not overwrite the app icon.

Run the same command **without** `NUMPAD_DOC_SCREENSHOTS=1` to execute the full UI
regression suite. Normal release builds must not enable the `e2e` feature.
