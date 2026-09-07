## OpenCalc v1.1.10 buildfix158

Changes
- Reorganized Settings... > Keyboard Shortcuts into two clear sections: **Shortcut assignment** and **Shortcut presets**. The Windows sections use the existing classic sunken-border painter; Linux uses native GTK frames.
- Replaced the preset-name textbox with an editable combo box. Existing `shortcuts/*.cfg` presets populate the drop-down, while users can still type a new preset name and save it as a plain-text preset.
- Removed the redundant "Available presets" text list; Refresh now rebuilds the combo-box choices while preserving the current typed/selected preset where possible.
- Added a confirmation prompt before **Delete Preset** actually removes a preset. The Windows confirmation is owned by the Keyboard Shortcuts dialog; Linux uses a modal GTK alert with Cancel as the default action.
- Split shortcut-recording status from preset-management status so preset messages no longer appear inside the shortcut-assignment workflow.
- Preserved Ctrl+A and Backspace editing for the new editable Windows preset combo box by applying the existing native EDIT-control helper to the combo box's child edit control.
- Kept preset files, `shortcuts/default.cfg`, Restore Defaults behavior, search, shortcut recording, and text-editor compatibility unchanged.

Validation
- PASS: Rust lexical delimiter scan accepted all source files.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native wxDragon/GTK compilation and live dialog testing could not be rerun here.

## OpenCalc v1.1.10 buildfix157

Changes
- Fixed normal text editing inside the Windows Keyboard Shortcuts dialog. Ctrl+A now selects all text and Backspace deletes the selected text (or the previous character) in the Search shortcuts, Shortcut keys, and Preset name textboxes.
- The fix is implemented at the native EDIT-control layer so the dialog's calculator/shortcut key routing cannot steal these editing commands before the textbox processes them.
- Backspace deletion is selection-aware and preserves UTF-16 surrogate pairs when deleting the previous character.
- Linux GTK entries already provide native Ctrl+A and Backspace editing, so no Linux behavior change was necessary.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- PASS: Python helper syntax and Bash helper syntax passed.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native wxDragon compilation and live dialog testing could not be rerun here.

## OpenCalc v1.1.10 buildfix156

Changes
- Fixed a shortcut-search crash caused by synchronous GUI event re-entry. While the search handler was updating the selected shortcut and key textbox, it could keep an immutable `RefCell` borrow of the shortcut draft alive; wxDragon then synchronously emitted the textbox-change callback, which tried to borrow the same draft mutably and panicked. Typing a longer search string made this easy to trigger.
- The Windows shortcut search now resolves/clones the matched draft value before calling any widget setter, so selection/text callbacks can safely re-enter the shortcut state.
- Applied the equivalent lifetime fix to the Linux GTK search path, where `DropDown::set_selected` may synchronously emit `selected-notify`.
- Fixed the same latent re-entry hazard when loading a shortcut preset on both Windows and Linux by cloning the selected shortcut text before updating the editor widget.
- No shortcut matching, preset format, calculator behavior, or default bindings were otherwise changed.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- PASS: no event-time `entry.set_value(&draft.borrow()[...])` / `entry.set_text(&draft.borrow()[...])` pattern remains in the affected shortcut callbacks.
- PASS: ZIP integrity and source-manifest verification succeeded after repackaging.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and live GUI testing could not be rerun here.

## OpenCalc v1.1.10 buildfix155

Changes
- Fixed the Windows shortcut-search compile error introduced in buildfix154. `wxDragon::Choice::set_selection` expects a `u32`, so the matched shortcut index is now passed with the correct type.
- No shortcut preset, search, calculator, Linux, or other behavior was changed.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- PASS: ZIP integrity and source-manifest verification succeeded after repackaging.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and `cargo test` could not be rerun here.

## OpenCalc v1.1.10 buildfix154

Changes
- Added a shortcut search box to Settings... > Keyboard Shortcuts. It searches action names, internal shortcut IDs, and the current key assignments, then selects the first matching action.
- Added a plain-text shortcut preset manager to the Windows and Linux shortcut dialogs. Presets are stored as `shortcuts/*.cfg` beside the executable and can be loaded into the current draft, saved/overwritten, deleted, and refreshed without changing active bindings until the main **Save** button is pressed.
- Added the shipped `shortcuts/default.cfg` preset containing the complete default shortcut map. Fresh installations use this file when it is valid, so advanced users can edit it directly with a text editor.
- Kept **Restore Defaults** independent from `default.cfg`: it reconstructs the pristine compiled-in default table, so an edited, malformed, or deleted `default.cfg` cannot break recovery.
- Preset files use the existing human-readable `shortcut.<action>=...` format and support comments, disabled actions, manual editing, validation, and duplicate-key rejection.
- Updated Windows and Linux build packaging so `shortcuts/*.cfg` is included with runtime output; Linux rebuild/clean paths preserve user-created runtime presets.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- Added shortcut preset round-trip/reset/search regression coverage.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and `cargo test` could not be rerun here.

## OpenCalc v1.1.10 buildfix153

Changes
- Implemented the Win95 hidden entry-sign state for `+/-` on zero/new operands. The display may remain `0.` (or the previous left operand immediately after selecting a binary operator), while the next digit is entered with the stored negative sign. Example: `2 + +/- 3 =` now evaluates as `2 + (-3)` and returns `-1`. The same state is honored in Hex/Oct/Bin entry; Exp keeps its separate exponent-sign behavior.
- Fixed Scientific `=` when a binary operator has no newly typed RHS. The displayed left operand is reused as the missing RHS (`2 + =` -> `4`, then repeated `=` -> `6`; `5 * =` -> `25`). The same saved operation remains available for subsequent repeated equals.
- Canonicalized Scientific operands at the moment they enter the expression so later radix changes cannot reinterpret older operands. Decimal `10 +`, then switching to Hex and entering `5`, now produces Hex `F`; Hex `10 +`, then switching to Decimal and entering `5`, produces Decimal `21`. Paste remains intentionally radix-aware and transactional.
- Added classic invalid-input feedback for Backspace. Backspace is accepted only while a direct entry is actually editable; otherwise both Windows and Linux front ends use the existing invalid-input beep path instead of silently doing nothing.
- Preserved OpenCalc's intentional improvements: unlimited valid parenthesis nesting, unrestricted Decimal direct-entry length, transactional/parser-based Paste, separate `floor()` semantics, and keyboard `**` exponentiation.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- Added regression coverage for hidden sign-before-entry, Scientific missing-RHS/repeated-equals behavior, mixed-radix Scientific expressions, and Backspace editability.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and `cargo test` could not be rerun here.

## OpenCalc v1.1.10 buildfix152

Changes
- Corrected buildfix151's DMS normalization to match the recovered Win95 routine exactly: the `0.99999999999` threshold is applied only to the fractional remainder from the second split. If that remainder exceeds the threshold, only that split's integer component is incremented and the remainder is zeroed. OpenCalc no longer adds an extra seconds-rounding stage or an explicit 60-minutes-to-degree carry that CALC.EXE does not perform.
- Replaced buildfix150's whole-expression parenthesis evaluation with a dynamically sized Scientific parenthesis-frame stack. Closing `)` now evaluates only the innermost group, materializes that group result on the display, and restores the suspended outer expression. Example: `2*(3+4)` shows `7` when `)` is pressed while retaining the outer `2*`; `=` then gives `14`. Nested frames such as `2*(3+(4*5))` resolve as `20`, then `23`, then `46`. OpenCalc intentionally retains unlimited valid nesting rather than the original 25-level cap.
- Fixed F2-F8 selector shortcuts while Standard mode is active. They no longer mutate hidden Scientific angle, radix, or word-size state; Standard remains Decimal. A defensive Calculator-level invariant also rejects non-Decimal base changes while Standard is active.
- Fixed the enhanced expression parser's `int()` function to truncate toward zero, matching the classic Int button (`int(-1.5) = -1`). The separate `floor()` function remains available and still returns `-2` for `floor(-1.5)`.
- Preserved OpenCalc's intentional improvements: unlimited valid parentheses, unrestricted Decimal direct-entry length, transactional/parser-based Paste, and keyboard `**` exponentiation.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- PASS: parser/static regression coverage added for DMS split semantics, nested parenthesis frames, Standard-mode Decimal invariance, and `int()` versus `floor()`.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and `cargo test` could not be rerun here.

## OpenCalc v1.1.10 buildfix151

Changes
- Fixed DMS carry normalization around `59.999...` minute/second boundaries. OpenCalc now treats a fractional minute or second component above approximately `0.99999999999` as the next whole unit before packing or unpacking DMS values, matching the Win95 calculator's recovered behavior.
- This removes floating-point artifacts in edge cases such as values effectively equal to `10° 59' 59.999999999999"`, which now normalize cleanly to `11° 00' 00"` instead of displaying packed DMS residue.
- Applied the same normalization to inverse DMS conversion (`Inv + DMS`) so packed `D.MMSS` inputs near `59.999...` also carry correctly.
- Preserved the deliberate OpenCalc improvements, including unlimited valid parenthesis nesting, unrestricted Decimal direct-entry length, the transactional/parser-based Paste behavior, and `**` support.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- PASS: ZIP integrity and source-manifest verification succeeded after repackaging.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and `cargo test` could not be rerun here.

## OpenCalc v1.1.10 buildfix150

Changes
- Fixed Standard percent semantics for multiplication and division so `%` uses the saved left operand there as well. Example: `200 × 10 %` now displays `20`, and `200 ÷ 10 %` displays `20`, matching the Win95 calculator's percentage routine.
- Fixed Scientific `)` handling so closing a parenthesized group materializes that group's result immediately as the current operand. This restores behaviors such as `(2+3)` showing `5` right away and `(2+3) x²` operating on `5` rather than a stale display value.
- Fixed repeated `=` in Scientific mode for normal binary calculations. After `2 + 3 =`, another `=` now continues the saved operation (`8`, then `11`, etc.) instead of becoming a no-op.
- Preserved the deliberate OpenCalc improvements: unlimited valid parenthesis nesting, unrestricted Decimal direct-entry length, the transactional/parser-based Paste behavior, and `**` support.

Validation
- PASS: Rust lexical delimiter scan accepted the updated source tree.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- PASS: ZIP integrity and source-manifest verification succeeded after repackaging.
- A Rust/Cargo toolchain is not installed in the packaging environment, so native compilation and `cargo test` could not be rerun here.

# HISTORY

This file consolidates the OpenCalc v1.1.10 build-fix notes that were previously stored as separate `BUILDFIX*.txt` files. The historical content is preserved here in one place; the standalone build-fix note files have been removed.

## OpenCalc v1.1.10 buildfix149

Win95 Scientific clear-state, exponent-format and trig-zero corrections

This build addresses only the first three confirmed compatibility gaps found after buildfix148. OpenCalc's intentional improvements remain unchanged.

1. C / CE Scientific modifier cleanup
- C now clears Inv and Hyp and also returns F-E to normal display mode.
- CE clears Inv and Hyp but deliberately preserves the current F-E selection.
- The distinction is implemented in Calculator state, so both Windows and Linux front ends refresh the selector checkboxes consistently.

2. Three-column scientific exponents
- Automatic scientific notation and F-E now use the classic signed three-digit exponent field (`e+000` / `e-000`).
- Examples: `1.e+013`, `1.e-016`, and forced F-E `1.25e+001`.
- This changes presentation only; OpenCalc retains its existing 13-significant-digit display model and unrestricted internal f64 arithmetic.

3. Classic trigonometric zero handling
- sin and cos now clamp post-FPU-style residue with the recovered `1e-15` threshold, so canonical values such as sin(180 degrees) and cos(90 degrees) are exactly zero.
- tan preserves the original distinction: it does not get a general tiny-value clamp, but the exact degree/radian multiples explicitly special-cased by CALC.EXE are returned as zero before tangent evaluation.
- The existing tangent asymptote guard (`|tan| > 1e15`) remains unchanged.

Intentional OpenCalc improvements retained
- Unlimited valid parenthesis nesting.
- Dynamically sized direct Decimal entry.
- Transactional real expression Paste parser.
- Keyboard `**` exponentiation support.

Regression coverage added
- C versus CE Inv/Hyp/F-E state cleanup.
- Three-column positive and negative scientific exponents.
- Exact-zero sin/cos/tan degree/radian cases plus the recovered sin/cos zero clamp in Grads.

Native Cargo/wxDragon compilation is not available in this container because there is no Rust/native GUI toolchain. Static source checks, helper-script checks, manifest verification, patch dry-run and ZIP integrity are performed before packaging.

## OpenCalc v1.1.10 buildfix148

### Documentation consolidation

- Consolidated the complete buildfix136 through buildfix147 development notes into this `HISTORY.md`.
- Removed the redundant standalone `BUILDFIX136.txt` through `BUILDFIX147.txt` files.
- Added a direct history reference from `README.md`.
- No calculator, UI, platform, parser, Help, shortcut, or arithmetic behavior changed in buildfix148.

## OpenCalc v1.1.10 buildfix147

Win95 unary/memory/display-state completion while retaining OpenCalc extensions

This build fixes the compatibility discrepancies identified after buildfix146
without reintroducing obsolete capacity limits or removing OpenCalc's safer
expression features.

1. Memory commands end direct numeric entry
- MC, MR, MS and M+ now end the current direct-entry phase. A following digit
  starts a fresh number instead of appending to the value that was on screen.
- MR still remains a real current operand, including inside a Scientific
  expression; this is represented separately from the direct-entry state so
  expression evaluation is not broken merely to reproduce the next-digit rule.
- Active Exp input is materialized before the memory action ends entry, so later
  digits cannot continue editing a stale exponent field.
- MS of zero leaves the memory indicator off. M+ also clears the indicator when
  the resulting memory value is zero.

2. Inv/Hyp one-shot behavior
- Inv is consumed after a successful function for which Inv changes meaning:
  Int, dms, sin/cos/tan, ln/log, x^2 and x^3.
- Hyp is consumed after a successful sin/cos/tan operation.
- If the function reports a domain/overflow error, the selectors remain set,
  matching the original dispatcher's early error exit.
- The Windows and Linux checkboxes refresh directly from Calculator state, so
  the one-shot clearing is immediately visible in both front ends.

3. Correct Int semantics
- Normal Int truncates toward zero: Int(-1.5) = -1.
- Inv+Int returns the signed fractional remainder: Inv+Int(-1.5) = -0.5.
- Inv is consumed only after a successful Int operation.

4. Inverse square and cube
- Inv+x^2 performs square root.
- Inv+x^3 performs real cube root, including negative operands.
- The existing direct sqrt function remains available where the Standard layout
  uses it.

5. Win95-style decimal result formatting
- Completed Decimal results and F-E display use a 13-significant-digit decimal
  record, matching the original common display path.
- General display uses fixed notation while the recovered exponent/width test
  permits it; otherwise it switches to scientific notation.
- F-E forces the same 13-significant-digit scientific representation.
- This is presentation only: OpenCalc still keeps f64 arithmetic internally and
  still permits direct Decimal entry longer than the original 13-digit buffer.

Intentional OpenCalc improvements retained
- Unlimited valid parenthesis nesting; no Win95 25-level ceiling.
- Dynamically sized direct Decimal entry; no Win95 13-digit entry ceiling.
- The real expression Paste parser, including context-correct unary signs.
- Keyboard ** exponentiation in addition to the recovered classic shortcuts.

Regression coverage added
- All four memory commands start a fresh following numeric entry; MR remains a
  valid Scientific operand; zero does not light the M indicator.
- Inv/Hyp successful one-shot clearing and error-state preservation.
- Int(-1.5), Inv+Int(-1.5), Inv+x^2 and Inv+x^3.
- 13-significant-digit rounding and the recovered fixed/scientific boundaries,
  plus forced F-E scientific formatting.

Native Cargo/wxDragon compilation is not available in this container because
there is no Rust/native GUI toolchain. Static source checks, helper-script
checks, manifest verification, patch dry-run and ZIP integrity are performed
before packaging.

## OpenCalc v1.1.10 buildfix146

Win95 calculation-state completion without reintroducing obsolete limits

This build fixes the remaining state-machine discrepancies identified after
buildfix145 while deliberately keeping OpenCalc improvements that are strictly
more capable than the Windows 95 Calculator.

1. Classic calculator commands remain error-locked until C or CE
- Calculator input no longer clears an error merely because a digit or decimal
  point is pressed.
- Arithmetic, unary, memory, statistics, PI/Exp, F-E and selector/modifier
  commands remain blocked while an error is displayed.
- Windows/Linux front ends issue the classic invalid-input beep for blocked
  calculator commands.
- C and CE remain the classic calculator recovery commands. Copy, Help and About
  are application UI commands and remain usable while an error is visible. Modern
  OpenCalc Undo/history recall also remain available rather than being artificially
  disabled for Win95 compatibility.

2. Standard repeated '='
- Standard mode now remembers the last completed binary operator and right-hand
  operand.
- 2 + 3 = produces 5; subsequent '=' presses produce 8, 11, 14, ...
- Starting a fresh numeric calculation or choosing a new operation cancels the
  old repeat chain.
- The optional OpenCalc History panel also records repeated-equals calculations
  using the previous result as the next left operand.

3. Standard/Scientific switching preserves the current value
- Switching modes is no longer implemented as Clear.
- Scientific Hex FF -> Standard now becomes Decimal 255 rather than 0.
- Entering Standard still forces Decimal, as recovered in buildfix145.
- Unfinished mode-specific operator/expression state is discarded cleanly while
  memory, statistics, width selection and the current numeric value survive.

4. Statistics companion closes on Standard
- Switching to Standard closes an open Statistics companion window on both
  Windows and Linux, matching the original layout transition.
- The statistics dataset itself remains stored in Calculator state.

5. Scientific consecutive operators replace the pending operator
- A newly selected binary operator replaces the previous pending operator rather
  than constructing malformed expressions such as 2+*3.
- Textual operators (Mod/And/Or/Xor/Lsh/Rsh/root) and symbolic operators share
  the same replacement path.
- The existing OpenCalc '**' keyboard exponentiation extension is retained.

6. Parenthesis error feedback, without the Win95 nesting ceiling
- An unmatched ')' is rejected through the same invalid-input beep path.
- OpenCalc intentionally does NOT restore CALC.EXE's historical 25-level nesting
  limit. Parentheses remain dynamically nestable, subject only to practical
  memory limits.

Intentional modern behavior retained
- Clipboard Paste remains OpenCalc's real expression parser rather than the
  original character-to-command macro interpreter.
- Decimal direct entry does NOT restore the Windows 95 13-digit mantissa-entry
  ceiling; OpenCalc keeps dynamically growing entry text instead.
- Parenthesis nesting remains unlimited rather than capped at 25 levels.
- The '**' power shortcut remains available in addition to the recovered keys.

Validation
- Added calculator regression tests for error locking/CE recovery, repeated '=',
  value-preserving mode switching, Scientific operator replacement, unmatched
  close-parenthesis state, 100-level parenthesis nesting, and decimal entry well
  beyond the legacy 13-digit limit.
- Rust lexical delimiter and platform-separation checks pass.
- Python helper scripts compile successfully.
- Native Cargo/wxDragon compilation remains unavailable in this container because
  the Rust/native GUI toolchain is not installed.

## OpenCalc v1.1.10 buildfix145

Windows 95 radix-state, invalid-input, PI and Exp completion

This build completes the five surrounding Scientific/radix behaviors that were
still missing after buildfix144.

1. Radix selection commits before conversion errors
- Hex/Oct/Bin is now selected before the current Decimal value is converted.
- If conversion exceeds the unsigned DWORD domain, the error is shown while the
  newly selected radix remains active. Clearing then produces zero in that radix.

2. Standard mode always uses Decimal
- Entering Standard mode explicitly restores Base::Dec before clearing the
  calculator, matching the original Standard-mode command path.

3. Invalid radix input beeps
- Digits outside the active radix are rejected with the classic system beep.
  Examples: 8 in Oct, 2 in Bin, A in Dec.
- The decimal-point command likewise beeps outside Decimal mode.
- The rejected input does not alter calculator state or create an undo snapshot.

4. PI is Decimal-only and supports Inv+PI
- PI is rejected with a beep in Hex/Oct/Bin.
- PI displays pi in Decimal mode.
- Inv+PI displays 2*pi and consumes Inv.
- Scientific expression/history state keeps the exact pi / 2*pi constant token
  while the display remains numeric.

5. Exp is exponent entry, not e^x
- The Exp button now enters Windows 95-style scientific notation instead of
  applying the exponential function.
- Exp after C starts at 1.e+000.
- Active Decimal entries and completed displayed results become forms such as
  12.e+000; only a genuinely empty clear buffer gets the implicit mantissa 1.
- The visible exponent is a rolling three-column numeric field: leading zero
  keystrokes remain valid, while its magnitude cannot grow beyond 289.
- +/- while Exp entry is active changes the exponent sign, and Backspace divides
  the exponent magnitude by ten (289 -> 028 -> 002 -> 000).
- Exp and PI descriptions were corrected in all three compiled HLP manuals.

Implementation notes
- Added explicit Action::Exp so the UI cannot confuse exponent entry with the
  internal mathematical exp operation used by Inv+ln.
- Added Calculator exponent-entry state and input-acceptance queries.
- Added platform::invalid_input_beep(): Win32 MessageBeep(MB_OK) on Windows and
  a portable terminal-bell fallback on Linux.
- Added an idempotent tools/update_help_exp_pi.py WinHelp topic repacker.
- Added regression tests for radix commit-on-error, Standard Decimal reset,
  radix digit/decimal-point validation, Decimal-only PI/Inv+PI, Exp
  entry/sign/backspace, completed-result entry and the recovered 289 exponent
  magnitude ceiling.

Reference executable observations used for this pass
- Radix is committed before conversion/display validation: 0x004050C4..0x004050CB.
- Standard mode invokes the Decimal command: 0x004016B0..0x004016BC.
- Invalid radix digits route to MessageBeep: 0x00402CD0..0x00402CE1.
- PI Decimal guard / Inv handling: 0x004035C6..0x0040361D.
- Exp dispatches exponent-entry logic rather than e^x: 0x00403758..0x0040378A,
  entering through the exponent-input routine at 0x00405254.
- The exponent digit helper at 0x0040516B..0x0040517C compares the current
  numeric exponent with 29 before multiplying by ten and adding the next digit.
  This yields a 289 magnitude ceiling for either sign while allowing unlimited
  leading-zero keystrokes in the fixed three-column display.

Native Cargo/wxDragon compilation and live Windows execution remain unavailable
in this container. Static source, platform-separation, helper-script, WinHelp,
manifest, whitespace and ZIP-integrity validation are performed before packaging.

## OpenCalc v1.1.10 buildfix144

Final selector/radix audit corrections after buildfix143

A further pass over the supplied 59,392-byte Windows 95 CALC.EXE found three
remaining implementation details that were not yet represented correctly.

1. Inv + Lsh is arithmetic right shift
- With Inv active, Lsh command 0x59 is replaced by internal operation 7 and Inv
  is cleared (0x00402DD7..0x00402DFA).
- Operation 7 executes x86 SAR (0x00404D8A..0x00404DA9), so bit 31 is sign
  extended and the shift count follows x86's count & 31 behavior.
- This is implemented as an internal Rsh operation used by Inv+Lsh.
- No new '>' keyboard shortcut is added: the supplied Win95 input table exposes
  '<' for Lsh, and right shift is obtained with Inv+Lsh.

2. Non-decimal values are integerized with floor before range/masking
- The Hex/Oct/Bin display path calls the CRT floor routine at 0x0040590F,
  stores the result back into calculator state, and only then compares abs(value)
  with 4294967295.0 at 0x0040A4B0.
- Word and Byte still hide only high display bits; the retained hidden state is
  the full unmasked integer, not an undisplayed fraction.
- Decimal -1.1 therefore becomes -2 when converted to Hex/Oct/Bin.
- A non-decimal 5 / 2 result is stored as 2, so switching back to Dec does not
  reveal 2.5.
- floor precedes the DWORD limit check, matching the executable's exact order.

3. F2/F3/F4 are the shared selector shortcuts; F6 is always Dec
- Accelerator resource entries map F2/F3/F4 to 0x7F/0x80/0x81.
- In Decimal these controls mean Deg/Rad/Grad.
- In Hex/Oct/Bin the same controls mean Dword/Word/Byte.
- F5/F6/F7/F8 map to Hex/Dec/Oct/Bin respectively.
- The previous OpenCalc behavior that used F6 for Rad while already in Decimal
  was incorrect; Rad is F3.

Implementation
- Added internal BinaryOp::Rsh and parser support for the scientific expression
  buffer, preserving signed DWORD SAR semantics.
- Corrected the Windows and Linux F3/F6 dispatch paths.
- Corrected shortcut labels, context-help text, README, reverse-engineering
  notes and Help reference-table generators.
- Added regression coverage for Inv+Lsh, sign extension, shift-count masking,
  non-decimal floor semantics and the recovered F2..F8 shortcut definitions.

Reference executable
- SHA-256: b064b0ac430264eff7b79b91e743bcd36d7b3707857f5bcdc4db146911dd0e28
- Inv+Lsh dispatch: 0x00402DD7..0x00402DFA
- SAR implementation: 0x00404D8A..0x00404DA9
- non-decimal floor/store/range path: 0x0040426C..0x004042B2
- floor helper: 0x0040590F (FRNDINT path reaches 0x00406C5B)
- DWORD magnitude constant: 0x0040A4B0 = 4294967295.0
- accelerator resource: raw file offset 0xD05C onward

Native Cargo/wxDragon compilation and a live Windows GUI run are unavailable in
this container. Static source, helper-script, source-manifest and ZIP validation
are performed before packaging.

## OpenCalc v1.1.10 buildfix143

Fix
Buildfix140/141 changed the non-decimal selector labels and added a final
Dword/Word/Byte display mask, but the calculator entry/arithmetic state still
did not fully match Windows 95 CALC.EXE. Buildfix143 implements the recovered
width semantics instead of treating the three choices as labels alone.

Recovered Windows 95 behavior
- Dword mask: 0xFFFFFFFF (low 32 bits)
- Word mask:  0x0000FFFF (low 16 bits)
- Byte mask:  0x000000FF (low 8 bits)
- These masks apply to Hex, Oct and Bin display output.
- Word/Byte hide upper bits; they do not truncate the full current value.
- Switching back to Dword therefore reveals hidden upper bits.
- The full non-decimal magnitude is checked against 0xFFFFFFFF before masking.
- Direct entry, Backspace and +/- now operate on the full hidden value.
- Negative non-decimal values display in two's-complement form at the selected
  Dword/Word/Byte width.
- And/Or/Xor/Not/Lsh now use the original 32-bit DWORD register semantics.
- Lsh uses the x86 5-bit shift-count rule (count & 31), not a 64-bit count.

Examples
- Hex / Word: FFFF * 2 -> FFFE; switch to Dword -> 1FFFE.
- Oct / Word: 177777 * 2 -> 177776; switch to Dword -> 377776.
- Bin / Word: 1111111111111111 * 10 -> 1111111111111110; Dword reveals the
  extra high bit.
- Hex / Byte: FF + 1 -> 0 while the retained value is 100 in Dword.
- Oct / Byte: 377 + 1 -> 0 while the retained value is 400 in Dword.
- Bin / Byte: 11111111 + 1 -> 0 while the retained value is 100000000 in Dword.

Reference executable evidence
- Selector/mask dispatch: 0x004037C7..0x0040380A
- Mask table: 0x0040C418 = FFFFFFFF, FFFF, FF
- Pre-mask DWORD range check and display mask: 0x0040426C..0x004042FE
- Direct radix entry into full value: 0x00402D35..0x00402D4C
- NOT EAX / FILD DWORD: 0x00404399..0x004043CB
- 32-bit And/Or/Xor/Lsh: 0x00404DAB..0x00404E2A

Validation available in this environment
- Rust lexical delimiter scan passes.
- Platform-separation check passes.
- Python helper scripts compile with py_compile.
- Bash build/clean scripts pass bash -n.
- Source manifest verifies.
- ZIP integrity verifies.

A native Cargo/wxDragon build and live Windows GUI run cannot be performed in
this container because the Rust/Windows GUI toolchain is unavailable here.

## OpenCalc v1.1.10 buildfix142

Fix
Buildfix141 contained the Windows-native Help-return keyboard/focus helpers in
src/platform/windows.rs, but src/platform/mod.rs did not publicly re-export
three of them. The Windows UI therefore could not resolve:

- install_click_activation_focus_recovery
- install_keydown_translation
- keydown_character

Buildfix142 re-exports those Windows-only platform helpers through the same
platform facade used by src/ui/windows.rs. No calculator behavior was changed;
the Hex/Oct/Bin radix fixes from buildfix140/141 remain intact.

Validation available in this environment
- Python helper scripts compile with py_compile.
- Rust lexical delimiter scan passes.
- Platform-separation check passes.
- Source manifest is regenerated and verifies.
- ZIP integrity verifies.

A native Windows Rust/wxDragon build cannot be run in this container because
Cargo/Rust and the Windows GUI toolchain are unavailable here.

## OpenCalc v1.1.10 buildfix141

Hexadecimal and binary non-decimal parity
- Hex and Bin now have explicit regression coverage for the same Scientific-mode
  radix path fixed for Oct in buildfix140.
- Hexadecimal button arithmetic keeps every operand in base 16 before evaluation.
  In particular, digit-only values that are also valid decimal text are no longer
  vulnerable to decimal interpretation: 777 + 77 evaluates to 7EE, not 356.
- Hexadecimal A-F entry is covered separately: FF + 1 evaluates to 100.
- Binary button arithmetic keeps every operand in base 2: 111 + 11 evaluates to
  1010, not a decimal sum subsequently formatted as binary.
- Pasted bare-number expressions are explicitly regression-tested in both Hex and
  Bin as well as Oct.
- Calculation History remains in the selected radix's visible notation and hides
  the internal 0x / 0b prefixes used by the expression engine.
- Dword / Word / Byte remains the secondary selector for Hex and Bin exactly as
  for Oct; Decimal remains Deg / Rad / Grad.

Preserved
- Buildfix140's shared non-decimal implementation remains the single code path for
  Hex, Oct and Bin rather than adding per-base arithmetic special cases.
- Buildfix139's Windows Help/keyboard focus recovery remains intact.
- Windows wxDragon and Linux GTK continue to share the same calculator model.

Validation in this packaging environment
- PASS: Rust lexical delimiter scan accepts the source tree.
- PASS: platform-separation checks pass for Windows wxDragon and Linux GTK.
- PASS: git diff --check reports no whitespace errors for the buildfix141 delta.
- A Rust/Cargo toolchain is not installed in this environment, so the Rust unit
  tests and native GUI builds cannot be executed here. Rebuild with build.bat on
  Windows before binary release.

## OpenCalc v1.1.10 buildfix140

Classic non-decimal selector and radix fix
- The right-hand Scientific selector now follows the Windows 95 Calculator:
  Decimal shows Deg / Rad / Grad; Hex, Oct and Bin show Dword / Word / Byte.
- Dword displays the complete low 32-bit representation, Word the low 16 bits,
  and Byte the low 8 bits while retaining the current full numeric value.
- The original F2/F3/F4 behavior is restored in non-decimal modes:
  Dword / Word / Byte. In decimal mode F2/F4 remain Deg / Grad and F6 is Rad.
- Context Help follows the selector labels and uses the existing localized
  Dword / Word / Byte help entries.

Radix arithmetic
- Scientific-mode button expressions now preserve the selected radix internally
  instead of passing unprefixed non-decimal digits to the decimal expression
  parser.
- In Oct mode, 777 + 77 now evaluates as octal and displays 1076, matching the
  reference Calculator, rather than parsing the operands as decimal and merely
  formatting the decimal result as octal (which produced 1526).
- Pasted bare-number expressions also use the selected Hex/Oct/Bin radix.
  Explicit 0x / 0o / 0b literals remain accepted unchanged.
- Calculation History keeps the classic visible notation (for example
  777+77=) and does not expose the internal 0o prefixes.
- Negative non-decimal display formatting is constrained to the selected
  Dword/Word/Byte width instead of leaking a 64-bit representation.

Preserved
- Buildfix139's Windows KEY_DOWN shortcut/focus recovery is retained.
- Both the Windows wxDragon and Linux GTK interfaces implement the same selector
  state and arithmetic model.

Validation in this packaging environment
- PASS: Rust lexical delimiter scan accepted all 20 source files.
- PASS: platform-separation checks passed for Windows wxDragon and Linux GTK.
- PASS: git diff --check reported no whitespace errors.
- Added calculator regression tests for 777+77 octal arithmetic, pasted octal
  expressions, hexadecimal Scientific arithmetic, and Dword/Word/Byte views.
- A Rust/Cargo toolchain is not installed in this environment, so the new tests
  and native GUI builds could not be executed here. Rebuild with build.bat on
  Windows before binary release.

## OpenCalc v1.1.10 buildfix139

Fix
Returning from Help Topics can focus a native calculator button. Event tracing
confirmed that wx delivers KEY_DOWN there without delivering CHAR. Buildfix138
waited for CHAR for printable shortcuts, so focus recovery alone did not fix it.

Calculator shortcuts now resolve once on KEY_DOWN. A scoped native message
subclass supplies the original virtual key and scan code to ToUnicodeEx for
keyboard-layout translation without modifying Windows' dead-key state. Enter
and Escape request dialog-key delivery so the default/focused button cannot
consume them before the configured calculator shortcut is resolved.

Display selection and graph editing retain their native input handling. The
temporary diagnostic logging has been removed. Linux is unchanged.

Validation on Windows
- Reproduced buildfix138: after menu-launched Help closed, Back had focus;
  digit KEY_DOWN arrived, CHAR did not, and the display did not change.
- Fixed build: while Help remained open, returning/clicking Calculator and
  pressing 8 changed 7 to 78 with Back focused.
- After Help closed, pressing 9 changed 78 to 789 with Back focused.
- Repeated menu Help open/close: 7, shifted +, numpad 1, Enter produced 8
  and a single 7 + 1 = 8 history entry, still with Back focused.
- Ctrl+Z restored 1; Ctrl+Y restored 8. Display Ctrl+A selected its text.
- cargo test --locked: 77 passed, zero failed.
- cargo build --release --locked: passed.

## OpenCalc v1.1.10 buildfix138

Changes
- Fixed Windows keyboard shortcut focus after Help > Help Topics activates the external hlp-viewer.exe.
- Help launch now records the calculator's legitimate keyboard target (main frame, Standard/Scientific display, or Graph expression editor).
- Focus restoration is armed only after OpenCalc is genuinely deactivated by the Help viewer, then applied when OpenCalc becomes active again. This fixes lost calculator shortcuts when returning from Help without restoring the broad activation-time focus stealing removed in buildfix136.
- A failed Help launch clears the pending focus restoration state instead of affecting a later unrelated activation.
- Buildfix137's empty-client-surface focus recovery remains unchanged.

Validation
- PASS: Rust lexical delimiter scan accepted all 20 source files.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- Manual code-path review confirms Help restoration is scoped to a successfully launched Help session that has actually deactivated OpenCalc.
- A Rust/Cargo toolchain is not installed in the packaging environment, so cargo test/build could not be rerun here. Rebuild with build.bat on Windows before release.

## OpenCalc v1.1.10 buildfix137

Changes
- Fixed Windows keyboard focus after native menu use: clicking an empty calculator/client surface now restores focus to the main frame so calculator shortcuts work immediately again.
- The recovery is deliberately limited to background panels. Native display text selection and graph-expression editing keep their own focus behavior.
- No calculator, shortcut, settings, layout, graph, history, or Linux behavior was otherwise changed.

Validation
- PASS: Rust lexical delimiter scan accepted all 20 source files.
- PASS: platform-separation checks passed for the Windows wxDragon and Linux GTK backends.
- Diff against buildfix136 is limited to the Windows focus-recovery hook plus this buildfix report/manifest update.
- A Rust toolchain is not installed in the packaging environment, so cargo test/build could not be rerun here. Buildfix136's existing locked test/build validation remains unchanged; buildfix137 should be rebuilt with build.bat on Windows before release.

## OpenCalc v1.1.10 buildfix136

Changes
- Keyboard handlers now cover every calculator button and focusable calculator container. This fixes input being lost when startup focus lands on a button.
- Removed activation-time focus stealing; graph editing and display selection remain separate native input contexts.
- Shared shortcut definitions replace duplicated Windows/Linux mappings. C clears the calculation by default; Shift+C enters hexadecimal C.
- Settings... > Keyboard Shortcuts includes current/default bindings, direct Press key for [action] recording, Save, Cancel, and Restore Defaults. The Windows dialog uses automatic layout so its controls fit.
- Shortcut configuration persists in OpenCalc.cfg, rejects duplicates, permits disabled bindings, and safely falls back to defaults for invalid hand-edited shortcut sections.
- Removed unused Windows constants/FFI and the unused calculation-log wrapper. Retained Linux-only functionality with appropriate compilation conditions.
- Included Cargo.lock with compatible wxdragon, wxdragon-sys, and wxdragon-macros 0.9.17 versions. The original unlocked resolution selected incompatible support libraries.

Validation
- PASS: final cargo test --locked: 77 tests, zero failures (69 existing plus 8 new shortcut/settings tests).
- PASS: Windows optimized release build.
- Live Windows checks: reproduced ignored digit input with Back initially focused; verified default number entry and C clearing; visually verified Settings... menu and resized dialog with all controls visible; verified recording button enters listening state.
- The final adjustment to record keys when the dialog itself has focus was compiled and unit-tested, but direct recording/save/relaunch was not fully retested after the user requested packaging priority.
- Linux UI compilation/live tests were not performed: installed WSL environment has GTK 4.6.9, below required GTK 4.10, and no Cargo. Shared shortcut logic was tested on Windows.
- Strict Clippy did not pass: 17 style/complexity findings remain, including existing calculator and graph/UI code. No broad style rewrite was attempted.

Packages
- Source ZIP: full updated source, Cargo.lock, help assets, and this report. Build with build.bat on Windows.
- Windows ZIP: optimized OpenCalc.exe plus the original help viewer and localized Help files. Extract the entire folder before running.
- No test configuration is included; a fresh extraction uses default shortcuts.
