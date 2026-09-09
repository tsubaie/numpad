# NumPad design

The interface deliberately has its own identity instead of copying the reference website.

## Version 1.3 clarification and alignment

The working font toolbar control shows its current mode and has a hover explanation. The Indian-grouping feature and its toolbar control are removed; grouping always uses groups of three. The Copy block control is removed. The application is English-only; incomplete language selection is removed while old files remain readable.

At normal desktop heights, the keypad card expands to share the tape card's bottom edge, with flexible space above the tax controls. Short windows retain a scrollable sidebar so controls remain reachable. Tax buttons derive their displayed percentage directly from the validated saved setting (15% by default, customizable), and the adjacent link is explicitly labeled Tax settings.

## Version 1.2 simplification

Tape rows execute top-to-bottom: 10, +2, ×3 produces 36. Assignment expressions retain standard precedence. Existing tapes recalculate using this new rule; new documents carry format version 2.

The header contains only branding and the hamburger menu. Export is a submenu; appearance and language live in Settings. The sidebar help card is removed. Undo, redo and backspace use consistent outlined vector icons.

Tax buttons show only the direction and tax name, such as + VAT / − VAT. A single tax dialog controls both directions, with one validated rate and optional automatic calculation. Remove-tax divides by 1 + rate/100. Old custom-key preferences migrate from the add-tax formula; the rate is removed from the label and both directions are synchronized.

## Appearance

There are exactly two appearances, selected by one light/dark toggle:

- Light: white paper, cool neutral surfaces and a teal accent echoing the supplied logo.
- Dark: [Catppuccin Mocha](https://catppuccin.com/palette/), using its base/mantle/surface colors, pastel mauve accent, readable subtext and rose errors.

There is no palette selector. Legacy documents containing a `palette` preference still open; that obsolete value is ignored while the existing dark/light choice is retained.

## Logo

`Logo.png` is the original user-provided artwork, preserved unchanged. Its wordmark reads NumNote; the application name remains NumPad. `build.rs` extracts the calculator mark for small icon surfaces and generates a PNG and multi-resolution Windows ICO in Cargo's build directory. The logo is embedded in the app header, native window icon and Windows executable resource. No image file needs to sit next to the executable at runtime.

## Keypad

All number, operator and memory keys use a zero-padding button with a content container centered on both axes. The label is therefore centered in the actual button bounds, not just its top text line. Custom-key labels and their settings buttons use matching 40px heights. The equals key displays a single `=`; physical Enter and equals behavior is unchanged.

The header, quiet neutral keypad surfaces, compact stacked brand and softer color hierarchy are NumPad-specific. Notes and numbers still use the same readable tape and live recalculation model.
