# OpenCalc — Rust / wxDragon

OpenCalc is a clean-room Rust reimplementation of the supplied Windows 95 `CALC.EXE`. Version 1.1.10 keeps the genuine wxDragon/wxWidgets front end and the corrected expression parser. Buildfix40 replaces the separate History sidecar window with a real native vertical `wxSplitterWindow`: Calculator remains a fixed recovered-layout pane, History is a resizable pane on the right, and the operating-system window gap is gone. Buildfix43 extends the recovered classic 3-D control language to wxGTK/Linux instead of letting the desktop GTK theme replace the button/display/group bevels. Buildfix44 treats the open Statistics Box as an active member of the Calculator window group: clicking Calculator no longer leaves the Statistics utility looking abandoned/inactive, but real keyboard focus is not stolen from the control the user clicked. Buildfix48 strengthens the custom classic pushbutton bevel so the 3-D effect is visibly closer to the original Scientific calculator instead of remaining too flat on current native themes. Buildfix49 moves the decorative History boundary rule out of the pane interior and onto the splitter boundary itself so the etched vertical line properly joins the surrounding horizontal etched lines. Buildfix50 made the Windows button bevel DPI-aware, but incorrectly replaced the native button face with a fixed darker gray and added an oversized projected shadow. Buildfix51 renames the project and built binary to OpenCalc, embeds `calc95.ico` into the Windows PE resource table, selects a native extensionless HLP viewer on Linux, and gives the Statistics Box real retained Windows focus: Calculator mouse clicks are delivered without activating Calculator while Statistics is foreground, and RET remains the explicit way to return focus. Buildfix52 restores the active Windows button-face colour and keeps only a two-level DPI-scaled bevel, so the borders become thicker without darkening the button surface. Buildfix54 stores History numeric punctuation canonically and re-renders existing entries whenever the decimal separator changes, so both expressions and numeric results immediately follow the period/comma preference. Buildfix55 replaces the incomplete top-level-only Statistics activation guard with an explicit native owner relationship, descendant-aware mouse-activation guards, and explicit foreground activation when the Statistics Box opens, so it remains above Calculator and Calculator clicks no longer deactivate it. Buildfix57 separates initial Statistics centering from later activation and layout synchronization: a newly created or reopened box is centered once, while an already-open box keeps the position chosen by the user when Calculator is clicked, moved, resized, switched between modes, or changes History width. Buildfix58 adds an optional function-graph pane on the left while preserving the recovered Calculator geometry in the centre. Buildfix59 shortens the two action labels to **Clear** and **Export** in all supported languages without changing their commands or geometry. Buildfix60 gives PNG/JPG/SVG graph exports a presentation layout with a typeset function heading and a dedicated roots summary, and adds complete Portuguese and Spanish WinHelp manuals while retaining the existing English manual. Help Topics now selects the HLP/CNT pair that matches the current interface language. History recall, Statistics restoration, full-state Undo/Redo, error compatibility, high-DPI sizing, localization, locale input, and the corrected paste parser remain intact. Buildfix77 is Linux-only: it lightens the wxGTK classic palette, selects Liberation Sans for closer Windows metrics, exposes the native History splitter sash for dragging, corrects Standard/Scientific and Graph-pane resizing, keeps Calculator keyboard focus when Statistics opens, and presents About and Help failures as real modal dialogs. The Windows interface and its existing native code paths are unchanged. Buildfix78 is a structural refactor: shared calculator behavior now lives in target-neutral modules, while Windows, Linux, and portable fallback frontend/integration code live in separate source files selected at compile time. No interface behavior is intentionally changed by buildfix78. Buildfix79 adds native GTK3 clipboard access to the Linux integration backend and changes only the Linux calculator face to the exact neutral shade `RGB(240,240,240)` / `#f0f0f0`; the Windows frontend remains untouched. Buildfix80 fixes Linux pane/mode allocation by shrinking child widgets before the top-level GTK request is applied, temporarily releasing the GTK non-resizable flag during programmatic transitions, and reapplying the exact client size after layout. The History sash remains draggable and the Graph separator is now a Linux-only resize handle; Windows sizing and pane behavior remain unchanged. Buildfix81 makes Linux Copy and Paste focus-aware: when the Graph Function field owns keyboard focus, Edit > Copy/Paste and Ctrl+C/Ctrl+V use ordinary text-selection and caret semantics instead of sending the polynomial to the numeric calculator parser. Buildfix82 generalizes that focus-aware editor routing to wxMSW/Windows through the native Unicode EDIT selection and replacement messages, so both supported frontends behave consistently. Buildfix83 fixes the remaining Linux Statistics stacking race: the utility is now an explicit native GTK transient of Calculator, is raised without activation, and is never lowered merely because Calculator was clicked. It therefore remains above its owner without becoming globally topmost over unrelated applications. Buildfix84 removes the last visible dip when a Calculator button is pressed: while either OpenCalc window is active, Linux arms the window-manager above hint before input is delivered; deactivation is settled on the next GTK idle turn so an internal focus transfer keeps the hint while switching to another application removes it.

## Linux interface and packaging — buildfix77

Buildfix77 deliberately gates every new interface correction to Linux. Windows keeps its existing native `COLOR_BTNFACE`, `Microsoft Sans Serif`, MessageBox, Statistics activation, splitter decoration, and realized-HWND sizing paths.

On wxGTK, the calculator surfaces and custom control CSS use the exact neutral `RGB(240,240,240)` / `#f0f0f0` face requested for Linux, and the UI requests `Liberation Sans` instead of inheriting a heavier desktop font. The decorative History rule is hosted inside the History pane rather than over the `wxSplitterWindow` sash, so the sash receives pointer events and the panel width can be dragged again.

Linux no longer fixes the top-level frame by repeatedly assigning equal minimum and maximum sizes. It uses GTK's non-resizable window flag while preserving programmatic client-size changes, then explicitly lays out and updates the root surface. The Graph panel and Calculator/History splitter also receive logical wxGTK rectangles when the Win32-only pixel-position helper is unavailable. This lets the Graph pane open beside Standard mode instead of underneath it and gives Standard/Scientific transitions their complete width and height.

The Statistics Box remains visible and above Calculator on Linux, but Calculator retains keyboard focus after the box opens. About OpenCalc, missing native Help viewer errors, missing Help files, and other application messages now use a synchronous wxDragon message dialog rather than being written only to stderr. On Linux, Help accepts only an executable extensionless `hlp-viewer` beside `OpenCalc`.

The supplied Linux build script is included as `build.sh` and packages the release into `build-linux/` with `OpenCalc`, `calc.tooltip`, all localized Help pairs, and an extensionless native `hlp-viewer` when present. Run:

```bash
chmod +x build.sh clean-linux.sh
./build.sh
```

To remove Cargo output, the packaged Linux directory, a root test executable, and copied WSL `Zone.Identifier` marker files while retaining source/runtime inputs:

```bash
./clean-linux.sh
```

## Linux Statistics no-dip stacking — buildfix84

Buildfix84 keeps the buildfix83 transient ownership but adds an active-application `keep-above` guard on Linux. The hint is enabled before the Statistics Box is shown and remains enabled while either Calculator or Statistics is the active OpenCalc window, so pressing a Calculator button cannot place the owner above the utility even for a single compositor frame.

GTK activation/deactivation notifications can arrive in different orders when focus moves between the two OpenCalc windows. Instead of removing the hint immediately on a deactivation event, the Linux backend retains the GTK window until the next idle turn and then checks both the Statistics window and its transient owner. An internal focus transfer therefore preserves stacking, while switching to another application removes the hint and allows that application to cover the whole OpenCalc group normally. The Windows ownership and `WM_MOUSEACTIVATE` implementation are unchanged.

## Linux Statistics ownership — buildfix83

Buildfix83 gives the Linux Statistics Box an explicit GTK transient relationship with Calculator rather than relying on a temporary global `keep-above` hint. Owner activation now raises the utility through its native GDK window without requesting keyboard focus. The Statistics activation hook no longer lowers the utility on deactivation, eliminating the event-order race that allowed Calculator to cover it when clicked. Because the relationship is transient rather than globally topmost, unrelated applications can still cover the complete OpenCalc window group normally. Windows companion ownership and `WM_MOUSEACTIVATE` behavior are unchanged.

## Linux clipboard and exact neutral face — buildfix79

Buildfix79 is confined to `src/platform/linux.rs` and `src/ui/linux.rs`. The Linux Edit > Copy and Edit > Paste commands, together with the existing Ctrl+C and Ctrl+V accelerators, now use the desktop GTK3 clipboard. Copied display text is published as UTF-8 and requested for clipboard-manager persistence; pasted UTF-8 text continues through the shared corrected expression parser. Empty or non-text clipboard contents remain a no-op, while a missing GTK display/clipboard follows the recovered `Cannot open Clipboard.` error path.

The Linux calculator surface, classic button faces, and sunken field faces now use exactly `RGB(240,240,240)` (`#f0f0f0`). The former warm `RGB(212,208,200)` / `#d4d0c8` face and `#ece9d8` highlight are absent from the Linux backend. No Windows frontend, Win32 clipboard, Windows theme, or Windows palette code changed.


## Linux exact pane sizing and draggable panels — buildfix80

Buildfix80 corrects the wxGTK allocation order used whenever Standard/Scientific mode or the Graph/History visibility changes. Linux now temporarily releases GTK's top-level resize lock, hides or shows the requested panes, resizes the active calculator panel and nested splitter first, forces their allocation, and only then applies the exact client size. The client-size request is repeated after layout so returning from Scientific to Standard cannot retain the former width or height as blank space. The Windows native DPI fitter keeps its previous ordering and behavior.

The History pane continues to use the native `wxSplitterWindow` sash and persists its width through `history_width`. The Graph pane now has a wider Linux-only transparent drag target around its etched separator; dragging and releasing that separator changes the Graph width while the Calculator and History geometry remain fixed. Graph resizing is intentionally confined to `src/ui/linux.rs`; Windows retains the established fixed Graph pane.

## Focus-aware Graph clipboard editing — buildfix81

OpenCalc's Copy and Paste menu commands are frame-level accelerators. Before buildfix81, Ctrl+V could therefore intercept a paste intended for the Graph **Function** text field and send the clipboard contents to the numeric calculator parser. A polynomial such as `2x**2 + 3x + 10` was then evaluated without a value for `x`, produced `Unexpected token after expression: Ident("x")`, and was incorrectly added to Calculation History.

Buildfix81 checks which native control owns keyboard focus before handling Copy or Paste. When the Function field is focused, Copy exports only its selected text, while Paste replaces the current selection or inserts at the caret. The text remains in the editor, is not evaluated as an ordinary numeric calculation, and does not create a History entry. The Graph parser already accepts implicit multiplication and all three supported exponent forms, so `2x**2 + 3x + 10` normalizes to `2*x**2+3*x+10` when **Plot** is pressed.

## Cross-platform Graph clipboard editing — buildfix82

Buildfix82 applies the same focus-aware routing on Windows. The wxMSW Function field is a native Unicode EDIT control: OpenCalc reads its selection through `EM_GETSEL`, copies only that selected UTF-16 range, and performs Paste through `EM_REPLACESEL`, which replaces the selection or inserts at the caret while preserving the control's undo behavior.

The shared calculator action remains platform-neutral. Windows and Linux backends now both report whether the Function editor owns keyboard focus and provide their native selection/replacement bridge; the portable fallback retains safe no-op/full-field behavior. Pasting `2x**2 + 3x + 10` into the focused Function field therefore stays in the graph editor on both Windows and Linux and never enters Calculation History as a numeric-parser error.

Pasting into the ordinary Calculator display still follows the numeric expression evaluator, where a free variable is intentionally invalid.

## Platform-separated frontend architecture — buildfix78

Buildfix78 removes the mixed-platform `src/ui.rs`, `src/platform.rs`, and `src/locale.rs` files. The project now has three compile-time-selected backend families:

```text
src/
├── ui/
│   ├── mod.rs       shared controls, calculator actions, history, graph, and statistics logic
│   ├── windows.rs   wxMSW theme, decorators, selector bridge, DPI fallback, and focus policy
│   ├── linux.rs     wxGTK palette, font, sash layout, modal dialogs, sizing, and focus policy
│   └── other.rs     portable fallback
├── platform/
│   ├── mod.rs       shared integration facade and Help-file selection
│   ├── windows.rs   USER32/GDI/clipboard/DPI/activation implementation
│   ├── linux.rs     GTK3 CSS, fixed-frame policy, and Linux Help-viewer rules
│   └── other.rs     portable stubs
└── locale/
    ├── mod.rs       target-neutral numeric localization
    ├── windows.rs   Windows NLS discovery
    ├── linux.rs     glibc LC_NUMERIC discovery
    └── other.rs     invariant fallback
```

Each `mod.rs` contains only the small `cfg(target_os = ...)` selector and the shared contract. OS APIs and widget-policy branches are confined to the corresponding backend file. Consequently, Linux-only palette, sizing, sash, modal-dialog, and Statistics-focus changes cannot be compiled into the Windows frontend, while the Windows USER32/GDI behavior cannot leak into wxGTK.

`tools/check_platform_separation.py` verifies that the three backend families expose matching facades, that target-specific API tokens do not reappear in shared modules, and that the obsolete mixed-platform source files remain absent. `tools/check_rust_delimiters.py` performs a lexical delimiter/string/comment scan over every Rust source file.

## Reference executable

Reference SHA-256:

`b064b0ac430264eff7b79b91e743bcd36d7b3707857f5bcdc4db146911dd0e28`

The reference is Windows Calculator 4.00.950. Reverse engineering established the original Edit/View/Help menus, `SciCalc` resources, selector/status controls, keypad strings, clipboard command translation, parenthesis handling, and scientific precedence machinery. See `REVERSE_ENGINEERING.md`.

## Native wxDragon interface — buildfix7 post-realization DPI correction

All application controls remain real wxDragon/wxWidgets controls. The frame, menus, text display, buttons, checkboxes, radio buttons, group boxes, and status indicators are not replaced by a fake painted application window.

**Buildfix7 fixes the still-cropped high-DPI window shown after buildfix6.** The previous fix asked wxDragon for the panel size before the frame was shown. On the reported 200%-scale desktop, that still returned the 325×255 logical design size, while Windows subsequently realized the child HWND positions and sizes at roughly twice those physical coordinates. The frame was therefore locked to the same too-small width again.

The correction no longer infers physical size from wxDragon `Size` values. After `Frame::show(true)`, the Windows path now:

1. obtains the realized monitor DPI with `GetDpiForWindow`;
2. converts the 120-DPI design client size to the exact required physical client size;
3. compares the top-level HWND's physical `GetClientRect` and `GetWindowRect`;
4. grows the real frame with `SetWindowPos` by the exact client-area deficit, with a second correction pass for non-client/menu rounding; and
5. explicitly resizes the active wxDragon `Panel` HWND to that same physical client surface.

The old wx-level min/max lock is removed, because applying logical size constraints after the native high-DPI resize could clamp the frame back to the cropped dimensions. The same realized-HWND path runs when switching between Standard and Scientific mode.

At 200% monitor DPI, the Standard design client of 325×255 now becomes a 650×510 physical client area, matching the physical coordinate space in which the child controls are actually realized. At 100%, 125%, 150%, and other monitor scales, the calculation follows the window's actual DPI instead of assuming a scale factor.

The rendering/layout corrections from buildfix5 remain:

- `PER_MONITOR_AWARE_V2` is enabled before wxWidgets creates HWNDs.
- The UI uses the TrueType `Microsoft Sans Serif` alias and the coloured classic-button painter requests `CLEARTYPE_NATURAL_QUALITY`.
- Standard mode has one recessed memory/status well; Scientific mode retains the two recovered memory/parenthesis wells.
- `Back`, `CE`, and `C` use the recovered dark maroon `RGB(128,0,0)` label colour.
- All calculator pushbuttons remain bold and use classic raised/sunken 3-D edges.
- Recovered 96-DPI coordinates retain the requested 120-DPI design enlargement and selector spacing.

Scientific keypad visual order remains:

```text
Sta   F-E   (     )     MC   7    8    9    /    Mod   And
Ave   dms   Exp   ln    MR   4    5    6    *    Or    Xor
Sum   sin   x^y   log   MS   1    2    3    -    Lsh   Not
s     cos   x^3   n!    M+   0    +/-  .    +    =     Int
Dat   tan   x^2   1/x   PI   A    B    C    D    E     F
```

The crate remains pinned to:

```toml
wxdragon = "=0.9.17"
```


## Numeric locale / decimal separator — buildfix10 + buildfix31

Calculator arithmetic keeps an invariant `.` radix internally and localizes only the user-facing number text. On first run, Windows reads the current user's non-monetary decimal/thousands symbols from the Windows NLS locale APIs; Linux selects `LC_NUMERIC` from the environment and reads `decimal_point` / `thousands_sep` through `localeconv()`.

Buildfix31 adds an explicit **View > Decimal separator** submenu. The user can choose a period or comma, and the choice is written immediately to `OpenCalc.cfg`. Switching the preference converts the current display punctuation in place without changing the numeric value or abandoning an entry already in progress.

Keyboard entry deliberately accepts **both `.` and `,`** regardless of the display preference. Pasted expressions do the same and also accept the two common grouped forms `1,234.56` and `1.234,56`. Internally, parsing and arithmetic remain locale-independent.

The classic Calculator display still does not insert thousands grouping automatically; only the radix symbol is localized. Group separators are accepted when pasted back into the parser.

## Corrected expression paste parser

The Windows 95 binary translates pasted characters into calculator commands rather than parsing a full mathematical grammar. The Rust implementation instead validates and evaluates the complete expression with a lexer and Pratt parser before changing Calculator state.

Examples:

```text
(2 + 2) * 4      => 16
2 * -3           => -6
2 -- 3           => 5
-(2 + 3)         => -5
3 * (4 + 5)      => 27
1e-3 * 1000      => 1
sin(30)          => 0.5   (Degrees mode)
cos(60)          => 0.5   (Degrees mode)
tan(45)          => 1     (Degrees mode)
sqrt(9) + 2^3    => 11
2**3             => 8
9**0.5           => 3
2**-3            => 0.125
2**3**2          => 512   (right-associative)
pi                => 3.141592...
pi()              => 3.141592...
5!                => 120
(3+2)!            => 120
factorial(6)      => 720
```

`^` and `**` are synonyms for general exponentiation; the exponent may be an integer, fractional value, negative value, or another expression. Exponentiation is right-associative, so `2**3**2` means `2**(3**2)`.

Factorial is available as postfix `!` and through `factorial(x)` / `fact(x)`. It uses the same recovered Calculator domain categories as the scientific `n!` button: negative or non-integral operands are **Invalid input for function.**, while values above `170!` are **Result is too large.**

Malformed text such as `12+34@56` is rejected transactionally; it cannot partially execute the valid prefix.


## Win95-style “What’s This?” context help — buildfix9 + buildfix31

The control-level help path from the original Calculator is restored. Right-click a Calculator control to open the localized native **What’s This?** context menu; selecting it displays a pale-yellow popup tooltip beside the pointer. Buildfix31 extends dismissal beyond the source control: a click on another Calculator control, empty calculator surface, title/non-client area, menu command, or another application closes the tracking tooltip. Escape also dismisses it.

The Calculator does **not** parse `CALC.HLP` to produce these popups at runtime. The original Portuguese control-help strings were decoded once from the supplied Windows 95 `CALC.HLP` and placed in the UTF-8 plain-text file `calc.tooltip`. Buildfix31 adds English and Spanish translations. Sections are language-qualified, for example `[en.back]`, `[pt.back]`, and `[es.back]`, while the semantic key remains stable. This keeps all tooltip wording editable and auditable without duplicating the HLP parser inside Calculator.

Context help is attached to the Standard and Scientific displays, memory/parenthesis status wells, all keypad buttons, the radix selectors, angle selectors, and `Inv`/`Hyp`. Changing the application language updates the already-created context-help bindings immediately; no restart is required.

`build.bat` copies `calc.tooltip` beside the release executable. Calculator searches for the catalog beside its own executable and then in the current directory.

## Undo / Redo — buildfix34

The **Edit** menu now contains **Undo** (`Ctrl+Z`) and **Redo** (`Ctrl+Y`). The commands use bounded calculator-state snapshots rather than editing only the display text, so an undo can restore the complete computational context: the displayed value/error, pending operation and accumulator, current entry, memory, scientific expression/parentheses, radix, angle mode, Inv/Hyp flags, F-E state, statistics data, and Standard/Scientific mode.

Undo/Redo items are disabled when their respective history direction is empty. A successful new calculator action after Undo clears the Redo branch; no-op input does not create an empty history entry. Copy, Help/About, opening the Statistics Box, language changes, and decimal-separator preference changes are intentionally not history edits. Decimal punctuation remains a persisted preference, so restored snapshots are normalized to the separator currently selected in **View > Decimal separator**.

Undo/Redo menu labels and help strings are localized in English, Portuguese, and Spanish. Undo/Redo state history is intentionally session-only and is not written to `OpenCalc.cfg`.

## Calculation History splitter — buildfix35 through buildfix40

**View > History** shows or hides a vertical calculation-history pane on the right side of the **same Calculator top-level window**. Buildfix40 supersedes the buildfix36–39 sidecar architecture: History is now the right child of a genuine wxDragon `SplitterWindow`, while a dedicated Calculator host is the left child. Because both panes share one frame, there is no title-border/window-manager gap between them.

The splitter deliberately does not use live-update dragging. A classic sash guide moves while it is dragged; when released, the requested delta is converted into a History-width change while the Calculator pane is restored to its exact recovered Standard/Scientific width. This prevents the clipping/reflow regression that occurred when buildfix35 simply enlarged the Calculator client surface. The selected History width is persisted in `OpenCalc.cfg` and restored on the next run.

Inside the pane, the UI retains the same native/classic design language: a compact localized **History** heading, a read-only multiline field with native vertical scrolling, and a bottom-aligned **Clear** button. The History controls reflow only within their pane when its width or the Calculator mode height changes; Calculator controls retain their recovered coordinates.

The log records completed calculations rather than raw keystrokes. Standard-mode `=` calculations and chained binary results, Scientific-mode expressions, unary functions, percentages, statistics aggregates, and pasted expressions are recorded with their resulting display value or localized Calculator error. Entries are newest-first and bounded to 256 items. The log keeps collecting while the pane is hidden. Each successful record also stores the exact numeric result: clicking anywhere on a numeric History entry recalls that captured value into the main Calculator display without reparsing localized text or re-running the original expression. Error-only entries remain visible but are not recallable. Recalling a result is an ordinary undoable Calculator-state change and does not add a duplicate History record.

The calculation log is intentionally independent from `Ctrl+Z`/`Ctrl+Y`: clearing the visible pane does not erase Undo/Redo snapshots, and Undo/Redo does not rewrite the activity log. Calculation entries themselves remain session-only. **View > History** is persisted as `history_visible=true|false`, and the splitter-selected width is persisted as `history_width=<source pixels>`.

### Statistics Box positioning — buildfix37 through buildfix40

The Statistics Box remains a separate titled utility window owned by Calculator. Buildfix38 centers it over the main Calculator frame, and buildfix39 preserves its logical open state across Calculator minimization: an existing Statistics Box is restored with Calculator, while a box that was closed (or never created) is not recreated. Buildfix40 keeps that behavior; because History is now a child pane, changing its visibility or splitter width simply re-centers an already-open Statistics Box over the resulting single application frame. On Windows the centered position is clamped to the current monitor work area.

## Localization and persistent preferences — buildfix31 + buildfix35 + buildfix40

Human-facing menu/window strings are centralized in `src/i18n.rs` rather than scattered through the UI code. The **View > Language** submenu switches the interface live between **English**, **Português**, and **Español**. Menu titles/items, window titles, About text, known runtime/clipboard messages, the **What’s This?** command, and the context-help catalog follow the selected language immediately. Compact mathematical/operator labels are intentionally kept stable to preserve the recovered Windows 95 control geometry.

Preferences live in a small, dependency-free text file named `OpenCalc.cfg`, normally beside the executable (with the current directory as a writable fallback):

```ini
# OpenCalc preferences
language=en
decimal_separator=period
history_visible=true
graph_visible=false
history_width=210
```

Supported language values are `en`, `pt`, and `es`; the decimal value is `period` or `comma`; `history_visible` and `graph_visible` accept Boolean values; and `history_width` stores the splitter-selected History width in the recovered 96-DPI source coordinate system (clamped to 120–420). The file is read on startup and rewritten whenever a menu preference or splitter width changes.

Buildfix51 changes the primary filename to `OpenCalc.cfg`. If that file is absent, OpenCalc can still read the former `Calculator95-Rust.cfg` beside the executable or in the current directory; the next successful save writes the renamed `OpenCalc.cfg`.

## Calculator Help through Rust HLP Viewer

`Help > Help Topics` and `F1` launch a platform-native companion viewer with the manual selected from the current interface language:

```text
English:    Help/CALC_EN.HLP + Help/CALC_EN.CNT
Portuguese: Help/CALC_PT-BR.HLP + Help/CALC_PT-BR.CNT
Spanish:    Help/CALC_ES.HLP + Help/CALC_ES.CNT
```

For example, Windows launches `hlp-viewer.exe Help\CALC_PT-BR.HLP` when Portuguese is selected; Linux uses the extensionless native `hlp-viewer`. OpenCalc searches the `Help` directory beside its executable and under the current directory, and Windows additionally checks `%WINDIR%\HELP`. Portuguese and Spanish prefer their matching manuals and fall back to `Help\CALC_EN.HLP` when the localized HLP is unavailable. Each CNT uses the same basename as its HLP and its `:Base` directive names that exact HLP.

All three manuals include dedicated **Copy and Paste**, **Pasted Expression Syntax**, **Functions and Constants**, **Numbers, Locales, and Based Literals**, and **Paste and Graph Examples** topics. They document the accepted operators, parentheses, unary signs, `^`/`**`, postfix factorial and percent, supported functions/constants, locale-tolerant decimal input, and `0x`/`0o`/`0b` literals.

## Build

Requirements on Windows are stable Rust/Cargo plus the native C/C++ environment required by wxDragon/wxWidgets.

Run:

```text
build.bat
```

The resulting root executable is `OpenCalc.exe`. `build.rs` uses the Windows resource compiler through `winresource` to link `calc95.ico` and OpenCalc version metadata directly into the PE file, so Explorer does not depend on the runtime `WM_SETICON` path. `build.bat` copies `hlp-viewer.exe` and `calc.tooltip` beside `target\release\OpenCalc.exe`, and copies the three localized HLP/CNT pairs into `target\release\Help`.

Run parser/calculator tests with:

```text
test.bat
```

On Linux, run `./build.sh`. It performs the release build and creates a self-contained `build-linux/` directory containing `OpenCalc`, `calc.tooltip`, all three localized HLP/CNT pairs, and a native extensionless `hlp-viewer` when one exists in the project root. `hlp-viewer.exe` is intentionally excluded. Run `./clean-linux.sh` to remove Linux build output.

## Source map

- `src/ui/mod.rs` — target-neutral original-layout wxDragon Calculator construction, Calculator/History splitter behavior, graph/statistics logic, clickable history, menus, Undo/Redo, and preference commands.
- `src/ui/windows.rs` — Windows-only wxMSW classic theming, decorator widgets, selector notification bridge, modal-message routing, high-DPI fallback, and Statistics focus policy.
- `src/ui/linux.rs` — Linux-only wxGTK surface/font policy, draggable-sash decoration, logical pane sizing, modal dialogs, and Statistics focus policy.
- `src/ui/other.rs` — portable fallback frontend policy.
- `src/history.rs` — bounded generic snapshot history used by calculator-state Undo/Redo, with branch/limit regression tests.
- `src/calculation_log.rs` — bounded newest-first user-visible calculation log, separate from Undo/Redo state snapshots.
- `src/i18n.rs` — centralized English/Portuguese/Spanish user-interface strings and runtime-message translations.
- `src/settings.rs` — dependency-free `OpenCalc.cfg` loading/saving for language, decimal separator, History visibility, and persisted splitter width, with one-way loading compatibility for the former `Calculator95-Rust.cfg` name.
- `src/calc.rs` — Calculator state machine, memory, bases, functions, status state.
- `src/expr.rs` — corrected locale-aware lexer / Pratt parser and regression tests.
- `src/graph.rs` — graph-expression normalization, sampling, discontinuity handling, visible-range roots, pan/zoom state, and presentation-oriented PNG/JPG/SVG export.
- `src/locale/mod.rs` — canonical/display numeric conversion and compile-time backend selection.
- `src/locale/windows.rs` — Windows NLS decimal-symbol discovery.
- `src/locale/linux.rs` — Linux/glibc `LC_NUMERIC` decimal-symbol discovery.
- `src/locale/other.rs` — portable invariant numeric-locale fallback.
- `src/platform/mod.rs` — target-neutral integration facade, localized Help-file selection, and compile-time backend selection.
- `src/platform/windows.rs` — Windows clipboard, per-monitor DPI, USER32/GDI painters, context help, icon, and Statistics activation integration.
- `src/platform/linux.rs` — Linux wxGTK/GTK3 classic-theme bridge, fixed-frame behavior, and native HLP-viewer discovery.
- `src/platform/other.rs` — portable integration stubs.
- `src/tooltip.rs` — parser/search for the language-qualified plain-text `calc.tooltip` context-help catalog.
- `tools/check_platform_separation.py` — architectural regression check for backend isolation and facade parity.
- `tools/check_rust_delimiters.py` — lightweight Rust lexical integrity scan used when a compiler is unavailable.
- `calc.tooltip` — Portuguese control-help text decoded from the supplied `CALC.HLP`, plus English and Spanish translations.
- `src/main.rs` — entry point.
- `build.rs` — Windows PE icon/version-resource compilation for `OpenCalc.exe`.
- `calc95.ico` — icon reconstructed from the supplied `CALC.EXE` resources.
- `hlp-viewer.exe` — supplied Windows Rust HLP Viewer companion executable, packaged beside OpenCalc; Linux deployments place the native extensionless `hlp-viewer` beside OpenCalc.
- `Help/CALC_EN.HLP` / `Help/CALC_EN.CNT` — English OpenCalc WinHelp manual and Contents hierarchy.
- `Help/CALC_PT-BR.HLP` / `Help/CALC_PT-BR.CNT` — Portuguese OpenCalc WinHelp manual and localized Contents hierarchy.
- `Help/CALC_ES.HLP` / `Help/CALC_ES.CNT` — Spanish OpenCalc WinHelp manual and localized Contents hierarchy.

## Validation note

This artifact environment does not contain `cargo`, `rustc`, or `rustfmt`, so no local Rust compilation is claimed. Static/source/package checks are recorded in `VALIDATION.txt`; the Windows build remains the authoritative compile/runtime check.


## v1.1.6 buildfix8

- Adjusted the Scientific-mode selector spacing for modern DPI/font metrics.
- Widened the Hex/Dec/Oct/Bin and Deg/Rad/Grad radio controls so labels no longer clip.
- Increased the Inv/Hyp group width and checkbox widths.
- Shifted the Scientific display and the small status indicators slightly right so the selector area breathes more naturally.


## v1.1.7 buildfix9

- Restored control-level right-click **What’s This?** behavior.
- Added `calc.tooltip`, using the original control-help text decoded from the supplied `CALC.HLP`.
- Added native popup-menu and tracking-tooltip support without reintroducing an HLP parser into Calculator.


## v1.1.8 buildfix10

- Added operating-system decimal-separator discovery on Windows and Linux.
- The display and decimal button now follow the user locale (`.` or `,`).
- Keyboard input accepts both comma and period as the decimal key.
- Clipboard expressions accept both decimal conventions and common mixed thousands/decimal forms.
- Calculator state remains invariant internally, preventing locale punctuation from breaking arithmetic, memory, scientific expressions, or backspace/sign handling.
- Statistics output now uses the same localized decimal formatting as the main display.


## v1.1.9 buildfix11

- Fixed interactive decimal input. The display intentionally shows a trailing locale radix for integer entries, so Calculator now stores an explicit `decimal_entered` bit instead of trying to infer whether that trailing separator was merely display decoration. `3 , 2` and `3 . 2` therefore both become 3.2 internally and render with the operating-system decimal symbol.
- Keyboard handling accepts both printable comma/period and wxWidgets decimal/separator key codes, including numpad decimal keys.
- Fixed Windows context-help tracking: `TTF_IDISHWND` now uses the containing HWND in `TOOLINFO.hwnd` and the actual control HWND in `uId`; the tooltip is created with the application module handle and classic WinHelp palette before activation.
- Restored the fixed-size Calculator window. wxDragon removes resize/maximize styles cross-platform; Windows also enforces the native `WS_THICKFRAME`/`WS_MAXIMIZEBOX` removal after final DPI sizing, while Linux locks the realized wxGTK min/max size.


## v1.1.10 buildfix12

- Restored the original Scientific top-band structure more closely: a full-width etched separator row, then the display on its own row, then the selector decorators below it.
- Moved the Scientific display, selector group boxes, Inv/Hyp row, status wells and Back/CE/C row so the display no longer visually shares the same band as the decorators.


## v1.1.10 buildfix13

- Fixed the buildfix12 wxDragon compile error by removing the nonexistent `StaticTextStyle::AlignLeft` flag from the empty full-width separator control. Empty `StaticText` requires no alignment style.
- Corrected the Cargo package version from the stale `1.1.9` to `1.1.10`.


## v1.1.10 buildfix14

- Scientific mode now uses a dedicated display band with `separator / display / separator` structure, matching the original layout more closely.
- The full-width etched separators use a taller recessed strip for a more pronounced 3D sunken effect.


## v1.1.10 buildfix15

- Corrected a Windows high-DPI coordinate mismatch affecting the Scientific selector area. Classic/theming-disabled `StaticBox`, radio-button, and checkbox controls are now explicitly mapped from wx logical coordinates to the monitor's physical DPI coordinate space, matching the display and keypad.
- This moves and scales the `Hex/Dec/Oct/Bin`, `Deg/Rad/Grad`, and `Inv/Hyp` groups as a unit instead of applying another arbitrary vertical offset, preserving the original recovered spacing at 100%, 125%, 150%, 200%, and other Windows DPI settings.
- Linux keeps the same wxDragon logical coordinates; the compensation helper is a no-op outside Windows.


## v1.1.10 buildfix16

- Removed the incorrectly added second Scientific separator.
- Replaced the thick recessed separator field with a true two-pixel classic etched rule on Windows (dark 3-D shadow + highlight), kept thin at any DPI.
- Aligned the Inv/Hyp decorator with the first three scientific keypad columns and with the Back/CE/C control band, matching the original screenshot more closely.
- Kept a shallow native wxGTK separator fallback on Linux.


## v1.1.10 buildfix17

- Moved the single scientific top separator and display downward, leaving blank space above the separator as in the original layout.
- Repositioned the selector groups and Inv/Hyp group accordingly.
- Realigned the two scientific status wells to the `)` and `MC` columns and matched their widths to those buttons.
- Reduced the separator control itself to a true thin etched rule while preserving stronger contrast.


## v1.1.10 buildfix18

- Recalculated Scientific mode as one coordinated vertical grid instead of moving individual controls independently.
- Moved the separator to source Y=20 and the display to Y=28, preserving a proper top margin and a compact separator-to-display gap.
- Kept the selector boxes at Y=58, aligned `Inv/Hyp`, the two status wells, and `Back/CE/C` into one command band, and moved the first keypad row to Y=133.
- Subsequent keypad rows remain on the recovered 34-unit pitch, leaving the original-like bottom margin instead of oversetting the panel.


## v1.1.10 buildfix19

- Fixed the broken `Inv / Hyp` decorator edge on high-DPI Windows. Native checkbox control backgrounds were overlapping and erasing the top edge of the enclosing classic group box.
- `Inv` and `Hyp` now use a dedicated vertically centred checkbox row inside the decorator instead of sharing the `Back / CE / C` row coordinate.
- The command buttons and the two sunken status wells keep their existing alignment.


## v1.1.10 buildfix20

- Moved only the Scientific top separator and numeric display upward by four source units (about ten physical pixels on the reported high-DPI setup), leaving the now-correct selector/keypad grid untouched.
- Replaced Windows empty `StaticBox` decorators with an exact wxDragon-owned custom etched-frame painter so left/right borders terminate cleanly at the corners.
- Reworked the small sunken status-well border painter so all four sides are drawn strictly inside the control rectangle and cannot visually overrun the top/bottom edges.


## v1.1.10 buildfix21

- Fixed the remaining clipped decoration on the two blank scientific status wells by decoupling them from the shared command-box top edge.
- Nudge the wells slightly down and slightly right so their custom 3D borders no longer share the same pixel row/column as the neighbouring Inv/Hyp decorator.


## v1.1.10 buildfix22

- Fixed the Scientific selector controls (Hex/Dec/Oct/Bin, Deg/Rad/Grad, Inv/Hyp).
- Selector state is now driven by wxDragon mouse/key window events and explicitly synchronized to the calculator model, avoiding the unreliable themed-control command callback path seen on Windows.
- Space-key activation is supported for focused radio buttons and checkboxes; the same event path is cross-platform under wxGTK/Linux.
- Added model regressions proving base, angle, inverse-trig, and hyperbolic selector state changes affect calculations.

## v1.1.10 buildfix31

- Added live **View > Language** switching for English, Portuguese, and Spanish, with centralized UI strings in `src/i18n.rs`.
- Expanded `calc.tooltip` to complete English, Portuguese, and Spanish catalogs (61 sections per language); language changes refresh already-installed control help immediately.
- Localized the native **What’s This?** context-menu label and the translated tooltip bodies.
- Fixed tracking-tooltip lifetime so clicks outside the tooltip/source control dismiss it as expected, including empty calculator/panel areas, non-client/title clicks, menu commands, and Calculator deactivation; Escape still dismisses it.
- Added **View > Decimal separator > Period / Comma**. Changing it updates both keypad decimal buttons and the current display without changing the stored numeric value or the current entry state.
- Preserved keyboard/paste acceptance of both `.` and `,` regardless of the selected display separator.
- Added `OpenCalc.cfg` persistence for `language=en|pt|es` and `decimal_separator=period|comma`, with executable-directory storage and a current-directory fallback when necessary.
- Added source-level regression coverage for preference parsing/serialization, localized tooltip catalog lookup, and decimal-separator switching during an active entry.

## v1.1.10 buildfix32

- Disassembled the original `CALC.EXE` error dispatcher at `0x00404B13` and recovered all five calculator-runtime error categories exactly: divide by zero, invalid function input, undefined function result, result too large, and result too small.
- Corrected negative square root and non-positive `ln`/`log` to report **Result of function is undefined.** rather than **Invalid input for function.**
- Added the original tangent-asymptote undefined-result guard (`abs(tan) > 1e15`), square (`1e154`) and cube (`1e102`) overflow guards, factorial domain/overflow split, exponent/power underflow reporting, and signed non-decimal range errors.
- Preserved two subtle dispatcher distinctions found in the binary: out-of-range unary **Not** reports **Invalid input for function.**, while out-of-range **And/Or/Xor/Lsh** operands report **Result is too large.**
- Recovered **Inv + x^y** as `x^(1/y)`: selecting it consumes Inv, and a zero `y` reports **Invalid input for function.**, exactly matching the explicit branch at `0x00404FFE`.
- Corrected the Statistics Box so **Ave** with no samples reports **Cannot divide by zero.**, while standard deviation with zero or one sample returns zero as in the original.
- Centralized recovered error text in `src/errors.rs` and added English/Portuguese/Spanish runtime translations for all recovered math, clipboard, help-memory, and startup-memory messages.
- Restored the original **Cannot open Clipboard.** failure text and the original long data-memory error for equivalent Windows integration failures.
- Mirrored the original startup 0x400-byte fallible resource allocation so **Not Enough Memory** has a corresponding startup path.
- Added regression tests for the recovered error categories and documented their disassembly evidence in `REVERSE_ENGINEERING.md`.


## v1.1.10 buildfix33

- Added `**` as a full synonym for `^` in pasted expressions; it is general exponentiation rather than a square-only shortcut.
- Preserved right-associative power parsing, including arbitrary, fractional, and negative exponents such as `2**3`, `9**0.5`, `2**-3`, and `2**3**2`.
- Added zero-argument `pi()` alongside the existing `pi` / `π` constant spelling.
- Documented and regression-tested postfix factorial (`5!`, `(3+2)!`) and named factorial forms (`factorial(6)`, `fact(4)`).
- Added regression coverage for interaction between factorial and exponent precedence (`2^3! == 64`) and for the recovered factorial invalid-input/overflow errors.


## v1.1.10 buildfix34

- Added **Edit > Undo** (`Ctrl+Z`) and **Edit > Redo** (`Ctrl+Y`) with English, Portuguese, and Spanish labels/help text.
- Added a bounded 256-state snapshot history. Undo/Redo restores the complete `Calculator` state, including pending arithmetic, entry state, memory, scientific expression/parentheses, radix/angle/Inv/Hyp/F-E state, statistics data, errors, and Standard/Scientific mode.
- Menu commands automatically enable/disable according to available history.
- New successful calculator input after Undo clears the Redo branch; no-op actions do not create history entries or destroy the redo branch.
- Pasted expressions, calculator buttons, scientific selectors, mode changes, base/angle changes, statistics LOAD/CD/CAD operations, and calculation errors are undoable. Copy, Help/About, opening the Statistics Box, and persistent UI preferences are not treated as calculator edits.
- Restored snapshots keep the currently selected decimal-separator preference rather than silently reverting the persisted UI setting.
- Added `src/history.rs` regression coverage for ordered undo/redo, redo-branch invalidation, and history bounds.


## v1.1.10 buildfix35

- Added a native vertical **History** side panel that aligns to the full Standard or Scientific calculator height and uses a read-only multiline control with a vertical scrollbar.
- Added **View > History** with English, Portuguese, and Spanish labels/help text and persisted visibility. Buildfix36 subsequently detached this view from the Calculator client surface so showing it no longer resizes or reflows Calculator.
- Persisted the visibility preference as `history_visible=true|false` in `OpenCalc.cfg`.
- Added a bottom-aligned localized **Clear** button. Clearing calculation history is deliberately independent from Ctrl+Z/Ctrl+Y state history.
- Added a bounded 256-entry calculation log, newest first. It records completed standard/chained operations, scientific expressions, unary functions, percentages, statistics aggregate results, pasted expressions, and Calculator error results.
- History collection continues while the panel is hidden; reopening the panel shows calculations performed in the meantime.
- Added `src/calculation_log.rs` regression coverage for newest-first order, bounds, and clearing.


## v1.1.10 buildfix36

- Corrected the buildfix35 History layout regression: removed the combined Calculator+History root client surface entirely.
- Restored Standard and Scientific Calculator panels as direct children of the original Calculator frame, with their original fixed client widths.
- Moved History into a separate undecorated wxDragon `Frame` owned by Calculator and excluded it from the Calculator DPI-fitting path.
- Added move/size synchronization so the History companion stays glued to Calculator's right edge without altering Calculator geometry.
- On Windows the sidecar position and height use realized HWND physical client/outer rectangles, avoiding the logical/physical DPI mismatch that caused the clipped controls in buildfix35.
- Hiding History now hides only the companion frame; the Calculator is not resized.


## v1.1.10 buildfix37

- Changed History docking so its outer top and bottom edges align with the Calculator outer window while its left edge remains glued to Calculator's outer right edge.
- Added clickable History recall. Each successful History record retains its exact `f64` result, and clicking the rendered entry restores that value to the main display without re-evaluating the expression.
- Error-only History records remain visible but cannot be recalled because they have no numeric result.
- History recall participates in Ctrl+Z/Ctrl+Y state history but does not create another calculation-log entry.
- Made the Statistics Box an owned Calculator utility window and docked it immediately to Calculator's left with matching outer top edges.
- Main-window move, size, and activation events now re-synchronize both History and Statistics companion positions.
- Windows History click hit-testing uses the native multiline Edit control character/line position so wrapped or scrolled entries map to the correct stored result and blank space below the final entry does not recall it.


## v1.1.10 buildfix38

- Center Calculator at startup using its final, DPI-fitted native outer rectangle and the nearest monitor work area; wxDragon `centre()` remains the non-Windows/failure fallback.
- Changed the Statistics Box from left-side docking to owner-relative centering. It opens centered over Calculator and stays centered when Calculator moves or changes size.
- Position the Statistics Box before its first `show(true)` call to avoid a transient flash at the toolkit's default top-level position.
- Clamp native Statistics positioning to the Calculator monitor work area so the utility window does not open partly off-screen or behind the taskbar.
- History docking/recall behavior is unchanged.


## v1.1.10 buildfix39

- Fixed companion-window restoration after minimizing and restoring Calculator.
- Removed the explicit `show(false)` calls that converted a transient owner minimization into a persistent hidden state for History and Statistics.
- Added a native Windows `WM_SIZE` state observer that distinguishes `SIZE_MINIMIZED` from a genuine `SIZE_RESTORED`/`SIZE_MAXIMIZED` transition.
- After an actual restore, the app re-synchronizes the complete companion group from application state: History reappears only when **View > History** is enabled, and Statistics reappears only when its window is still open.
- Closed, explicitly hidden, and never-created companion windows are never resurrected by restore.
- Existing wx move/size/activation synchronization remains in place as a cross-platform/fallback path.


## v1.1.10 buildfix40

- Replaced the buildfix36–39 separate History top-level frame with a genuine vertical wxDragon `SplitterWindow` inside the Calculator frame, eliminating the OS border/gap between Calculator and History.
- Added a fixed-layout Calculator host as the left splitter pane and a single History panel as the right pane; Standard/Scientific controls remain at their recovered coordinates and are never reflowed into History.
- Added native sash handling. The non-live classic guide may be dragged to request a wider/narrower History pane; after release the outer frame is resized by the drag delta and the Calculator pane is restored to its canonical width.
- Added persisted `history_width` (120–420 source pixels, default 210) to `OpenCalc.cfg`; `View > History` now splits/unsplits the right pane instead of showing/hiding another frame.
- Kept clickable History recall, newest-first logging, scrolling, Clear, localization, and independent Undo/Redo semantics.
- Removed the obsolete Win32 History-companion positioning helper and all move/restore bookkeeping for History; only the still-separate Statistics Box participates in owned-window restoration.
- Statistics remains centered over the complete main frame and is re-centered when History is shown, hidden, or resized.

## Classic 3-D fidelity — buildfix42

The Windows painter now follows the original `CALC.EXE` drawing primitives
instead of approximating the bevel with manually filled one-pixel strips.
Pushbuttons use USER32 `DrawFrameControl(DFC_BUTTON, DFCS_BUTTONPUSH...)`,
selector/group frames use `DrawEdge(EDGE_ETCHED, BF_RECT)`, recessed display
and status wells use `DrawEdge(EDGE_SUNKEN, BF_RECT)`, and the Scientific
separator uses the recovered `EDGE_ETCHED/BF_BOTTOM` combination.  Control
coordinates and splitter geometry are unchanged.


## Linux classic 3-D fidelity — buildfix43

The Windows path continues to use the recovered USER32 `DrawFrameControl` and
`DrawEdge` primitives.  On Linux, wxDragon 0.9.17 uses wxWidgets' GTK3 backend,
so the same painter entry points now attach one process-wide GTK3 CSS provider
to the native wxGTK controls.  The provider uses the Windows 95 default classic
palette (`#c0c0c0` face, white highlight, `#808080` shadow and black dark
shadow) and the same two-stage edge ordering for raised buttons, pressed
buttons, recessed display/status fields, etched group frames and the Scientific
separator.  Native GTK controls still own their text, pressed/disabled state,
keyboard/mouse handling and accessibility.

Linux calculator surfaces are explicitly set to the classic button-face grey so
the emulated bevels do not inherit an unrelated desktop-theme background.  The
History text well and Statistics buttons also receive the same classic chrome.
No Linux-only Rust GTK crate is added: wxDragon already requires GTK3 on Linux,
and the bridge is compiled only for `target_os = "linux"`; other non-Windows
targets retain the ordinary wxWidgets fallback.


## Statistics application-focus grouping — buildfix44

The Statistics Box remains a separate modeless utility window, because its list and RET/LOAD/CD/CAD controls need independent interaction. Buildfix44 changes activation semantics without making the dialog modal.

On Windows, Calculator and Statistics are treated as one visually active application group. When either top-level window activates, Statistics receives the matching non-client `WM_NCACTIVATE` state. This keeps the Statistics caption visually active when the user clicks Calculator, while the actual keyboard focus remains on the Calculator control/window so typed input, menu accelerators and Ctrl+Z/Ctrl+Y continue working. Moving focus to another application restores the normal inactive Statistics caption.

On Linux/wxGTK, window managers do not expose a portable equivalent for painting two top-level title bars as simultaneously active. The same activation events therefore toggle GTK's keep-above hint for the Statistics utility while Calculator/Statistics is the active application group, and remove it immediately when the application loses focus. No keyboard focus is forcibly redirected on either platform.

## Statistics active-caption guard — buildfix46

Buildfix44 attempted to keep the Statistics Box visually active by sending a
one-shot `WM_NCACTIVATE(TRUE)` when Calculator activated.  That was racy: on a
normal owner click Windows can deliver Statistics' real
`WM_NCACTIVATE(FALSE)` later in the same activation transition and repaint the
caption inactive.

Buildfix46 installs a native `SetWindowSubclass` guard on the Statistics frame.
When Statistics receives `WM_NCACTIVATE(FALSE)`, the guard uses the message's
`lParam`, which Win32 defines as the handle of the window that is about to be
activated.  If that handle is the Calculator owner, the guard forwards the
message to the default procedure as `WM_NCACTIVATE(TRUE)` instead.  When focus
is leaving for another application, `lParam` is another HWND or may be NULL, so
the ordinary inactive state is allowed through.  The guard removes itself on
`WM_NCDESTROY`.

This is still visual application-group activation, not keyboard-focus theft:
Calculator controls, shortcuts and RET continue to work normally while the
Statistics caption remains active whenever the Calculator application is
foreground.

## Visible History separator — buildfix47

The native History splitter remains gapless and fully draggable, but its sash
could visually disappear because Calculator, History and the splitter all use
the same classic button-face background. Buildfix47 adds a dedicated two-pixel
etched vertical rule immediately inside the History pane. On Windows it uses
USER32 `DrawEdge(EDGE_ETCHED, BF_LEFT)`; on Linux the existing GTK3 classic
provider supplies the matching shadow/highlight pair. The rule is decorative
(`HTTRANSPARENT` on Windows), follows History pane height/DPI changes, and does
not alter the persisted splitter width or recovered Calculator geometry.


## Stronger classic pushbutton bevel — buildfix48

The previous button painter switched the controls to a classic owner-drawn path, but in practice the relief still looked too subtle compared with the original Windows 95 Scientific calculator. Buildfix48 replaces the plain `DrawFrameControl(DFC_BUTTON, DFCS_BUTTONPUSH)` outline with an explicit two-level Win95-style bevel: bright top/left highlight, lighter inner highlight, darker inner bottom/right edge, and a distinct darker outer lower/right shadow. Unpressed buttons now keep a visible lifted face with a stronger lower-right drop, while pressed buttons invert the bevel and keep the one-pixel label offset.

The Linux classic-theme CSS path is updated in parallel so wxGTK buttons also gain a more pronounced outer lower-right shadow instead of the earlier flatter border. Text colours, button sizes, layouts, keyboard behaviour, and non-button controls remain unchanged.


## Joined History boundary line — buildfix49

Buildfix47 introduced a decorative etched rule inside the History pane so the separator between Calculator and History remained visible, but in practice the rule began after the native sash and left a small break where it should visually meet the surrounding etched lines. Buildfix49 keeps the splitter fully native and draggable, but reparents the decorative rule from the History pane to the `wxSplitterWindow` itself and positions it on the sash boundary. The result is that the vertical etched line now visually joins the adjacent horizontal etched lines instead of beginning a few pixels too far to the right.

The rule remains mouse-transparent on Windows, follows sash/DPI changes, and is still hidden whenever History is hidden. Calculator geometry, persisted `history_width`, and the actual splitter drag behavior are unchanged.


## DPI-scaled Windows 95 button relief — buildfix50

Buildfix48 changed the Windows button drawing code, but its bevel strips were still literal one-pixel device offsets. On a 200%-scale display the buttons doubled in physical size while their bevel did not, so the apparent relief remained almost as thin as before. Its lower-right “shadow” was also painted as additional lines within the face instead of being reserved as a separate projected region, and its colours came from the active Windows theme.

Buildfix50 corrects all three causes. The Windows painter queries `GetDpiForWindow` on every paint and scales the two bevel levels and lower-right drop from 96-DPI source metrics. The raised face is shortened by the scaled drop amount so the shadow occupies its own lower/right portion of the button client, rather than replacing pixels inside the face. The five chrome colours are fixed to the recovered classic palette: `#C0C0C0`, `#FFFFFF`, `#DFDFDF`, `#808080`, and `#000000`. The brushes are cached for process lifetime, avoiding GDI-object creation on every repaint.

At 200% DPI, each classic one-pixel bevel level is two physical pixels and the original two-pixel lower-right projection is four physical pixels. Pressed buttons remove the projected shadow, draw the inverse sunken edge over the full client, and move the label by one DPI-scaled source pixel. Button text colours, command wiring, sizes, and layout coordinates are unchanged.


## OpenCalc identity, resources, Linux Help, and Statistics activation — buildfix51

- Renamed the Cargo package identity to `opencalc` and the binary target/root release file to `OpenCalc` / `OpenCalc.exe`. The localized top-level caption deliberately remains `Calculator`, `Calculadora`, or the corresponding language-specific title.
- Added `build.rs` with `winresource` so `calc95.ico`, `FileDescription`, `ProductName`, `InternalName`, and `OriginalFilename` are linked into the Windows executable resource table. The existing runtime icon assignment remains for the live Calculator and Statistics title bars.
- Renamed the primary preference file to `OpenCalc.cfg` while retaining read migration from `Calculator95-Rust.cfg`.
- Split HLP-viewer discovery by platform: Windows selects `hlp-viewer.exe`; Linux requires a native extensionless `hlp-viewer` beside the running OpenCalc executable.
- Fixed Statistics focus rather than merely repainting its caption. While Statistics is the foreground window, the Calculator owner handles `WM_MOUSEACTIVATE` with `MA_NOACTIVATE`: the pending mouse click is still delivered, but Calculator does not take activation from Statistics. The existing RET button explicitly focuses Calculator, and clicking the application from another program still activates normally because the guard applies only while Statistics itself is foreground.


## Native face with DPI-scaled borders — buildfix52

Buildfix50 overreached: in addition to scaling the bevel, it replaced the entire Windows button face with fixed `#C0C0C0` and reserved a large fixed-palette lower/right projection inside the control. On modern Windows this visibly darkened every button and made the shadow much harsher than the requested change.

Buildfix52 reverts that colour change. The button face and the text background now come from the current Windows `COLOR_BTNFACE`, exactly as before the failed fixed-palette change. The highlight, light, shadow, and dark-shadow bands likewise use the corresponding Windows system 3-D colours. Only the edge geometry remains custom: an outer and inner bevel layer are each scaled from one 96-DPI source pixel using `GetDpiForWindow`. Thus the total bevel is two physical pixels at 100%, three to four pixels at intermediate/high scaling as rounding requires, and four physical pixels at 200%, without changing the button fill or adding a separate black drop region.

Pressed buttons invert the same two scaled layers and retain the DPI-scaled text displacement. Labels, command events, sizes, layout, and non-button controls are unchanged.


## History separator alignment — buildfix53

The buildfix49 decorative rule was positioned entirely to the left of the native sash coordinate. On the reported high-DPI layout this made the rule appear a few pixels too far into the Calculator pane. Buildfix53 centres the existing DPI-scaled rule over the sash instead. The adjustment is half of the rendered separator width: one physical pixel at 100% DPI and approximately two physical pixels at 200% DPI.

Only the decorative rule moves. The actual `wxSplitterWindow` sash, Calculator width, History width, persisted `history_width`, drag target, and child-control layout remain unchanged.


## Decimal-separator synchronization in History — buildfix54

Calculation History formerly stored the already-localized expression and result strings. Changing **View → Decimal separator** therefore updated the display and keypad but left entries already present in History using their old punctuation.

Buildfix54 stores numeric punctuation in the calculation log with invariant `.` notation and localizes it only when the History pane is rendered. Switching between period and comma now immediately redraws every existing numeric expression and numeric result with the selected separator. The conversion is token-aware: only punctuation between digits is treated as a decimal mark, so function punctuation, argument separators, ordinary prose, and periods in localized error messages are preserved. Exact `f64` recall values remain unchanged, and no calculation is re-evaluated.


## Native Statistics ownership and retained activation — buildfix55

The earlier Windows fix relied on a `WM_MOUSEACTIVATE` subclass attached only to Calculator's top-level frame and assumed that `wxFrame::SetFocus` had made the Statistics top-level HWND the real foreground window. That assumption was not reliable on wxMSW: mouse activation may be resolved through the clicked child HWND, and focusing a wx frame or descendant does not necessarily establish the top-level foreground/active state checked by the guard. Consequently, Calculator could still activate and the Statistics Box could fall behind or repaint inactive.

Buildfix55 makes the relationship explicit at the Win32 level. The Statistics HWND receives Calculator through `GWLP_HWNDPARENT`, making it a true owned top-level window that Windows keeps above its owner without using global `WS_EX_TOPMOST`. The `MA_NOACTIVATE` guard is installed on Calculator and all current descendant HWNDs, covering buttons, panels, selectors, text controls, and the top-level menu/non-client path. When Statistics is genuinely active, Calculator mouse messages are still delivered but cannot transfer activation. Opening Statistics now explicitly calls `SetForegroundWindow` and `SetActiveWindow`, rather than relying on wx focus alone.

The companion's destruction removes the guard from Calculator and every descendant, preventing stale HWND references when the Statistics Box is closed and reopened. RET remains the intentional control for transferring focus back to Calculator. Statistics stays above Calculator only; it does not float over unrelated applications.


## Bundled Windows Help set — buildfix56

Buildfix56 replaces the previous HLP Viewer companion with the newly supplied `hlp-viewer.exe` and adds the supplied Help files under their canonical runtime names: `CALC.HLP` and `CALC.CNT`. The uploaded HLP carried the source filename `CALC(4).HLP`; it is intentionally packaged as `CALC.HLP`, matching both OpenCalc's launcher and the `:Base CALC.HLP` directive in the supplied CNT file.

All three files live in the project root, so they are automatically beside the root `OpenCalc.exe` produced by `build.bat`. The build script also copies all three into `target\release`, beside Cargo's final `OpenCalc.exe`. `CALC.CNT` is not passed on the command line: the viewer discovers it by basename when it opens `CALC.HLP`, restoring the complete hierarchical Contents view.


## One-time Statistics centering — buildfix57

The Statistics Box was still being recentered after the buildfix55 activation fix because `sync_stats_box()` combined two unrelated responsibilities: restoring/showing the owned window and positioning it relative to Calculator. Calculator activation, movement, resizing, Standard/Scientific changes, History visibility changes, and sash normalization all called that function, so a simple click could place Statistics back in the center.

Buildfix57 separates those operations. `center_stats_box()` is called only after a fresh Statistics window is constructed. Closing Statistics removes the cached window, so opening it again constructs a new box and centers that new instance relative to the current Calculator rectangle. Once the box is open, its screen position is preserved across Calculator clicks, activation changes, movement, resizing, mode changes, History changes, undo/redo restoration, and splitter adjustments.

Minimizing Calculator still temporarily suppresses its owned Statistics window. Restore now calls a visibility-only helper that shows the existing box again without modifying its coordinates. The buildfix55 native ownership, retained activation, z-order, and RET behavior are unchanged.


## Optional function graphing — buildfix58

Buildfix58 adds a native graph pane to the **left** of the fixed Calculator surface. It is disabled by default and can be shown or hidden with **View → Graph**. The choice is written as `graph_visible=true` or `graph_visible=false` in `OpenCalc.cfg`; existing configuration files that do not contain the key retain the disabled default. Graph and History remain independently selectable, and neither pane stretches or reflows the recovered Standard or Scientific Calculator controls.

The graph field accepts a bare expression such as `x^2 - 4`, an explicit function such as `y = sin(x)` or `f(x) = sin(x)`, or a single equation such as `x^2 = 4`. Equations are normalized to a zero-finding expression, and graph evaluation reuses OpenCalc's existing expression language, decimal-separator rules, and Deg/Rad/Grad mode. Pressing Enter in the field is equivalent to pressing **Plot**.

The rectangular native canvas renders axes, grid lines, the current function, and real roots found within the visible x-range. Roots are listed immediately below the canvas and marked on the x-axis. Numerical detection combines sign-change bracketing with a tangent-root refinement path, while non-finite samples and large discontinuity jumps split the curve so asymptotes are not deliberately connected. Mouse dragging pans the viewport, the wheel zooms around the pointer, and **Reset view** restores the default `x = -10..10` range with automatic y-ranging.

**Export** sits below the canvas and writes the current visible graph as PNG, JPG/JPEG, or SVG. Buildfix60 exports a complete presentation page rather than a raw canvas dump: the top heading formats common notation (`x^2` as `x²`, multiplication as `·`, `pi` as `π`, and `sqrt` as `√`), the central chart keeps uncluttered root markers, and a separate localized footer lists the roots and visible x/y ranges. Raster output is enlarged to a readable minimum page size; SVG preserves the same layout as vector drawing commands.


## Concise action labels — buildfix59

Buildfix59 shortens the History command from **Clear history** to **Clear** and removes the ellipsis from the graph pane's **Export** button. Portuguese now uses **Limpar** and **Exportar**; Spanish uses **Borrar** and **Exportar**. The underlying clear-history and graph-export commands, file dialog, localization refresh, button sizes, and pane layouts are unchanged.


## Presentation graph export and localized manuals — buildfix60

Buildfix60 retains the existing English `CALC.HLP`/`CALC.CNT` pair unchanged and adds complete Portuguese (`CALP.HLP`/`CALP.CNT`) and Spanish (`CALS.HLP`/`CALS.CNT`) variants. The four-character localized stems remain compatible with the WinHelp internal CNT filename field. Each localized HLP preserves the English manual's 41-topic structure, symbolic contexts, embedded screenshots, hyperlinks, and full copy/paste grammar documentation while translating the visible topic text and Contents hierarchy.

The Help command reads the current `Language` preference at invocation time. English opens `CALC.HLP`, Portuguese opens `CALP.HLP`, and Spanish opens `CALS.HLP`; localized searches retain English as a compatibility fallback. `build.bat` copies all six Help files beside `target\release\OpenCalc.exe`.

Graph export now separates information from the plotting surface. Common calculator notation is formatted into a readable function heading, roots are marked without overlapping numeric labels on the curve, and the localized roots sentence appears in a dedicated footer together with the visible coordinate ranges. PNG, JPG/JPEG, and SVG share the same composition.


## Complete paste and graph syntax documentation — buildfix61

Buildfix61 expands the English, Portuguese, and Spanish WinHelp manuals so their expression-language reference matches the syntax accepted by the current parser and graph field.

The manuals document the directly renderable Unicode multiplication and division aliases `×` and `÷`. The parser also accepts the Unicode minus character, but that alias is intentionally omitted from the legacy WinHelp text because its glyph is not rendered reliably by the HLP code-page path; the manuals instruct users to use ASCII `-`. They also state that ASCII identifiers are case-insensitive, covering function names, constants, word operators, and uppercase/lowercase based-literal prefixes. Examples include `SIN(30)`, `Factorial(6)`, `PI`, `0XFF`, `0B1010`, and `0O17`; lowercase and uppercase Greek pi are accepted as well.

The documented precedence is the parser's actual order, from strongest to weakest:

1. postfix factorial `!` and percent `%`;
2. unary `+` and `-`;
3. right-associative exponentiation/root operators `^`, `**`, and `root`;
4. `lsh`;
5. multiplication, division, and `mod`;
6. addition and subtraction;
7. `and`, `xor`, and `or` at one shared left-associative level.

This explicitly records the existing unary-before-power behavior: `-2^2` evaluates as `4`, while `-(2^2)` evaluates as `-4`. The manuals also include `2^3! = 64` and right-associative `2^3^2 = 512` examples.

Number documentation now includes leading-decimal forms such as `.5` and `,5`, in addition to grouped numbers, scientific notation, and case-insensitive `0x`/`0X`, `0o`/`0O`, and `0b`/`0B` prefixes.

The former pasted-expression examples topic is now **Paste and Graph Examples**. It explains that the graph field enables the otherwise graph-only variable `x` and accepts a bare expression, `y = expression`, `f(x) = expression`, or one general equation. A general equation is normalized as left side minus right side, so the roots displayed in the visible range solve that equation. Ordinary Calculator paste still rejects `x`, and the graph field accepts only one function or one equation at a time.


## Help directory and block-safe localized manuals — buildfix62

Buildfix62 moves the runtime manuals into the `Help` directory and adopts explicit language names: `CALC_EN`, `CALC_PT-BR`, and `CALC_ES`. The HLP `|SYSTEM` contents filename and each CNT `:Base` directive match the new filenames. OpenCalc reads the selected interface language at Help invocation time and resolves the matching file under `Help`; Portuguese and Spanish retain the English fallback.

The localized HLP writer was also corrected. A TOPICLINK payload may span transformed `|TOPIC` blocks, but its fixed 21-byte header may not. The earlier translation repacker could place a header in the final bytes of a 4,084-byte transformed block; the Portuguese manual exposed this at TOPICPOS 8162. Buildfix62 inserts compiler-style padding before such headers, rewrites the linked-record positions, and regenerates every physical TOPICBLOCKHEADER from the final chain. The same structural repair is applied to all three manuals while preserving the English visible manual text except for the Help-file naming documentation.


## Buildfix63-64 manual cleanup (superseded by buildfix66)

Buildfix63 first reorganized the precedence examples, and buildfix64 corrected the Portuguese **Copiar e colar expressões** terminology. Buildfix66 replaces those intermediate compiled manuals with a format-aware revision containing a real WinHelp table and actual inline font runs for emphasis.

## Original-style focused keyboard input — buildfix65

Buildfix65 restores the original Calculator's keyboard-first interaction model.
A direct audit of the Windows 95 `SA` accelerator resource and its main message
loop shows that digit, arithmetic, scientific, memory, statistics, radix,
angle, editing, and clipboard shortcuts are translated at the top-level
Calculator rather than requiring focus on a particular button. OpenCalc now
routes the corresponding keystrokes through the same `Action` path used by its
native wxDragon controls whenever the Calculator surface or one of its
Scientific selectors has focus. The graph expression editor is excluded so it
continues to behave as a normal text field.

The focused Calculator now accepts the original shortcuts such as `r` for
reciprocal, `s`/`o`/`t` for trigonometric functions, `y` for x^y, `p` for PI,
`@` for sqrt/x^2, `!` for factorial, the logic punctuation keys, the documented
Ctrl memory/statistics combinations, keypad aliases, and the classic
Backspace/Delete/Escape/F-key behavior. Clicking a Calculator pushbutton
returns focus to Calculator so subsequent keyboard input continues naturally.

OpenCalc additionally accepts keyboard `**` as exponentiation. A single `*`
remains multiplication; only a second consecutive ASCII star converts that
pending operation to power. This does not replace the recovered Scientific
`^` shortcut, which remains Xor, and does not change numeric-keypad `*` or
Unicode `×` multiplication.


## Blank graph input and rebuilt formatted Help — buildfix66

The graph pane now opens with an empty **Function** field instead of pre-filling `x^2 - 4`. `GraphModel::default()` is likewise blank and remains in `NotPlotted` state until the user enters a function and plots it. A regression test protects that startup state.

All six files in `Help/` were replaced with the current localized Help revision rather than leaving the older compiled HLPs in the package. The Portuguese Contents chapter is **Copiar e colar expressões**. The paste-syntax topic no longer prints a code-point label for Unicode minus; it documents `×` and `÷` and uses ordinary ASCII `-` in visible Help text.

The precedence examples are now a real two-column WinHelp type-`0x23` table with localized **Expression/Result** headings and separate rows for `2^3!`, `2^3^2`, `-2^2`, and `-(2^2)`. This replaces the earlier pipe-delimited approximation.

The HLP font table now carries dedicated bold body and bold monospace descriptors. References to **Standard Mode/Scientific Mode** and their Portuguese/Spanish equivalents use actual bold font runs in ordinary body text; mathematical expressions and code examples use bold body/monospace runs as appropriate. Existing title/heading bold formatting is retained.

The Keyboard Shortcuts topics were also brought forward to buildfix65 behavior: with Calculator focused, direct keys operate the corresponding calculator actions, the classic one-letter scientific shortcuts are summarized, Ctrl+Insert/Shift+Insert are documented, and keyboard `**` is explicitly identified as exponentiation while a single `*` remains multiplication.


## Structured WinHelp reference layout — buildfix67

Buildfix67 replaces the remaining prose-like reference formatting in the packaged English, Portuguese, and Spanish HLP manuals with native WinHelp structure. The pasted-expression syntax topic now uses a bordered operator-group table and a separate bordered precedence table rather than a long inline operator sentence. The introductory rule and the unary/final-equals rule are presented as bordered callout boxes.

The same presentation policy is applied to the other reference-heavy topics: paste examples use a bordered table; graph-field syntax is grouped in a bordered block; direct keyboard input uses a compact table; calculator error messages use a message/meaning table; the Standard button reference is a button/action table; numeric/function examples and the Scientific button matrix receive bordered grouping where a full table would be unnecessarily dense. Narrative topics remain ordinary paragraphs.

The Help rewrite keeps all 41 TopicHeader TOPICPOS values fixed. The `|CONTEXT` and `|TopicId` streams are unchanged, while the moved display records are relinked and each TopicHeader's internal scrolling/non-scrolling offsets are remapped. This preserves authored Contents/context destinations while allowing the display records inside a topic to be packed more neatly.


## Top-level keyboard focus and key feedback — buildfix68

Buildfix68 fixes the remaining difference between OpenCalc's buildfix65 keyboard routing and the original Windows 95 Calculator. The original `TranslateAcceleratorA` path is active as soon as the Calculator top-level window is active; it does not require a calculator child control to acquire focus first. OpenCalc now explicitly gives the Calculator frame keyboard focus immediately after the realized window is shown and again when the Calculator top-level window is reactivated. The graph expression editor is deliberately exempt so text entry there remains normal.

Keyboard-triggered calculator actions now also pulse the matching visible pushbutton. On Windows the classic button subclass uses `BM_SETSTATE` and a short non-blocking timer, so its existing DPI-aware pressed bevel is drawn for approximately 85 ms. Key repeat re-arms the timer. `KeyboardStar` visually maps to the multiplication button; all ordinary digit, arithmetic, editing, memory, statistics, and scientific shortcuts map to their matching visible control when one exists.


## Two-column keyboard reference — buildfix69

Buildfix69 corrects the keyboard-reference presentation in all three WinHelp manuals. The Direct/Calculator Keys block is now a genuine two-column WinHelp table: **Key / Shortcut** in English, **Tecla / Atalho** in Portuguese, and **Tecla / Atajo** in Spanish. Key spellings are kept in the left column and their calculator action is kept in the right column instead of embedding both halves into one prose cell. The surrounding menu-shortcut wording was compacted slightly so the improved table fits without moving any TopicHeader or changing any CNT/context destination.


## Help-directory replacement — buildfix70

Buildfix70 republishes all six localized WinHelp files from the current buildfix69 manual set and changes `build.bat` to delete `target\release\Help` recursively before recreating it. This prevents stale HLP/CNT files from an older build from surviving when a release is rebuilt. The source package itself also contains only the six canonical Help files: `CALC_EN.HLP/.CNT`, `CALC_PT-BR.HLP/.CNT`, and `CALC_ES.HLP/.CNT`.


## One-entry-per-row Help lookup tables — buildfix71

Buildfix71 replaces the grouped keyboard and operator summaries with true lookup tables. Every operator or key spelling occupies its own left-hand cell, and the corresponding behavior is written as a complete description in the right-hand cell. For example, `!` is paired with a factorial explanation, `**` with exponentiation, and `Backspace` with deletion of the last entered digit.

The pasted-expression syntax topic now uses an **Operator / Description** table with separate rows for `+`, `-`, `*`, `×`, `/`, `÷`, `^`, `**`, `%`, `!`, `mod`, `and`, `or`, `xor`, `lsh`, `root`, parentheses, and the final equals sign. The keyboard topic is split into three readable **Key / Description** tables: basic/editing keys, scientific keys, and function/control shortcuts. Portuguese uses **Operador / Descrição** and **Tecla / Descrição**; Spanish uses **Operador / Descripción** and **Tecla / Descripción**.

`tools/rebuild_help_reference_tables.py` records the format-aware HLP migration used for this release. It repacks `|TOPIC`, remaps internal TOPICOFFSET destinations, preserves all 41 context/topic destinations, and is safe to run again on an already migrated buildfix71 Help directory.

## Buildfix72 graph-input repair

The graph field now accepts ordinary graphing-calculator notation in addition to the shared expression grammar. Implicit multiplication is inserted only for graph input, so `2x`, `2(x+1)`, `(x+1)(x-1)`, and `2sqrt(x)` are normalized before parsing. Unicode superscript runs are converted to powers, allowing `x²`, `x³`, and multi-digit superscripts. Caret and double-star notation remain accepted, so `2x² + 2x + 2`, `2x^2 + 2x + 2`, and `2x**2 + 2x + 2` produce the same graph.

A failed plot attempt now clears the previous compiled graph before reporting the new error, preventing an old caption or curve from remaining visible beside an unrelated error. The roots/status area reserves three lines, wraps long messages, and restores its fixed pane rectangle after each label change so wxStaticText auto-sizing cannot extend the message beyond the graph pane.

The English, Brazilian Portuguese, and Spanish HLP manuals have also been repacked to document implicit multiplication, Unicode superscripts, and both `^` and `**` power notation in the graph field.


## Buildfix73 help-table alignment cleanup

The packaged English, Portuguese, and Spanish WinHelp manuals now use shorter one-line descriptions in the operator and keyboard reference tables, and the table-generation script was updated to reserve more width for the description column. This avoids the row-wrap behavior that made later operator rows appear visually misaligned in the viewer.


## Buildfix74 padded Help tables

The packaged WinHelp manuals were rebuilt again to keep operator and key reference tables visually aligned in the WinHelp viewer. Description strings were shortened and the table generator now allocates a narrower left column and a wider description column so table rows remain padded and do not desynchronize when rendered.


## Buildfix75 aligned keyboard tables

The English, Portuguese, and Spanish WinHelp keyboard-reference tables were fully regenerated rather than text-patched in place. Each key now occupies its own row, every cell includes explicit nonbreaking-space padding, descriptions are intentionally kept to one line, and each keyboard subsection uses column widths appropriate to its key labels. This prevents wrapped right-hand cells from visually pulling later left-hand labels out of alignment.


## Buildfix76 row-safe WinHelp tables

The operator and keyboard reference grids are no longer encoded as one multi-row WinHelp table record. Classic WinHelp tables are independent vertical column flows, so a wrapped key or description shifts only its own column and causes the remaining entries to drift. Buildfix76 emits one two-cell type-0x23 table record per visible row. Each record advances by the taller of its two cells, keeping the next key and description aligned even when either side wraps. The cells retain nonbreaking-space padding and table borders.


## Linux separator and group-frame cleanup — buildfix85

The Linux History boundary no longer exposes wxGTK's desktop-themed sash as a warm grey strip. OpenCalc now places its own neutral, etched separator directly over a thin native sash and uses that visible handle for History resizing, preserving the fixed Calculator geometry and persisted pane width. This is confined to the Linux frontend; Windows keeps its existing native splitter and DPI-aware decoration path.

The empty Scientific selector and command frames on Linux now use a single one-pixel etched border. The previous GTK CSS added a second inset black band to the upper and left edges, making the top line visibly thicker than the lower and right edges at high scaling.
