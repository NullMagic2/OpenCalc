# OpenCalc

OpenCalc is a reimplementation of the classic Windows 95 Calculator, written from scratch in Rust.

The project preserves the familiar interface and interaction model of the original application while adding correcting old limitations.

## Why OpenCalc?

The Windows 95 Calculator remains a recognizable example of compact and practical interface design. 
OpenCalc aims to preserve that experience and improve it.

At the same time, it expands the original calculator with features such as improved expression parsing, calculation history, graph plotting, localization, modern keyboard support, and high-DPI rendering.

<img width="1770" height="852" alt="image" src="https://github.com/user-attachments/assets/2f60e2fa-dd7c-4b03-9bce-fee3a6ea8961" />


## Features

## A native reimplementation in Rust
While OpenCalc behaves like the Windows 95 calculator, it contains no legacy code.
This means that it does not depend on legacy libraries and can run reliably on modern systems, while also providing modern features such as Undo/Redo, calculation history, and a graph manager.
So yes, you can run it on Linux natively too!

<img width="2594" height="846" alt="image" src="https://github.com/user-attachments/assets/cfe00db3-28c4-47a0-9922-33281d58488d" />

<br><br>
Here is how the Linux version looks like (GTK4 needed):
<img width="1900" height="1334" alt="image" src="https://github.com/user-attachments/assets/43f15d64-b76c-4fba-9dab-562bcf047c1f" />




### Classic calculator interface

- Faithful Windows 95 appearance
- Standard and Scientific modes
- Improved font aliasing
- DPI-aware classic button borders
- Memory operations
- Statistics window
- Configurable decimal separator

### Calculation history

The optional History panel records completed calculations and can be shown or hidden from the **View** menu.

History entries automatically follow the selected decimal separator and can be cleared independently from the calculator display.

## Keyboard input

When the main Calculator window is focused, the keyboard can be used directly, similarly to the original Windows Calculator.

Some common shortcuts include:

| Key | Action |
|---|---|
| `0–9` | Enter digits |
| `+`, `-`, `*`, `/` | Arithmetic operations |
| `**` | Exponentiation |
| `Enter` or `=` | Calculate the result |
| `Backspace` | Delete the last digit |
| `Delete` | Clear the current entry |
| `Esc` | Clear the calculation |
| `F9` | Change the sign |
| `Ctrl+Insert` | Copy |
| `Shift+Insert` | Paste an expression |

Scientific mode also supports the original single-key function shortcuts.

### Complete expression parsing

An obscure feature rediscovered in the Windows 95 Calculator is its ability to evaluate complete expressions pasted from the clipboard.

For example, copying:

```text
3 * (4 + 5)
```

and pasting it into Calculator produces:

```text
27
```

This is surprisingly advanced for such an old application!

The pasted expression can contain multiple operations and parentheses, even though the normal calculator interface is designed around entering one operation at a time.

However, our reverse engineering showed that the original implementation was not a true expression parser. It was closer to a **character-to-button-command translator**.

When text was pasted, the program read each character and converted it into the same internal commands used by the calculator buttons. In simplified terms, pasting:

```text
2 * 3
```

was treated roughly like pressing:

```text
[2] [*] [3] [=]
```

This works for many ordinary expressions, but it breaks when the meaning of a character depends on its position in the expression.

The clearest example is a negative number following an operator.

The mathematically correct result of:

```text
2 * -3
```

is:

```text
-6
```

The Windows 95 Calculator instead produced:

```text
-1
```

This happened because the paste feature translated the expression into a sequence resembling ordinary button presses:

```text
[2] [*] [-] [3] [=]
```
After * was pressed, the calculator stored 2 and waited for the second operand. However, in the calculator's button-driven state machine, pressing another binary operator at that point did not make the next number negative. Instead, it replaced the pending operation.

Therefore, the - replaced the earlier *, and the calculator effectively evaluated:
```text
2 - 3
```
which produced:

```text
-1
```

The same underlying bug affected several other expressions:

| Pasted expression | Correct result | Original result |
|---|---:|---:|
| `2 * -3` | `-6` | `-1` |
| `2 / -3` | approximately `-0.6666667` | `-1` |
| `2 - -3` | `5` | `-1` |
| `2 * +3` | `6` | `5` |

The problem was that the original paste routine did not consistently distinguish between:

- binary subtraction, as in `5 - 2`;
- a unary negative sign, as in `-3`;
- a unary sign following another operator, as in `2 * -3`.

It contained special handling for a minus sign at the beginning of an expression and for signs used in exponent notation, but signs appearing after ordinary operators were still passed through the calculator's button-oriented state machine.

Consequently, the `-` in:

```text
2 * -3
```

was not reliably interpreted as “the following number is negative.” It could instead replace or interfere with the pending multiplication operation.

The feature therefore gave the impression of being a full infix-expression parser because expressions such as these worked:

```text
3 * (4 + 5)
(2 + 3) * 4
10 / (2 + 3)
```

But its behavior was actually dependent on whether the pasted character sequence happened to map cleanly onto the calculator's existing button state.

OpenCalc preserves the original paste feature but replaces that command-stream translation with a dedicated parser.

The expression is first divided into meaningful tokens:

```text
number
operator
unary sign
function
constant
parenthesis
```

Those tokens are then parsed according to explicit precedence and associativity rules. A minus sign can therefore be interpreted according to its context rather than always being treated like a press of the subtraction button.

The previously broken expressions now behave correctly:

| Expression | OpenCalc result |
|---|---:|
| `2 * -3` | `-6` |
| `2 / -3` | approximately `-0.6666667` |
| `2 - -3` | `5` |
| `2 * +3` | `6` |

The new parser also allows OpenCalc to support syntax that the original implementation did not provide consistently:

```text
3 * (4 + 5)
sqrt(3)
sin(pi / 2)
2**8
(2 + 5)^3
5!
```

Supported features include:

- Nested parentheses;
- Unary positive and negative signs;
- Exponentiation using `^` or `**`;
- Square roots and mathematical functions;
- Trigonometric and logarithmic functions;
- Constants such as `pi`;
- Postfix factorial and percentage operators;
- Scientific notation;
- Binary, octal, decimal, and hexadecimal literals;
- Bitwise operations;
- Case-insensitive function and operator names.


## Disclaimer

OpenCalc is an independent open-source reimplementation. It is not affiliated with, endorsed by, or distributed by Microsoft.
Windows and Windows 95 are trademarks of Microsoft Corporation.
