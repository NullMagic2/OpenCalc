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

The previous buildfix2 interpretation was wrong: it transposed the four scientific-function columns into a 4×5 block and then invented separate bottom rows for logic, hexadecimal digits and Dword/Word/Byte. Those extra rows are not present in the reference layout and have been removed. Dword/Word/Byte are nevertheless real controls: in the reference they reuse the same right-hand three-radio group as Deg/Rad/Grad. Decimal mode labels that group Deg/Rad/Grad; Hex, Oct and Bin relabel it Dword/Word/Byte.

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
F2/F3/F4        Deg / Rad / Grad while decimal; Dword / Word / Byte otherwise
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


## Buildfix143: exact Win95 Dword / Word / Byte semantics

A second audit of the supplied 59,392-byte Windows 95 `CALC.EXE` showed that
buildfix140/141 had recovered the selector labels and the final output mask, but
had not yet reproduced the complete state model behind those selectors.

The selector dispatcher at `0x004037C7..0x0040380A` subtracts control ID 127 to
obtain selector index 0/1/2.  In a non-decimal radix it reads the mask table at
`0x0040C418` and stores the selected mask at `0x0040C0FC`.  The three DWORDs in
that table are, in order:

```text
Dword  0xFFFFFFFF
Word   0x0000FFFF
Byte   0x000000FF
```

The crucial detail is *where* that mask is used.  The non-decimal display path
at `0x0040426C..0x004042FE` first passes the current x87 value through the CRT
floor path at `0x0040590F` and writes the integerized result back to
`0x0040C068`.  It then checks the magnitude against `4294967295.0`
(`0x0040A4B0`).  Only after that check does it convert the integer to EAX and
execute `AND [0x0040C0FC]` at `0x004042C8`.  Therefore Word and Byte are
lower-bit **display views**, not 16-bit/8-bit arithmetic accumulators.  Hidden
upper bits of the full non-decimal integer remain part of the current value and
reappear when Dword is selected.  Fractions do not remain hidden: entering a
non-decimal radix canonicalizes them with floor first.

Direct non-decimal digit entry follows the same rule.  At
`0x00402D35..0x00402D4C`, CALC.EXE computes `current * radix + digit` into the
full numeric state at `0x0040C068`, then calls the common display routine.  A
Word entry can consequently contain more than 16 significant bits even though
only the low 16 bits are visible; changing to Dword exposes those hidden bits.
Backspace and sign handling must operate on that full entry rather than on the
masked display string.

The bitwise operators are DWORD operations regardless of the selected display
width.  `And`, `Or` and `Xor` at `0x00404DAB..0x00404E06`, and `Lsh` at
`0x00404E0B..0x00404E2A`, consume the low 32 bits in x86 registers and store the
result with `FILD DWORD`, giving a signed 32-bit internal result.  The x86 shift
therefore masks its count to five bits.  Unary `Not` at
`0x00404399..0x004043CB` likewise uses `NOT EAX` followed by `FILD DWORD`.

Buildfix143 implements those recovered rules for Hex, Oct and Bin alike.  For
example, with Word selected, `FFFF * 2` displays `FFFE` but retains the full
value `0x1FFFE`; selecting Dword then displays `1FFFE`.  With Byte selected,
`FF + 1` displays `0` while the full value remains `0x100`.  The equivalent
masking occurs in octal (`377 + 1`) and binary (`11111111 + 1`).  A value whose
full magnitude exceeds one unsigned DWORD is rejected before any Word/Byte
mask can hide the overflow.

## Buildfix144: remaining selector/radix details recovered from CALC.EXE

The buildfix143 audit was extended through the inverse-shift dispatcher, the
non-decimal integerization path, and the original accelerator resource.  Three
details were still missing from OpenCalc.

First, `Inv+Lsh` is the Windows 95 calculator's right shift.  At
`0x00402DD7..0x00402DFA`, an active Inv flag combined with command `0x59`
(Lsh) is replaced by internal operation `7`, and Inv is cleared.  Operation 7
at `0x00404D8A..0x00404DA9` converts both operands to DWORDs and executes x86
`SAR`, not `SHR`.  Right shift is therefore **arithmetic/signed**, sign-extending
bit 31, with the x86 five-bit shift-count rule.  The original input table has
`<` for Lsh; there is no separate `>` accelerator in this Windows 95 binary,
so right shift remains Inv+Lsh rather than a new visible key binding.

Second, the integerization step described above is state-changing.  The helper
at `0x0040590F` sets an x87 round-down control word and reaches `FRNDINT` at
`0x00406C5B`; `0x004041F9` stores that result back before testing the DWORD
limit.  Consequently Hex/Oct/Bin arithmetic and decimal-to-non-decimal base
changes retain the floored integer, not an undisplayed fractional value.  For
example, decimal `-1.1` becomes integer `-2` on conversion to a non-decimal
radix, and `5 / 2` in a non-decimal radix finishes as integer `2`.

Third, the shared selector's function-key mapping is F2/F3/F4 in **both** label
sets.  The accelerator resource beginning at raw file offset `0xD05C` maps
`VK_F2/VK_F3/VK_F4` to control IDs `0x7F/0x80/0x81`, while F5/F6/F7/F8 map to
Hex/Dec/Oct/Bin (`0x7C/0x7B/0x7A/0x79`).  Thus decimal mode uses
F2/F3/F4 = Deg/Rad/Grad; Hex/Oct/Bin use F2/F3/F4 = Dword/Word/Byte; and F6 is
always Dec.  The earlier OpenCalc mapping of Rad to F6 was not faithful to the
supplied executable.

## Buildfix145: radix commit order, invalid-input beep, PI and Exp

A further pass over the Scientific command dispatcher exposed five behaviors
around the radix selectors that are separate from the Dword/Word/Byte masks.

The base command writes the newly selected radix before running the common
conversion/display path (`0x004050C4..0x004050CB`).  A Decimal value that is
outside the unsigned-DWORD domain therefore leaves Hex/Oct/Bin selected even
when conversion reports `Result is too large.`  Clearing after that error
starts at zero in the newly selected radix.

Standard mode explicitly dispatches Decimal while changing layouts
(`0x004016B0..0x004016BC`).  OpenCalc must therefore reset the radix to Decimal
when entering Standard rather than preserving a hidden Scientific radix.

The numeric input dispatcher compares a candidate digit with the active radix
and calls `MessageBeep` when the digit is not representable
(`0x00402CD0..0x00402CE1`).  The decimal-point command follows the same user
feedback convention when a non-decimal radix is active.  Rejected input does
not alter the numeric state.

PI has a Decimal-only guard in the Scientific dispatcher
(`0x004035C6..0x0040361D`).  Plain PI enters pi; with Inv active it enters 2*pi
and consumes Inv.  In non-decimal modes the command is rejected with the same
invalid-input beep.

Finally, Exp is not the mathematical exponential function.  Command dispatch
at `0x00403758..0x0040378A` enters the exponent-input routine at `0x00405254`.
The display uses a signed three-column exponent such as `12.e+003`; after C the
special initial form is `1.e+000`.  The helper at `0x0040516B..0x0040517C`
compares the current numeric exponent with 29 before multiplying it by ten and
adding the next digit.  The three columns are therefore a rolling numeric field,
not a three-keystroke counter: arbitrary leading zeroes remain acceptable, but
the magnitude cannot advance past 289 for either sign.  Backspace performs the
reverse decimal shift at `0x0040529F..0x004052C4` (289 -> 028 -> 002 -> 000),
and +/- independently toggles the exponent sign at `0x004051F3..0x00405217`.
The exponent-entry routine itself rejects only an already-active exponent field,
so a completed Decimal result can enter Exp mode too; the implicit mantissa 1 is
used only when the common entry buffer length is actually zero.  Mathematical
e^x remains available through Inv+ln; it is not attached to the Exp button.

## Buildfix146: state-machine fidelity versus obsolete limits

A final state-machine pass after buildfix145 separates behavioral compatibility
from limitations that do not need to be reproduced.  The original Calculator
keeps an error state modal for calculator commands until C or CE, retains the
last Standard binary operator/right operand for repeated `=`, preserves the
current numeric value across Standard/Scientific layout changes, closes the
modeless Statistics dialog when returning to Standard, and replaces a pending
Scientific binary operator when another operator is selected.  OpenCalc now
matches those behaviors.

Two recovered limits are deliberately *not* treated as compatibility goals.
CALC.EXE caps interactive parenthesis nesting at 25 and Decimal mantissa entry at
13 digits.  OpenCalc keeps dynamically sized parenthesis and Decimal entry state
instead.  This is the same compatibility policy already used for Paste: preserve
observable arithmetic/state semantics, but do not recreate an obsolete capacity
limit when removing it does not make ordinary Win95-compatible calculations
behave differently.  Unmatched closing parentheses still use the classic
invalid-input feedback path.

## Buildfix147: memory entry, one-shot modifiers, Int/inverse powers and display formatting

The unary dispatcher has a post-operation selector cleanup stage at
`0x00403006..0x00403075`.  It is reached only after the unary routine succeeds;
an error exits earlier at `0x00402FF4..0x00403001`.  Inv is cleared for the
commands whose meaning it changes (including Int, trig, ln/log, square/cube and
dms), while Hyp is cleared only for sin/cos/tan.  OpenCalc therefore models
those selectors as one-shot only for applicable successful operations rather
than treating the checkboxes as permanently latched modes.

The accelerator resource maps `;` to command `0x60`, identifying the Int case
at `0x00404341`.  That case calls the runtime split-number helper and uses the
integer component in normal mode, which is truncation toward zero rather than
floor.  Its inverse path uses the complementary fractional component.  The
square/cube cases at `0x0040489A..0x00404943` branch on Inv: x^2 becomes sqrt
and x^3 becomes the real cube-root operation when Inv is active.

The common Decimal display routine at `0x004041F9` passes precision `0x0D`
(13) to the decimal conversion helper at `0x00403F29`.  General formatting at
`0x004040E9` switches to exponential form when the recovered decimal exponent
is at least 13, or when the significant-digit count minus that exponent exceeds
16.  F-E takes the explicit exponential formatter at `0x0040407B`.  Buildfix147
moves this fidelity into the presentation layer only: direct-entry text remains
unrestricted and arithmetic remains f64, but completed results are rounded and
laid out from a 13-significant-digit decimal record like the reference.

Memory commands also terminate direct numeric editing in the reference state
machine.  Buildfix147 represents that independently from Scientific's
"current operand exists" state: MR/MS/MC/M+ can make the next digit replace the
visible value without causing an MR operand to disappear from a pending
Scientific expression.  This separation preserves the observable Win95 rule
without forcing OpenCalc back into the original tightly coupled input buffer.

## Buildfix149: Scientific clear state, exponent columns and trig zeroes

The central clear-command state machine distinguishes C from CE beyond their numeric buffer effects. In Scientific mode both release Inv and Hyp. C additionally clears the F-E display selector, while CE leaves F-E unchanged. OpenCalc now keeps that distinction in `Calculator::clear_all` versus `Calculator::clear_entry` instead of trying to reproduce it only in a front end.

The exponential display path uses a fixed template with three exponent columns. The visible exponent is therefore `e+000` / `e-000`, not a variable-width language-runtime exponent. The 13-significant-digit conversion and fixed/scientific selection recovered for buildfix147 are retained; buildfix149 only corrects the final exponent-field layout.

The direct trig routine contains additional floating-point cleanup around the x87 instructions. FSIN and FCOS results are compared against the constant `1e-15` at `0x0040A518`; magnitudes below that value are stored as exact zero. FPTAN is different: it has no corresponding general post-operation clamp. Instead, the dispatcher performs exact zero special cases for `|x| = 180, 360, 540, 720` degrees and `|x| = pi, 2pi, 3pi, 4pi` radians before FPTAN, then retains the existing `1e15` asymptote guard. The Grads path has no separate tangent exact-compare branch in the supplied executable.


DMS normalization at `0x00404A0F..0x00404ABF` is narrower than a general seconds/minutes carry. The routine performs a second `modf()` split after scaling the first fractional component by 60 (normal DMS) or 100 (Inv+DMS), compares only that second split's fractional remainder with `0.99999999999`, and, when it is greater, increments that split's integer component and zeros the remainder. It does not perform an additional third split that rounds arbitrary `30.999...` seconds, nor does it explicitly carry a resulting minute value of 60 into the degree field.

## Buildfix152: dynamic parenthesis frames, Standard selector invariants and parser Int

The original Scientific close-parenthesis handler saves and restores per-level calculation state rather than evaluating the entire visible expression at once. OpenCalc now mirrors that observable behavior with a dynamically sized frame stack: `(` suspends the outer Scientific expression, `)` evaluates only the active frame, and the resulting operand is restored into the outer frame. This preserves outer operations and nested groups while deliberately omitting CALC.EXE's 25-frame capacity limit.

The Standard-mode selector dispatch path at `0x0040309E..0x004030AC` routes F2-F8 through Decimal rather than allowing the hidden Scientific Deg/Rad/Grad, Dword/Word/Byte, or radix state to change. Both front ends now intercept these selector accelerators in Standard mode, and the core additionally refuses non-Decimal radix changes while Standard is active.

OpenCalc's richer expression grammar intentionally remains an extension of the classic command-by-command calculator, but named functions should agree with their corresponding buttons. The parser therefore maps `int(x)` to truncation toward zero, while keeping `floor(x)` as a distinct modern function.


## Buildfix153: entry sign state, Scientific equals and radix-stable operands

The `+/-` dispatcher around `0x004033DF..0x00403417` keeps entry sign separately from numeric magnitude. A zero/new operand can therefore carry a negative sign invisibly until the next digit is entered. OpenCalc now models that input state explicitly instead of treating zero as an unconditional no-op.

The Scientific equals path shares the original saved-operand behavior: when a binary operator is immediately followed by `=`, the displayed left operand supplies the missing RHS, and the resulting operator/RHS pair is retained for repeated equals.

Scientific expression accumulation now preserves the radix of each operand at entry time. Decimal operands remain canonical decimal literals, while non-Decimal operands are stored with explicit `0x`, `0o`, or `0b` prefixes. Internal Scientific evaluation therefore never re-qualifies earlier bare operands using a radix selected later. The enhanced Paste path intentionally remains separate and continues interpreting bare pasted integers in the currently selected radix.

The Backspace handler's invalid-state branch in the reference calculator reaches `MessageBeep`; both front ends now route non-editable Backspace actions through OpenCalc's existing invalid-input beep helper.
