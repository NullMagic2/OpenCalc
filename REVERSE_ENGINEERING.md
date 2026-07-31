# Reverse-engineering notes for the supplied CALC.EXE

## Identity

* PE32 / i386 GUI executable
* Image base `0x00400000`
* Entry point `0x0040534E`
* PE timestamp: 1995-06-30 01:31:07 UTC
* SHA-256: `b064b0ac430264eff7b79b91e743bcd36d7b3707857f5bcdc4db146911dd0e28`

## Clipboard architecture

The executable imports `OpenClipboard`, `GetClipboardData`, `CloseClipboard`, `GlobalLock`, `GlobalUnlock`, and `SendMessageA`.

The paste loop around `0x403D51` reads clipboard text one byte at a time. After special cases, it searches a translation table at approximately `0x40C458`. The table explicitly includes digits, arithmetic operators, parentheses, both decimal separators, and hexadecimal/scientific keyboard commands. At approximately `0x403E24`, each translated byte is sent back to the Calculator window as `WM_COMMAND`. Clipboard paste is therefore a command macro front end rather than a conventional expression grammar.

### Unary-minus defect

Around `0x403D6F`, a `-` at the beginning of pasted input is specially translated to the sign-change command (`0x50`). There is another exponent-sign special case around `0x403D97`. A `-` after an ordinary binary operator does not receive equivalent lexical treatment. The command engine treats consecutive binary operators as operator replacement, so `2*-3` is not parsed as multiplication by a negative operand.

The Rust version replaces this architecture with a real expression parser rather than adding another special case.

## Resources

The resource directory contains icons 1/2, menu `SM`, dialogs `SB` (Statistics Box) and `SC` (`SciCalc`), accelerator table `SA`, string tables, a group icon, and version information.

`SM` contains Edit → Copy/Paste, View → Scientific/Standard, and Help → Help Topics/About Calculator.

The `SC` dialog identifies itself as class `SciCalc`, uses 8-point `MS Sans Serif`, and contains the selector/status/display controls while the main calculator keys are largely handled separately by Calculator code.

Relevant `SC` controls recovered from the dialog template include:

```text
Hex  x=13  y=38  w=35 h=10
Dec  x=54  y=38  w=35 h=10
Oct  x=95  y=38  w=35 h=10
Bin  x=134 y=38  w=35 h=10
Inv  x=14  y=58  w=34 h=10
Hyp  x=54  y=58  w=34 h=10
angle radio IDs 127/128/129 at x=190/234/280, y=38
status ID 401 at x=136,y=58
status ID 403 at x=98,y=58
main display ID 413 at x=169,y=14,w=149,h=10
```

ID 403 is updated from the internal parenthesis depth. The handler builds the visible prefix from the literal `(=` and appends the decimal depth. IDs 401/402 are the scientific/standard memory indicators; the executable contains the corresponding ` M` status text.

These details explain a visual mistake in the first wxDragon conversion: the small status fields were incorrectly represented as a second top `TextCtrl` instead of being placed in the selector/status band.

## Literal paste translation table recovered at 0x40C458

The first ASCII-facing entries include:

```text
A->A B->B C->C D->D E->E F->F
0->0 1->1 2->2 3->3 4->4 5->5 6->6 7->7 8->8 9->9
/->Z *->[ %->^ -->] =->o +->\\
(->( )->) .->U ,->U
```

The right-hand values are internal Calculator command/key codes, not printable output. Parentheses are deliberately supported by the paste path.

## Scientific visual reconstruction — buildfix3

The supplied side-by-side reference makes the original visual matrix unambiguous. At the user's current DPI the reference is almost exactly a 2× rendering of a 500-pixel-wide logical client area. Halving the measured reference coordinates aligns with the resource-derived selector/status positions.

Buildfix3 therefore uses:

```text
Display       x=248 y=10  w=236 h=24
Radix frame   x=13  y=49  w=258 h=27
Angle frame   x=278 y=49  w=206 h=27
Inv/Hyp frame x=13  y=83  w=126 h=27
Paren well    x=145 y=83  w=35  h=27
Memory well   x=197 y=83  w=35  h=27
Back          x=330 y=83  w=48  h=27
CE            x=383 y=83  w=48  h=27
C             x=436 y=83  w=48  h=27
```

The keypad starts at `y=116` and is five rows high:

```text
Sta   F-E   (     )     MC   7    8    9    /    Mod   And
Ave   dms   Exp   ln    MR   4    5    6    *    Or    Xor
Sum   sin   x^y   log   MS   1    2    3    -    Lsh   Not
s     cos   x^3   n!    M+   0    +/-  .    +    =     Int
Dat   tan   x^2   1/x   PI   A    B    C    D    E     F
```

The previous buildfix2 interpretation was wrong: it transposed the four scientific-function columns into a 4×5 block and then invented separate bottom rows for logic, hexadecimal digits and Dword/Word/Byte. Those extra rows are not present in the reference layout and have been removed.

Pixel sampling of the supplied reference screenshot confirms the three key text colors as exact `RGB(255,0,0)`, `RGB(0,0,255)`, and `RGB(128,0,128)`.

The interface continues to use real wxDragon `Button`, `TextCtrl`, `CheckBox`, `RadioButton`, `StaticBox`, `StaticText`, `Menu`, `MenuBar`, `Panel`, and `Frame` objects. On Windows, `SetWindowTheme(control, L"", L"")` is applied to those controls so current Windows does not substitute rounded modern button chrome for the classic square/bevelled form. This changes rendering policy only; wxDragon/wxWidgets still owns every control and event.

## Help integration

The reference menu contains Help Topics, but the HLP data is external. OpenCalc resolves a platform-native companion (`hlp-viewer.exe` on Windows, extensionless `hlp-viewer` beside the binary on Linux) and then selects `Help/CALC_EN.HLP`, `Help/CALC_PT-BR.HLP`, or `Help/CALC_ES.HLP` from the current interface language. No WinHelp parser is duplicated in Calculator.

## wxDragon presentation correction — buildfix4

The recovered Calculator coordinates remain the source of truth at 96 DPI, but the modern wxDragon front end now applies a uniform 120-DPI design scale (125%). This is a presentation scale only; it does not change calculator behavior or the recovered control ordering.

The original `SC` resource requested bitmap-era `MS Sans Serif`. For modern rendering the front end aliases this to the TrueType `Microsoft Sans Serif` face at 9 points, preserving the family style while allowing normal Windows font smoothing. Pushbutton text remains bold.

Buildfix3's combination of recoloured wxButtons and per-button theme suppression could leave only the label visible on current Windows/wxWidgets. Buildfix4 keeps each wxDragon `Button` as the actual native input control but subclasses only its Windows paint path: the face is filled with `COLOR_BTNFACE`, `DrawEdge` supplies raised/sunken system 3-D edges, and the native `BM_GETSTATE/BST_PUSHED` state selects the pressed appearance. Input, accessibility ownership, button notifications, and wxDragon callbacks remain on the real control.


## Buildfix7 DPI realization note

The wxDragon child geometry is authored in design/logical units, but on wxMSW with Per-Monitor-V2 awareness the underlying child HWNDs are physically scaled when the top-level window is realized. Measuring the panel before `Show()` therefore cannot be used to infer the final physical client extent on a high-DPI monitor. Buildfix7 performs top-level sizing only after realization and uses native `GetDpiForWindow`, `GetClientRect`, `GetWindowRect`, and `SetWindowPos` so the frame and active panel are sized in the same physical coordinate space as their child HWNDs.

## Win95 control context help / “What's This?”

The supplied `CALC.EXE` imports both `WinHelpA` and `TrackPopupMenuEx`, matching the observed Windows 95 interaction: right-clicking a calculator control opens a small context menu containing **What's This?**, and choosing that command opens the control's context-help popup.

The supplied `CALC.HLP` contains a contiguous context-help topic run beginning at TOPICPOS 19780. Decoding it with the same WinHelp container/topic/phrase rules used by Rust HLP Viewer recovers control-specific strings for `C`, `CE`, `Back`, `1/x`, memory controls, display/digits, arithmetic operators, radix and angle selectors, `Inv`, `Hyp`, statistics, scientific functions, parentheses, bitwise operations, and hexadecimal A-F input. The HLP used for extraction has SHA-256 `25ebb11326406561f210c96ad03c7b35d3fc6cf0deee3958efc699e5bec47103`.

Buildfix9 intentionally does not link the Calculator to the HLP parser. Those decoded strings are retained in editable UTF-8 `calc.tooltip`, keyed by semantic control names. The wxDragon controls receive native Windows `WM_CONTEXTMENU` subclasses. The subclass uses a real popup menu for **What's This?** and a tracking `tooltips_class32` control for the pale-yellow context popup. This preserves the original interaction model while keeping full Help/F1 delegated to the platform-native HLP viewer (`hlp-viewer.exe CALC.HLP` on Windows; `hlp-viewer CALC.HLP` on Linux).

Buildfix97 gives Linux the same explicit right-click → **What's This?** → popup interaction without emulating Win32 or adding a second GUI toolkit abstraction. The wxGTK handles are native `GtkWidget` pointers. One reusable `GtkMenu` supplies the localized command, and one temporary undecorated `GTK_WINDOW_POPUP` renders the selected catalog text with the recovered pale-yellow/black presentation. Ordinary controls receive one button-event signal; parent panels perform a small rectangle hit test only for no-window GTK labels. The popup is destroyed on Calculator clicks, Escape, deactivation, menu operations, or direct clicks on the popup.

## Locale decimal handling — buildfix10

The recovered clipboard character table in the Windows 95 reference maps both `.` and `,` to the calculator's decimal-point command. The Rust UI therefore accepts both punctuation characters as decimal input instead of binding parsing to one hard-coded symbol.

The visible decimal symbol is a presentation concern. Buildfix10 keeps the calculation state and expression grammar canonical (`.` internally), then translates the displayed radix to the operating system's current non-monetary numeric locale. Windows uses the current user's NLS `LOCALE_SDECIMAL` / `LOCALE_STHOUSAND`; Linux uses `LC_NUMERIC` and `localeconv()`. This prevents a comma-formatted display from leaking locale punctuation into arithmetic state while retaining the reference binary's permissive comma/period input behavior.


## Buildfix11: explicit decimal-entry state and context-help correction

The classic display always appends its decimal separator to an integer (`3.` or `3,`). That means display text alone cannot distinguish an untouched integer from the state immediately after the user explicitly presses the decimal key. Buildfix10 attempted to reconstruct the entry from display text and stripped the trailing separator, losing the user's decimal action. Buildfix11 retains an explicit `decimal_entered` state bit while keeping the canonical internal radix as `.`.

The buildfix9 tracking tooltip also violated the Win32 `TTF_IDISHWND` contract by setting both `TOOLINFO.hwnd` and `uId` to the child control. Buildfix11 follows the common-control model used by our HLP viewer: the containing/parent HWND is stored in `hwnd`, while `uId` is the child HWND.

## Error dispatcher audit — buildfix32

A second pass over the supplied 59,392-byte `CALC.EXE` traced the actual error dispatcher instead of inferring messages from modern floating-point results.

The executable's `RT_STRING` resources contain these calculator-runtime errors:

| Resource | Original English text | Reimplementation constant |
|---:|---|---|
| 67 | `Cannot divide by zero.` | `DIVIDE_BY_ZERO` |
| 68 | `Invalid input for function.` | `INVALID_FUNCTION_INPUT` |
| 69 | `Result of function is undefined.` | `FUNCTION_UNDEFINED` |
| 70 | `Result is too large.` | `RESULT_TOO_LARGE` |
| 71 | `Result is too small.` | `RESULT_TOO_SMALL` |

The central routine at virtual address **0x00404B13** receives an index 0..4 and selects the corresponding pointer starting at `0x0040B5AC`, so the mapping above is direct rather than heuristic. The C-runtime math-error callback at **0x00404B5A** maps DOMAIN (1) to error 1, OVERFLOW (3) to error 3, UNDERFLOW (4) to error 4, and the remaining math exceptions to error 2. A second internal-error adapter at **0x00404B7F** maps internal code `0x83` to divide-by-zero, `0x84` to overflow, `0x85` to underflow, and other codes to undefined-result.

Concrete call-site checks establish the following behavior used by the Rust core:

- division/reciprocal by zero -> resource 67;
- inverse-function domain violations such as `asin(2)`, `acos(2)`, `acosh(x<1)`, and `atanh(|x|>=1)` -> resource 68;
- negative square root and non-positive `ln`/`log` -> resource 69, not resource 68;
- tangent results whose absolute value exceeds the original `1e15` asymptote guard -> resource 69;
- square operands above `1e154`, cube operands above `1e102`, factorial operands above 170, and overflowing arithmetic -> resource 70;
- C-runtime/internal underflow -> resource 71;
- negative/fractional factorial operands -> resource 68;
- unary `Not` checks `abs(input) <= 4294967295.0` at **0x00404399–0x004043AE** and classifies an out-of-range operand as resource 68 (`Invalid input for function.`);
- binary `And`, `Or`, `Xor`, and `Lsh` (command IDs `0x56..0x59`) share a separate unsigned-DWORD magnitude guard at **0x00404D31–0x00404D59** and classify an out-of-range operand as resource 70 (`Result is too large.`);
- `Inv` + `x^y` is a distinct root operation: the handler at **0x00404FCE–0x0040501F** consumes `Inv`, rejects a zero `y` with resource 68 at **0x00404FFE**, otherwise replaces `y` with `1/y`, and then calls the normal power routine;
- Average with an empty Statistics Box explicitly calls the dispatcher with index 0 at **0x00402096**; standard deviation with zero or one sample returns zero instead of raising an error;
- non-decimal integer conversion checks magnitude against `4294967295.0`; an out-of-range positive value routes to resource 70 and an out-of-range negative value to resource 71.

Three additional user-visible error resources were recovered outside the math dispatcher:

| Resource | Text | Original path |
|---:|---|---|
| 73 | `Cannot open Clipboard.` | `OpenClipboard` failure around **0x00403CED–0x00403D0B** |
| 74 | `There is not enough memory for data.\rClose one or more programs, and then try again.` | failed `WinHelpA` path at **0x00403ED7–0x00403EF1** (also used as the executable's generic data-memory message) |
| 78 | `Not Enough Memory` | startup resource-buffer allocation failure around **0x00401093–0x004010ED** |

Buildfix32 centralizes these recovered strings in `src/errors.rs`; the normal math/display paths use the five exact runtime categories, including the otherwise easy-to-conflate `Not`/binary-logic range cases and inverse-power zero case. Windows clipboard-open failure uses resource 73 exactly. Because the Rust port explicitly owns Unicode clipboard buffers whereas the original delegates ordinary copy behavior through Win32 controls, allocation/lock/set-data failures in that modern path use resource 74 as the closest recovered data-memory diagnostic rather than pretending that CALC.EXE had an identical clipboard allocation call site. The external HLP-viewer launch failure likewise maps to resource 74 once both modern help files have been resolved, matching the original failed-`WinHelpA` diagnostic. Startup performs a fallible 0x400-byte resource reserve so the original startup-memory message still has a corresponding recoverable path.

The executable also contains Microsoft C runtime fatal strings such as `<Main> Not enough memory.`, `runtime error`, `DOMAIN error`, `SING error`, and `TLOSS error`. These are compiler-runtime diagnostics rather than Calculator-owned `RT_STRING` UI resources; no Calculator message path was found that treats them as ordinary display errors. Buildfix32 therefore reproduces the Calculator-owned errors above instead of inventing UI routes for CRT fatal diagnostics.

The corrected expression parser remains intentionally more capable than CALC.EXE's character-at-a-time paste interpreter, so syntax diagnostics for malformed pasted expressions are Rust-port diagnostics rather than fabricated Windows 95 resource messages.

## Classic 3-D control rendering (buildfix42 audit)

A fresh audit of the original 59,392-byte `CALC.EXE` confirms that its visual
bevels are built with USER32's classic frame primitives rather than arbitrary
hand-drawn grey lines.

* `0x004025E0..0x00402642` calls `DrawFrameControl` with `DFC_BUTTON` (`4`).
  Normal pushbuttons use `DFCS_BUTTONPUSH` (`0x10`); the pressed path uses
  `DFCS_BUTTONPUSH | DFCS_PUSHED` (`0x210`).  The original keyboard-flash path
  also briefly uses the flat variant (`0x4010`) between pressed and normal.
* `0x00402804` calls `DrawEdge` with edge `0x06` (`EDGE_ETCHED`) and flags
  `0x02` (`BF_BOTTOM`) for the thin Scientific-mode separator.
* The edge-style table at `0x0040C3C8` contains repeated `(0x06, 0x0F)` and
  `(0x0A, 0x0F)` pairs: `EDGE_ETCHED/BF_RECT` and
  `EDGE_SUNKEN/BF_RECT`.  These correspond to the etched selector/group
  framing and recessed fields/wells used throughout the calculator.

Buildfix42 therefore routes our custom classic painters through the same
`DrawFrameControl`/`DrawEdge` primitives.  This preserves wxDragon ownership
and the modern DPI/layout work while making the bevel geometry and contrast
follow the original application's drawing model much more closely.


## Portable rendering bridge (buildfix43)

The USER32 primitives recovered above cannot execute on Linux.  The Linux path
therefore maps the same semantic painter hooks onto wxGTK's GTK3 widgets.  A
single application CSS provider reproduces the Win95 default 3-D palette and
edge ordering while leaving the actual wxDragon controls in place.  This is a
rendering adaptation, not a claim that `CALC.EXE` contained GTK code: the
reference semantics remain the recovered `DrawFrameControl`/`DrawEdge` states,
and the GTK rules are their platform-equivalent visual realization.


## OpenCalc executable identity and icon resources — buildfix51

The localized window caption remains the original application noun (`Calculator` / `Calculadora`), but the distribution identity is now OpenCalc. Cargo builds an explicit `OpenCalc` binary target, and a Windows-only build script links `calc95.ico` plus OpenCalc file-version strings into the PE resource table. This is separate from the existing runtime `WM_SETICON` assignment: the PE resource supplies Explorer/file properties, while the runtime path assigns the live Calculator and Statistics frame icons.


## Statistics retained-focus behavior — buildfix51

The previous builds attempted to make the Statistics caption look active while Calculator had actually taken focus. Buildfix51 instead implements retained focus with the Win32 activation protocol: when Statistics is the current foreground owned window, the Calculator owner returns `MA_NOACTIVATE` from `WM_MOUSEACTIVATE`. Windows still delivers the pending click, so Calculator controls remain mouse-operable, but Statistics stays the active/focused top-level window. RET explicitly focuses Calculator. The policy is conditional on Statistics being the actual foreground window, so activating OpenCalc from another application is not blocked.


## Buildfix62 localized-HLP block invariant

The buildfix60/61 localization scripts repacked translated LinkData2 inside each preallocated topic region but treated the transformed `|TOPIC` stream as if every byte were equivalent. That is incorrect at record boundaries: a TOPICLINK payload may be stitched across transformed blocks, while the fixed 21-byte TOPICLINK header must fit wholly inside one transformed block. The Portuguese buildfix61 manual placed a header at TOPICPOS 8162, offset 4066 of a 4084-byte transformed block, leaving only 18 bytes and causing the viewer to reject the file.

Buildfix62 retains the topic-header anchors but inserts zero padding whenever fewer than 21 transformed bytes remain before a physical block boundary. It then rewrites `BlockSize`, `DataLen2`, previous/next pointers, and all `TOPICBLOCKHEADER` fields (`LastTopicLink`, `FirstTopicLink`, `LastTopicHeader`) from the final record positions. The Help sidecars are simultaneously renamed and moved under `Help/`, with matching `|SYSTEM` CNT names and CNT `:Base` directives.

## Focused keyboard accelerators — buildfix65 audit

A fresh audit of the supplied 59,392-byte Windows 95 `CALC.EXE` confirms that
ordinary keyboard operation is not implemented by giving focus to individual
pushbuttons. The executable loads the named accelerator resource `SA` at
`0x004012E9` through `LoadAcceleratorsA`. In the main message loop, a live
Statistics dialog is first offered the message through `IsDialogMessageA` at
`0x00401321`; messages not consumed there are offered to
`TranslateAcceleratorA` at `0x0040133B`, before ordinary
`TranslateMessage`/`DispatchMessageA` at `0x00401345..0x0040134B`.

That architecture makes the Calculator's keyboard commands effectively
application-level while the Calculator is the active/focused window. The `SA`
table contains 76 accelerator entries. The user-facing mappings relevant to
OpenCalc include:

```text
0-9, A-F        numeric/hexadecimal input
+ - * /         arithmetic operators
. ,             decimal point
Enter, =        equals
Backspace, Left Back
Delete          CE
Escape          C
F9              +/-
( )             parentheses
r               reciprocal
@               sqrt (Standard) / x^2 (Scientific)
s o t           sin / cos / tan
n l             ln / log
m               dms
x               Exp
y               x^y
p               PI
i h             Inv / Hyp
v               F-E
! #             n! / x^3
%               percent (Standard) / Mod (Scientific)
& | ^ < ~ ;     And / Or / Xor / Lsh / Not / Int
Ctrl+L/R/M/P    MC / MR / MS / M+
Ctrl+S/A/T/D    Sta / Ave / Sum / s
Ctrl+Insert     Copy
Shift+Insert    Paste
F2/F4/F6        Deg / Grad / Rad while decimal
F5/F6/F7/F8     Hex / Dec / Oct / Bin as applicable
```

The original `^` accelerator means **Xor**, not exponentiation. OpenCalc keeps
that recovered Scientific shortcut. Buildfix65 adds `**` as an explicit
OpenCalc extension: two consecutive ASCII `*` keystrokes replace the pending
multiplication with power, matching the already-supported pasted-expression
syntax. A single `*`, the numeric-keypad multiply key, and the Unicode `×`
alias remain multiplication.

wxDragon delivers character events to the focused child rather than through a
Win32 accelerator table. Buildfix65 therefore installs one shared Calculator
keyboard handler on the top-level Calculator surface and on the Scientific
selectors that can take focus. The graph-expression text box is deliberately
excluded so text typed there remains ordinary editable expression input.
Clicking a Calculator pushbutton explicitly returns focus to the Calculator,
which restores the same keyboard-centric workflow after interacting with an
ordinary button.


## Buildfix68 accelerator focus correction

The recovered CALC.EXE message loop calls `TranslateAcceleratorA` for the top-level Calculator before normal message translation/dispatch. That is why the original accepts calculator keys immediately after the main window becomes active and why keyboard activation is not tied to CE or any other child button. OpenCalc therefore makes the frame the ordinary keyboard sink on initial show/reactivation (except while the graph expression editor deliberately owns focus) and routes those keys through the same calculator actions as mouse clicks. Keyboard actions also pulse the corresponding button face to reproduce the original visible key feedback.

## Buildfix127: Statistics commands terminate the current numeric entry

The supplied 59,392-byte Windows 95 `CALC.EXE` was checked again (SHA-256 `b064b0ac430264eff7b79b91e743bcd36d7b3707857f5bcdc4db146911dd0e28`). The main command dispatcher accepts the four Statistics operation codes `0x75` through `0x78` at `0x00402D68`. The `Dat` branch (`0x78`) stores the current double in the dataset at `0x00402207..0x004022EA` and increments the datum count. After the Statistics routine returns, the common path writes zero to the numeric entry-in-progress state at `0x00402DA4`.

That write explains the visible behavior: `Dat` does not erase the number that was just stored, but the next digit replaces the display instead of being appended to it. The same common reset runs after `Ave`, `Sum`, and `s`. OpenCalc now mirrors that state transition in the shared `Calculator` model by ending the current entry and clearing its explicit decimal-entry bit after every Statistics command. Both frontends therefore inherit the behavior without separate event-handler code.

