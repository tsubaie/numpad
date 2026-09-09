# CalcTape Web: observed behavior and compatibility specification

Reference: <https://calctape.app/en> and [the public manual](https://calctape.app/en/manual/).
Inspection date: 2026-09-09. Environment: English, decimal point, Ledger Green; both narrow and 1280 × 900 desktop layouts inspected. This is an independently written behavioral specification, not source code from CalcTape.

## Evidence and boundaries

**Observed** means exercised through the running application's editor or controls. **Documented** means described in its public manual. **Unresolved** means neither observation nor the manual establishes the exact rule. A displayed result alone cannot establish the engine's internal precision or binary file representation. NumPad's implementation choices and any omissions are recorded separately in `PARITY.md`.

The website is a tape calculator and a text editor sharing the same surface. Its numeric engine, its editor's keystroke transformations, and its formatted output are different layers. Pasting a string is not necessarily equivalent to typing its characters. Browser automation produced unreliable results for the literal equals key in some probes; Enter and the on-screen equals control are the reliable evidence for closing a calculation. Those instrumentation anomalies must not be copied as intended behavior.

## 1. Document and line model

A document is an ordered sequence of lines. Lines may be blank, explanatory text, a variable assignment, a signed operand with an optional comment, a separator, or a computed subtotal. Several independent calculations can coexist in one document. A subtotal can also feed the next operation without starting an independent calculation.

The displayed tape retains every operand. Editing an earlier operand recomputes dependent totals. A subtotal is derived data: it must not silently become an independent literal just because it is displayed as a number. Separator and total are visually separate rows. The editor inserts them together when a calculation is closed.

An empty line between calculations resets arithmetic state. A standalone text line also interrupts a sequence: the observed input `+10`, `A note`, `+20` did not add to 30. Inline comments attached to numeric lines do not interrupt the calculation. Variables remain available to later independent calculations.

## 2. Lexical grammar

The following EBNF describes semantic line forms, not every transient state accepted while the user is still typing. Spaces may surround operators and assignments.

```ebnf
document       = { line, newline }, [ line ];
line           = blank | text | assignment | operand_line | separator | total_line;
assignment     = identifier, ws, "=", ws, expression, [ comment ];
operand_line   = [ operator, ws ], operand, [ "%" ], [ comment ];
operand        = signed_number | identifier;
operator       = "+" | "-" | "*" | "/" | "^";
identifier     = letter, { letter | digit };
signed_number  = [ "-" ], number;
separator      = horizontal_rule;
total_line     = derived_signed_number, [ ws, "=", ws, identifier ], [ comment ];
expression     = expression_atom, { binary_operator, expression_atom };
expression_atom = signed_number | identifier | "(", expression, ")"
                | function_name, "(", expression, ")";
```

The expression production requires precedence; it is not an instruction to evaluate all binary operators identically. ASCII letters and digits are verified for identifiers; exact acceptance of every Unicode letter remains unresolved. Names cannot begin with a digit or contain punctuation such as underscore. Names are case-insensitive: `Tax = 15` followed by `+tax%` used Tax and the reference normalized the displayed name to its declared spelling. Duplicate names produce an error. Using an as-yet undefined name produces an error; forward declarations are not established as supported.

English numbers use a decimal point and comma grouping. Decimal-comma locales invert these conventions. Indian grouping changes presentation to groups such as `12,34,567.89`. Displayed numeric operands and totals normally have two fractional digits and align on the decimal column. Active text can remain temporarily unformatted while typing. Formatting commits when moving to the next arithmetic entry or closing a block.

After the first operand on a numeric tape line, the remaining text is a comment. This includes characters that could otherwise look like arithmetic when pasted: pasting `+10+2*3` and `+1` produced 11, with `+2*3` treated as the first line's comment. In contrast, physically typing `10+2*3` generated three arithmetic lines and produced 16. This distinction is central to compatibility.

## 3. Arithmetic semantics

The first arithmetic operand establishes the starting signed value. Multiplication, division or power cannot start a fresh calculation without an initial number/sign. Addition/subtraction have the lowest precedence, multiplication/division are next, and exponentiation is highest. Precedence applies across tape lines. Repeated powers are left-associative: the reference gave 64 for `2^3^2`.

Verified examples, where each operator after the first value is typed and creates the next line:

| Input | Result | Rule |
|---|---:|---|
| `10+2*3` | 16.00 | Multiplication before addition |
| `10+2`, Enter, `*3`, Enter | 36.00 | Subtotal becomes the new starting value |
| `9^0.5` | 3.00 | Fractional exponent / square root |
| `2^3^2` | 64.00 | Powers associate left-to-right |
| `10*-2` | -20.00 | Negative operand after multiplication |
| `1000+19%` | 1,190.00 | Add percentage |
| `910.50-40%` | 546.30 | Deduct percentage |
| `100+10%+10%` | 121.00 | Each percentage uses the updated running subtotal |
| `119/1.19` | 100.00 | Extract included tax |
| `100*10%` | 10.00 | Percentage multiplication uses factor 0.1 |
| `100/10%` | 1,000.00 | Percentage division uses divisor 0.1 |

Numbers should not be rounded to their displayed two decimals before subsequent arithmetic. A typed `1.2345*10000` retained `1.2345` visibly in the operand row and produced 12,345.00: display precision is a minimum for literal operands, not permission to destroy their extra digits. The manual states that `1 / 3 * 3` displays 1.00 and permits up to 32 digits before the decimal separator. Exact internal precision, rounding at ties, and behavior at every overflow boundary remain unresolved. NumPad uses explicitly bounded arbitrary-precision decimal calculations, never binary floating point for ordinary arithmetic.

### Percentages

For addition/subtraction, the percentage amount is the running subtotal times the percentage divided by 100. It is shown beside the operand after a vertical bar, with a negative sign for deductions. For multiplication/division, the annotation shows the effective factor/divisor, e.g. `*0.10` or `/0.10`, rather than a monetary amount.

A percentage cannot be the first operand. An immediately following operator cannot bind more tightly than the preceding percentage operator. The reference marked `100 +10% *2` as an error and displayed an invalid zero total when closed. Inserting a subtotal before `*2` resolves it and yields 220.00. The `%` sign applies to a simple operand; percentages on arbitrary expressions have additional restrictions.

### Assignments and functions

Assignments support arithmetic and parentheses even though ordinary operand-line trailing text is a comment. Observed: `rate = 10+2`, `+rate`, `+1` totals 13; `rate = (10+2)*3`, `+rate`, `+1` totals 37.

Verified single-argument functions in assignments: `sqrt`, `abs`, `round`, `sin`, `cos`, `tan`, `ln`, `log`, `exp`. Observed values include sqrt(9)=3, abs(-5)=5, round(1.6)=1.60, sin(0)=0, cos(0)=1, tan(0)=0, ln(1)=0, log(100)=2, exp(1)=2.72. A later nonzero probe, `a=sin(90)` followed by `+a` and `+0`, displayed 0.89, confirming radians rather than degrees. At the default two decimal places, `a=round(1.2345)` followed by `+a` and `*10000` displayed 12,300.00: round explicitly quantizes to two places before subsequent arithmetic. Tie behavior and every alternative precision setting remain unverified. Other function names, constants, and multiple-argument syntax are not established by these tests. `floor`, `ceil`, `min(2,3)` and `max(2,3)` did not produce valid calculations in the probes.

A computed subtotal may be named by appending `= budget` directly to that subtotal. Observed: name a 120 total, insert a blank line, type `+BUDGET-25%`, Enter: result 90, grand total 210. Pasting a standalone `=budget` is not equivalent to editing a generated sum line.

## 4. Keystroke and editor contract

| Action/context | Required behavior / evidence |
|---|---|
| Digits on an empty row | Enter a value; display may be unformatted until committed |
| `+ - * / ^` after a numeric row | Commit/align the existing row and start an operator row |
| Operator after inline comment | Starts the next operation, preserving the comment |
| Operator in standalone prose | Remains ordinary text; observed `Hello world+test` |
| `%` after numeric value | Mark percentage; update result immediately |
| Enter on a multi-operand calculation | Commit last value; insert rule and computed subtotal; move caret to following empty row |
| Enter on only one value | Observed: simply moves to a new row, without adding a subtotal |
| Enter on prose/assignment | New text row; assignment stays intact |
| Enter on empty row after a sum | Leaves an empty line, allowing the next calculation to be independent |
| Operator on the row directly after a sum | Continues from that sum |
| Shift+Enter | Observed on a multi-operand calculation: also closes it; it is not a reliable raw-newline escape |
| Mouse click / arrow movement | Edit anywhere; active line and caret position update |
| Backspace / Delete | Standard character and selection deletion |
| Selection + typing/paste | Replace selected text; keep surrounding content |
| Undo / Redo | Restore both source edits and generated formatting as logical edit steps |
| AC | Empty the current document; undo restores it during the current session |
| Paste | Insert text using line parsing, not simulated key-by-key operator splitting |
| Editing computed numeric portion | Must preserve its derived relationship; exact reference boundary navigation unresolved |

Enter closes the calculation containing the caret, rather than implicitly requiring the document's last row. Cursor/selection must survive numeric formatting and dependent-total recalculation. Number keys on the on-screen keypad insert at the editor caret even when the button is clicked. Keypad clicks should not discard the editor selection. The reference supports number-row and numeric-keypad input.

Documented global shortcuts: Ctrl/Cmd+S save; Ctrl/Cmd+Z undo; Ctrl/Cmd+Y redo; Ctrl/Cmd+Shift+L toggle ruled paper; Ctrl/Cmd+Shift+plus/minus zoom; Ctrl/Cmd+Shift+0 reset zoom. Dialog text fields receive their own keystrokes instead of triggering document shortcuts.

## 5. Results, memory, copying

The large result is the result of the calculation containing the caret, not necessarily the last calculation in the document. Grand total sums completed independent calculations; intermediate subtotal chains should contribute their final completed value once. An unclosed calculation was observed to show a live result but contribute zero to grand total.

Memory is distinct from the grand total. M+ and M− operate on the current numeric operand or selected sum, **not automatically the large block result**: clicking the first operand of a 100+20=120 calculation changed the memory preview to +100; clicking the subtotal changed it to +120. MR inserts the stored number; MC clears it. The numeric result, grand total, memory and subtotal copy affordances copy their respective values. The whole-block copy includes formatted operands, comments and computed totals.

The manual describes a transient block highlight when selecting a total, a copy button for the highlighted block, and a long-press subtotal menu offering copy and M+/M−. Exact highlight expiration is documented as ten seconds, but was not timed in this inspection.

## 6. Visual specification

Desktop: a full-width dark header; a flexible tape card on the left; result card, keypad card, and product-links card in a fixed-width right column. Footer sits below. The tape scrolls internally. The desktop probe at 1280 × 900 had an approximately 330-pixel sidebar and 16-pixel gaps/margins.

The header contains main menu, brand, palette selector, light/dark toggle, language selector and Share. The tape toolbar contains undo, redo, ruled-paper toggle, proportional-font toggle, Indian-grouping toggle, zoom minus/reset/plus and help. The status strip has zoom, line/column, current-line error and autosave status.

Ledger Green uses a dark forest header, pale green page surround, an almost-white warm green tape, fine horizontal rules, dark green sums, and muted red negative values. Arithmetic sections have a narrow pale strip along the left; the active row has a light accent background and a darker vertical marker. Sums are bold, and separators span the numeric portion rather than the entire paper width. Comments sit after the aligned numeric column. The display supports proportional or monospaced type.

Desktop keypad, row order:

```text
 AC       Backspace   %      ÷
 7        8           9      ×
 4        5           6      −
 1        2           3      +
 0        00          .      = / Enter
 M+       M−          MR     MC
 [custom key 1]       [custom key 2]
```

Narrow layout moves a shorter, landscape keypad into a collapsible bottom sheet with a mini result. Memory keys are omitted there. Short windows compress keypad geometry. Exact CSS breakpoints are not exposed by the manual; both representative layouts were inspected.

Seven documented palettes: Classic Copper, Vivid Indigo, Subtle Slate, Ledger Green, Sunlit Amber, High Contrast, High Contrast Warm. Each combines with light/dark mode. High-contrast variants add stronger boundaries and stronger key differentiation. Locale-dependent first-run palette and tax defaults are documented. Explicit choices persist.

## 7. Custom keys

Two tax keys are configurable. A gear affordance opens a dialog with label (maximum 16 characters), formula/text, and an automatic-calculate toggle. Literal `\n` sequences expand into actual line breaks. With automatic calculation enabled, insertion closes the calculation. Disabled means keep editing after insertion. Default repopulates factory values; Cancel discards pending changes; OK stores changes and updates the button. Tax extraction uses division by (1 + rate/100), not subtracting the rate from gross. Exact regional default table is not public in the manual.

## 8. Persistence and sharing

The browser automatically persists its tape and view/key preferences locally and restores them on reload. Browser-data removal and private browsing affect durability. Undo history is session-scoped and bounded.

Share offers native CalcTape `.calc` download, copy formatted text, copy link, QR, Excel and PDF exports, and native system sharing on supported browsers. A share link carries compressed document data in its URL fragment; the manual describes checksum validation, a replacement confirmation when opening over existing content, and cleanup of the fragment after import. It does not establish the compression algorithm, binary schema, checksum algorithm or backward-compatibility contract. These formats must not be guessed and advertised as interoperable. Web CalcTape does not reopen its own downloaded `.calc` files according to the manual; its native apps can.

PDF/Excel exports contain calculated results and comments. Exact page layout, spreadsheet formulas versus static values, sheet naming, fonts, metadata, and large-document pagination require export-specific verification.

## 9. Errors and limits

Errors have a line-level red treatment and a plain-language message in the current-line status area. Documented categories: zero divisor; adjacent/unexpected operators; malformed syntax; invalid first operator; percentage requiring an intermediate total; percentage with non-simple input; percentage at calculation start; unknown function; invalid identifier; duplicate identifier; undefined variable/value; naming a sum outside a sum row. A dependent sum may show an invalid-calculation error too. Do not substitute a valid-looking silently stale result for an erroneous dependent calculation.

Documented limit: 500 lines. Around 95% capacity a warning appears. At capacity new rows are rejected but edits/deletion/undo remain possible. Oversized pastes require truncation confirmation, and older oversized stored documents load without silent truncation. Exact byte/character limits and undo memory budget are unresolved.

## 10. Secondary product behavior

The public UI includes seven languages (English, German, Spanish, French, Italian, Portuguese, Hindi), five interactive tutorials (basics, variables, percentages, totals, comments), news, manual and About/license links. Tutorial entry preserves and later restores the user's tape. Repeated-value detection can offer converting three identical values to a named variable; two declines suppress further prompts. These are documented and not fully interaction-tested. Product/legal/store links are specific to CalcTape and are not NumPad functionality.

## 11. Verification requirements for NumPad

Automated tests must cover the arithmetic table, assignment expressions, case-insensitive variables, independent blocks, subtotal chains, percentages and percentage errors, source corrections, precision, locale parsing, and invalid dependency propagation. Editor tests must cover typed-versus-pasted operators, Enter, generated sums, caret preservation, selection, clear/undo/redo, memory operand selection and file round trips. Native smoke testing must exercise typing and reopening the application. Every platform build claim must be backed by an actual build; configuration alone is not platform validation.

This specification covers all public manual sections and the inspected UI, while deliberately labeling what is documented, observed or unresolved. It is not a claim that every possible hidden engine behavior has been discovered.
