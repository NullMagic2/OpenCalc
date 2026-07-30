//! User-visible error strings recovered from the Windows 95 CALC.EXE string table.
//!
//! Keep these English source strings stable. The UI localization layer maps them
//! to the selected language, while the calculator core can compare/store them
//! without depending on presentation state.

pub const DIVIDE_BY_ZERO: &str = "Cannot divide by zero.";
pub const INVALID_FUNCTION_INPUT: &str = "Invalid input for function.";
pub const FUNCTION_UNDEFINED: &str = "Result of function is undefined.";
pub const RESULT_TOO_LARGE: &str = "Result is too large.";
pub const RESULT_TOO_SMALL: &str = "Result is too small.";

pub const CANNOT_OPEN_CLIPBOARD: &str = "Cannot open Clipboard.";
pub const NOT_ENOUGH_MEMORY_FOR_DATA: &str =
    "There is not enough memory for data.\rClose one or more programs, and then try again.";
pub const STARTUP_NOT_ENOUGH_MEMORY: &str = "Not Enough Memory";
