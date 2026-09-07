use crate::errors::{DIVIDE_BY_ZERO, FUNCTION_UNDEFINED, INVALID_FUNCTION_INPUT, RESULT_TOO_LARGE, RESULT_TOO_SMALL};
use crate::expr::{eval_expression, AngleMode, EvalContext};
use crate::locale::NumericLocale;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Standard, Scientific }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base { Hex=16, Dec=10, Oct=8, Bin=2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordSize { Dword, Word, Byte }

impl WordSize {
    fn mask(self) -> u32 {
        match self {
            WordSize::Dword => u32::MAX,
            WordSize::Word => u16::MAX as u32,
            WordSize::Byte => u8::MAX as u32,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExponentEntryState {
    mantissa: String,
    negative: bool,
    exponent: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp { Add, Sub, Mul, Div, Mod, Pow, Root, And, Or, Xor, Lsh, Rsh }

#[derive(Clone, Debug, PartialEq)]
pub struct Calculator {
    pub mode: Mode,
    pub angle: AngleMode,
    pub base: Base,
    pub word_size: WordSize,
    pub inv: bool,
    pub hyp: bool,
    pub memory: f64,
    pub memory_set: bool,
    pub display: String,
    pub error: Option<String>,
    accumulator: f64,
    pending: Option<BinaryOp>,
    // Both calculator modes remember the last completed binary operation so
    // repeated '=' re-applies the same operator/right operand (2 + 3 = = => 8).
    repeat_binary: Option<(BinaryOp, f64)>,
    entering: bool,
    // True only when the user explicitly entered the decimal separator for the
    // current entry. The classic display always shows a trailing separator for
    // decimal integers, so this state cannot be reconstructed from display text.
    decimal_entered: bool,
    scientific_expr: String,
    // Dynamically sized parenthesis frame stack. Each entry stores the outer
    // Scientific expression that was suspended when `(` opened a new group.
    // Unlike CALC.EXE, OpenCalc deliberately imposes no fixed nesting limit.
    paren_frames: Vec<String>,
    pub stats: Vec<f64>,
    pub force_exp: bool,
    number_locale: NumericLocale,
    // CALC.EXE keeps the full unmasked non-decimal integer even while a
    // Word/Byte selector hides its upper bits.  Keep that value separate from
    // the masked display so selector changes, continued entry and decimal
    // conversion preserve the original DWORD-wide numeric state.
    non_decimal_value: Option<f64>,
    // Exp in the original Calculator is an input mode, not e^x.  Keep the
    // mantissa and exponent-entry state separately so +/- and Backspace edit
    // the exponent field while the display remains in classic 1.e+000 form.
    exponent_entry: Option<ExponentEntryState>,
    // Exact expression token for constants whose display is numeric (PI and
    // Inv+PI).  This keeps Scientific evaluation/history faithful without
    // sacrificing the classic numeric display.
    expression_entry_override: Option<String>,
    // CALC.EXE tracks the formatted entry-buffer length separately from its
    // higher-level "currently typing" state.  Exp uses an implicit mantissa 1
    // only when that buffer is genuinely empty (not merely after a completed
    // calculation whose result is no longer being typed).
    entry_buffer_empty: bool,
    // Some classic commands (notably MC/MR/MS/M+) leave the current value
    // available as an operand but end direct numeric editing.  Keep that
    // separate from `entering`, which is also used by the Scientific
    // expression builder to decide whether a current operand exists.
    replace_on_next_digit: bool,
    // Win95 keeps the sign of a new/current numeric entry separately from its
    // magnitude.  This matters for an invisible negative-zero entry: `+/-`
    // immediately after a binary operator can mark the upcoming operand
    // negative before any digit has been typed.
    entry_negative: bool,
}

impl Default for Calculator {
    fn default() -> Self {
        let number_locale = NumericLocale::system();
        Self {
            mode: Mode::Standard, angle: AngleMode::Degrees, base: Base::Dec, word_size: WordSize::Dword,
            inv: false, hyp: false, memory: 0.0, memory_set: false,
            display: zero_display(Base::Dec, number_locale), error: None, accumulator: 0.0,
            pending: None, repeat_binary: None, entering: false, decimal_entered: false, scientific_expr: String::new(), paren_frames: Vec::new(),
            stats: Vec::new(), force_exp:false, number_locale, non_decimal_value: None, exponent_entry: None, expression_entry_override: None, entry_buffer_empty: true, replace_on_next_digit: false, entry_negative: false,
        }
    }
}

impl Calculator {
    pub fn clear_all(&mut self) {
        self.display = zero_display(self.base, self.number_locale); self.error = None; self.accumulator = 0.0;
        self.pending = None; self.repeat_binary = None; self.entering = false; self.decimal_entered = false; self.scientific_expr.clear(); self.paren_frames.clear();
        self.non_decimal_value = if self.base == Base::Dec { None } else { Some(0.0) };
        self.exponent_entry = None;
        self.expression_entry_override = None;
        self.entry_buffer_empty = true;
        self.replace_on_next_digit = false;
        self.entry_negative = false;
        // Win95 C is a full Scientific-state reset: both one-shot modifiers
        // are released and F-E returns to normal display.  These flags are
        // harmless in Standard mode, so clearing them unconditionally keeps
        // mode transitions deterministic too.
        self.inv = false;
        self.hyp = false;
        self.force_exp = false;
    }

    pub fn clear_entry(&mut self) {
        self.display = zero_display(self.base, self.number_locale);
        self.error = None;
        self.repeat_binary = None;
        self.entering = false;
        self.decimal_entered = false;
        self.non_decimal_value = if self.base == Base::Dec { None } else { Some(0.0) };
        self.exponent_entry = None;
        self.expression_entry_override = None;
        self.entry_buffer_empty = true;
        self.replace_on_next_digit = false;
        self.entry_negative = false;
        // CE clears the active Inv/Hyp modifiers but deliberately preserves
        // the F-E presentation selector.  CALC.EXE distinguishes this from C.
        self.inv = false;
        self.hyp = false;
    }

    pub fn backspace(&mut self) {
        if !self.entering || self.error.is_some() || self.replace_on_next_digit { return; }
        if let Some(state) = self.exponent_entry.as_mut() {
            state.exponent /= 10;
            self.refresh_exponent_entry_display();
            return;
        }
        self.expression_entry_override = None;
        if self.base != Base::Dec {
            // The Win95 calculator edits the full current value, not the
            // masked Word/Byte text.  Dividing by the radix removes the last
            // entered digit while preserving any bits currently hidden by the
            // selector.
            let current = self.non_decimal_value.unwrap_or(0.0);
            let radix = self.base as u32 as f64;
            let next = (current / radix).trunc();
            let became_zero = next == 0.0;
            if self.try_set_value(next) {
                self.entry_negative = next.is_sign_negative() || (became_zero && self.entry_negative);
                self.entering = !became_zero;
            }
            return;
        }

        let mut s = self.raw_entry();
        s.pop();
        self.decimal_entered = s.contains('.');
        if s.is_empty() || s == "-" {
            self.display = zero_display(self.base, self.number_locale);
            self.entering = false;
            self.decimal_entered = false;
            self.entry_buffer_empty = true;
        } else {
            self.display = entry_display(&s, self.base, self.number_locale);
            self.entry_negative = s.starts_with('-');
            self.entry_buffer_empty = false;
        }
    }

    pub fn digit(&mut self, ch: char) {
        if self.error.is_some() || !self.can_accept_digit(ch) { return; }
        self.repeat_binary = None;
        let replace_current = std::mem::take(&mut self.replace_on_next_digit);

        if let Some(state) = self.exponent_entry.as_mut() {
            let digit = ch.to_digit(10).unwrap_or(0) as u16;
            state.exponent = state.exponent * 10 + digit;
            self.entry_buffer_empty = false;
            self.refresh_exponent_entry_display();
            return;
        }

        self.expression_entry_override = None;
        if self.base != Base::Dec {
            // CALC.EXE keeps a full DWORD-wide numeric entry even when Word or
            // Byte masks upper bits from the display.  Each digit extends that
            // full value and the common display path applies the current mask.
            let digit = ch.to_digit(16).unwrap_or(0) as f64;
            let radix = self.base as u32 as f64;
            let current = if self.entering && !replace_current {
                self.non_decimal_value.unwrap_or(0.0)
            } else {
                0.0
            };
            let negative = if self.entering && !replace_current && current != 0.0 {
                current.is_sign_negative()
            } else {
                self.entry_negative
            };
            let candidate = current.abs() * radix + digit;
            let signed = if negative { -candidate } else { candidate };
            if self.try_set_value(signed) {
                self.entry_negative = negative;
                self.entering = true;
                self.entry_buffer_empty = false;
            }
            return;
        }

        self.non_decimal_value = None;
        let mut s = if self.entering && !replace_current {
            self.raw_entry()
        } else {
            self.decimal_entered = false;
            if self.entry_negative { "-".to_string() } else { String::new() }
        };
        if s == "0" && !self.decimal_entered {
            s.clear();
            if self.entry_negative { s.push('-'); }
        }
        if s == "-0" && !self.decimal_entered { s.truncate(1); }
        // Deliberate OpenCalc enhancement: do not reproduce CALC.EXE's
        // historical 13-digit mantissa-entry ceiling. The entry string grows
        // normally and is converted to f64 only when arithmetic needs a value.
        s.push(ch.to_ascii_uppercase());
        self.display = entry_display(&s, self.base, self.number_locale);
        self.entry_negative = s.starts_with('-');
        self.entering = true;
        self.entry_buffer_empty = false;
    }

    pub fn decimal_point(&mut self) {
        if self.error.is_some() || self.base != Base::Dec || self.exponent_entry.is_some() { return; }
        self.repeat_binary = None;
        self.expression_entry_override = None;
        let replace_current = std::mem::take(&mut self.replace_on_next_digit);
        let mut s = if self.entering && !replace_current {
            let mut raw = self.raw_entry();
            if self.entry_negative && !raw.starts_with('-') {
                raw.insert(0, '-');
            }
            raw
        } else if self.entry_negative {
            "-0".into()
        } else {
            "0".into()
        };
        if !self.decimal_entered {
            if !s.contains('.') { s.push('.'); }
            self.decimal_entered = true;
        }
        self.display = entry_display(&s, self.base, self.number_locale);
        self.entering = true;
        self.entry_buffer_empty = false;
    }

    pub fn sign(&mut self) {
        if self.error.is_some() { return; }
        if !self.can_toggle_sign() { return; }
        self.repeat_binary = None;
        if let Some(state) = self.exponent_entry.as_mut() {
            // During Exp entry, +/- changes the exponent sign rather than the
            // mantissa sign, exactly like the Windows 95 Calculator.
            state.negative = !state.negative;
            self.refresh_exponent_entry_display();
            return;
        }
        self.expression_entry_override = None;

        let waiting_for_operand = !self.entering && match self.mode {
            Mode::Standard => self.pending.is_some(),
            Mode::Scientific => trailing_scientific_binary_operator(&self.scientific_expr).is_some()
                || (self.scientific_expr.is_empty() && !self.paren_frames.is_empty()),
        };
        let value = self.value().unwrap_or(0.0);

        if waiting_for_operand || value == 0.0 {
            // The reference calculator stores this sign even though zero itself
            // remains visually unchanged.  The next entered digit consumes the
            // sign as part of the new operand (e.g. 2 + +/- 3 = -> -1).
            self.entry_negative = !self.entry_negative;
            return;
        }

        self.entry_negative = !value.is_sign_negative();
        if self.base != Base::Dec {
            if self.try_set_value(-value) {
                self.entry_negative = !value.is_sign_negative();
                self.entering = true;
            }
            return;
        }

        self.non_decimal_value = None;
        let mut raw = self.raw_entry();
        if raw.starts_with('-') { raw.remove(0); } else { raw.insert(0, '-'); }
        self.display = entry_display(&raw, self.base, self.number_locale);
        self.entry_negative = raw.starts_with('-');
        self.entering = true;
    }

    pub fn binary(&mut self, op: BinaryOp) {
        if self.error.is_some() { return; }
        self.replace_on_next_digit = false;
        // CALC.EXE implements Inv+x^y as x^(1/y).  It consumes Inv when the
        // operator is selected and explicitly rejects a zero root/exponent as
        // invalid function input (0x00404FCE..0x00405016).
        let op = if op == BinaryOp::Pow && self.inv {
            self.inv = false;
            BinaryOp::Root
        } else if op == BinaryOp::Lsh && self.inv {
            // In the supplied Windows 95 CALC.EXE, Inv+Lsh is not another
            // left shift.  The command dispatcher substitutes internal
            // operation 7, clears Inv, and that operation executes x86 SAR
            // (arithmetic/signed right shift).
            self.inv = false;
            BinaryOp::Rsh
        } else {
            op
        };
        self.repeat_binary = None;
        match self.mode {
            Mode::Standard => {
                let cur = self.value().unwrap_or(0.0);
                if self.entering {
                    if let Some(prev) = self.pending.take() {
                        match apply_binary(prev, self.accumulator, cur) {
                            Ok(v) => {
                                self.accumulator = v;
                                if !self.try_set_value(v) { return; }
                            }
                            Err(e) => { self.error = Some(e.clone()); self.display = e; return; }
                        }
                    } else { self.accumulator = cur; }
                }
                self.pending = Some(op); self.entering = false;
                self.entry_negative = false;
            }
            Mode::Scientific => {
                if self.entering || self.scientific_expr.is_empty() {
                    self.push_current_into_expr();
                    self.scientific_expr.push_str(op_text(op));
                } else if !replace_trailing_scientific_operator(&mut self.scientific_expr, op) {
                    // Keep permissive expression entry around constructs such as
                    // a freshly opened parenthesis; only an actually pending
                    // binary operator is replaced.
                    self.scientific_expr.push_str(op_text(op));
                }
                self.entering = false;
                self.entry_negative = false;
            }
        }
    }

    /// Handle the keyboard-only `**` exponentiation alias without changing
    /// the recovered single-`*` multiplication button semantics.
    ///
    /// The first `*` is processed normally.  When a second `*` arrives before
    /// another operand is entered, replace that pending multiplication with a
    /// power operation.  This mirrors the expression parser's `**` spelling
    /// while preserving the original `y` accelerator for the x^y button.
    pub fn keyboard_star(&mut self) {
        if self.error.is_some() {
            return;
        }
        match self.mode {
            Mode::Standard if !self.entering && self.pending == Some(BinaryOp::Mul) => {
                self.pending = Some(BinaryOp::Pow);
            }
            Mode::Scientific if !self.entering && self.scientific_expr.ends_with('*') => {
                self.scientific_expr.pop();
                self.scientific_expr.push('^');
            }
            _ => self.binary(BinaryOp::Mul),
        }
    }

    pub fn equals(&mut self) {
        if self.error.is_some() { return; }
        self.replace_on_next_digit = false;
        self.entry_negative = false;
        match self.mode {
            Mode::Standard => {
                let operation = if let Some(op) = self.pending.take() {
                    let rhs = self.value().unwrap_or(0.0);
                    self.repeat_binary = Some((op, rhs));
                    Some((op, rhs))
                } else {
                    self.repeat_binary
                };
                if let Some((op, rhs)) = operation {
                    // accumulator is the original left operand on the first '=',
                    // then the previous result on every repeated '='.
                    let lhs = self.accumulator;
                    match apply_binary(op, lhs, rhs) {
                        Ok(v) => {
                            self.accumulator = v;
                            self.set_value(v);
                        }
                        Err(e) => self.fail(&e),
                    }
                }
                self.entering = false;
            }
            Mode::Scientific => {
                // A binary operator followed immediately by '=' reuses the
                // displayed left operand as the missing RHS (2 + = -> 4), just
                // like the shared Win95 equals machinery. Do this for each open
                // parenthesis frame as it is resolved too.
                if !self.complete_missing_scientific_rhs() { return; }
                while !self.paren_frames.is_empty() {
                    if !self.close_paren_internal() { return; }
                    if !self.complete_missing_scientific_rhs() { return; }
                }

                if self.scientific_expr.trim().is_empty() {
                    if let Some((op, rhs)) = self.repeat_binary {
                        let lhs = self.accumulator;
                        match apply_binary(op, lhs, rhs) {
                            Ok(v) => {
                                self.accumulator = v;
                                self.set_value(v);
                            }
                            Err(e) => self.fail(&e),
                        }
                    }
                    self.entering = false;
                    return;
                }

                let repeat_candidate = self.repeat_binary.or_else(|| {
                    if self.entering {
                        trailing_scientific_binary_operator(&self.scientific_expr)
                            .and_then(|op| self.value().ok().map(|rhs| (op, rhs)))
                    } else {
                        None
                    }
                });

                self.push_current_into_expr();
                let expression = std::mem::take(&mut self.scientific_expr);
                if expression.trim().is_empty() { return; }
                self.evaluate_scientific_expression(&expression);
                if self.error.is_none() {
                    self.repeat_binary = repeat_candidate;
                }
                self.entering = false;
            }
        }
    }

    fn complete_missing_scientific_rhs(&mut self) -> bool {
        if self.entering { return true; }
        let Some(op) = trailing_scientific_binary_operator(&self.scientific_expr) else {
            return true;
        };
        let rhs = match self.value() {
            Ok(v) => v,
            Err(e) => { self.fail(&e); return false; }
        };
        let rhs_expression = self.expression_entry();
        self.scientific_expr.push_str(&rhs_expression);
        self.repeat_binary = Some((op, rhs));
        // The substituted RHS is already part of the expression string; keep
        // `entering` false so the normal finalization path does not append it a
        // second time.
        self.entering = false;
        true
    }

    pub fn open_paren(&mut self) {
        if self.error.is_some() || self.mode != Mode::Scientific { return; }
        self.repeat_binary = None;
        // Preserve implicit multiplication (for example 2(3+4)) while moving
        // the entire outer expression into a dynamically sized frame.
        if self.entering {
            self.push_current_into_expr();
            self.scientific_expr.push('*');
        }
        let outer = std::mem::take(&mut self.scientific_expr);
        self.paren_frames.push(outer);
        self.entering = false;
        self.expression_entry_override = None;
    }

    pub fn close_paren(&mut self) {
        if self.error.is_some() || self.mode != Mode::Scientific || self.paren_frames.is_empty() { return; }
        self.repeat_binary = None;
        self.close_paren_internal();
    }

    fn close_paren_internal(&mut self) -> bool {
        if self.paren_frames.is_empty() { return false; }
        if !self.complete_missing_scientific_rhs() { return false; }
        self.push_current_into_expr();
        let group_expression = std::mem::take(&mut self.scientific_expr);
        if group_expression.trim().is_empty() {
            return false;
        }
        // Scientific entries are canonicalized operand-by-operand when they are
        // pushed: Decimal stays decimal, while Hex/Oct/Bin use explicit
        // 0x/0o/0b literals. Never reinterpret older operands using a radix that
        // was selected later in the calculation.
        match eval_expression(&group_expression, self.eval_context()) {
            Ok(v) => {
                let outer = self.paren_frames.pop().unwrap_or_default();
                if self.try_set_value(v) {
                    self.scientific_expr = outer;
                    // Keep the exact group available for history / subsequent
                    // binary input. A unary operation clears this override via
                    // try_set_value and then operates on the materialized value.
                    self.expression_entry_override = Some(format!("({group_expression})"));
                    self.accumulator = v;
                    self.entering = true;
                    true
                } else {
                    false
                }
            }
            Err(e) => {
                self.fail(&e);
                false
            }
        }
    }

    pub fn can_close_paren(&self) -> bool {
        self.error.is_none() && self.mode == Mode::Scientific && !self.paren_frames.is_empty()
    }

    pub fn unary(&mut self, name: &str) {
        if self.error.is_some() { return; }
        self.replace_on_next_digit = false;
        self.repeat_binary = None;
        let x = self.value().unwrap_or(0.0);
        // The reference calculator treats Inv as a one-shot modifier for the
        // functions that actually have inverse meanings, and Hyp as one-shot
        // for the three trig keys.  Importantly, the dispatcher clears these
        // flags only after a successful operation; a domain/overflow error
        // leaves the selector state untouched.
        let consume_inv = self.inv && matches!(name, "dms" | "sin" | "cos" | "tan" | "ln" | "log" | "square" | "cube" | "int");
        let consume_hyp = self.hyp && matches!(name, "sin" | "cos" | "tan");
        let actual = match (name, self.inv, self.hyp) {
            ("sin", true, true) => "asinh",
            ("cos", true, true) => "acosh",
            ("tan", true, true) => "atanh",
            ("sin", true, false) => "asin",
            ("cos", true, false) => "acos",
            ("tan", true, false) => "atan",
            ("sin", false, true) => "sinh",
            ("cos", false, true) => "cosh",
            ("tan", false, true) => "tanh",
            ("ln", true, _) => "exp",
            ("log", true, _) => "pow10",
            ("dms", true, _) => "dms_inv",
            ("square", true, _) => "sqrt",
            ("cube", true, _) => "cuberoot",
            ("int", true, _) => "frac",
            _ => name,
        };
        let result = match actual {
            "sqrt" => {
                if x < 0.0 { Err(FUNCTION_UNDEFINED.into()) } else { Ok(x.sqrt()) }
            }
            // CALC.EXE checks conservative pre-multiply limits rather than
            // waiting for the x87 result to become infinity.
            "square" => {
                if x.abs() > 1.0e154 { Err(RESULT_TOO_LARGE.into()) } else { Ok(x * x) }
            },
            "cube" => {
                if x.abs() > 1.0e102 { Err(RESULT_TOO_LARGE.into()) } else { Ok(x * x * x) }
            },
            "cuberoot" => Ok(x.cbrt()),
            "recip" => {
                if x == 0.0 {
                    Err(DIVIDE_BY_ZERO.into())
                } else {
                    checked_large(1.0 / x)
                }
            }
            // CALC.EXE keeps these as two separate string-table messages. A
            // negative or fractional operand is invalid input, while 171! and
            // above report the overflow string.
            "factorial" => {
                if x < 0.0 || x.fract() != 0.0 {
                    Err(INVALID_FUNCTION_INPUT.into())
                } else if x > 170.0 {
                    Err(RESULT_TOO_LARGE.into())
                } else {
                    let mut r = 1.0;
                    for n in 2..=x as u64 { r *= n as f64; }
                    Ok(r)
                }
            }
            // Win95 Int uses the integer part returned by modf(), i.e.
            // truncation toward zero rather than floor.  Inv+Int returns the
            // signed fractional remainder.
            "int" => Ok(x.trunc()),
            "frac" => Ok(x.fract()),
            "not" => {
                // Unary Not has its own domain guard in CALC.EXE: unlike the
                // binary logic operators, an operand outside unsigned DWORD
                // magnitude is classified as invalid function input.
                if x.abs() > u32::MAX as f64 {
                    Err(INVALID_FUNCTION_INPUT.into())
                } else {
                    Ok(signed_dword(!low_dword(x)))
                }
            },
            "dms" => {
                let sign = if x < 0.0 { -1.0 } else { 1.0 };
                let a = x.abs();
                let degrees = a.floor();
                let (minutes, minute_fraction) = split_dms_secondary((a - degrees) * 60.0);
                // The original stores the remaining fractional minute directly
                // with a 0.006 scale. It does not separately round arbitrary
                // 30.999... second values or carry minute 60 into degrees.
                Ok(sign * (degrees + minutes * 0.01 + minute_fraction * 0.006))
            }
            "dms_inv" => {
                let sign = if x < 0.0 { -1.0 } else { 1.0 };
                let a = x.abs();
                let degrees = a.floor();
                let (minutes, packed_fraction) = split_dms_secondary((a - degrees) * 100.0);
                Ok(sign * (degrees + minutes / 60.0 + packed_fraction / 36.0))
            }
            "fe" => Ok(x),
            "pow10" => checked_exp_like(10f64.powf(x)),
            n => eval_expression(&format!("{n}({x})"), self.eval_context()),
        };
        match result {
            Ok(v) => {
                if self.try_set_value(v) {
                    self.entering = true;
                    if consume_inv { self.inv = false; }
                    if consume_hyp { self.hyp = false; }
                }
            }
            Err(e) => self.fail(&e),
        }
    }

    pub fn percent(&mut self) {
        if self.error.is_some() { return; }
        self.replace_on_next_digit = false;
        self.repeat_binary = None;
        let x = self.value().unwrap_or(0.0);
        let v = match self.pending {
            Some(_) => self.accumulator * x / 100.0,
            None => x / 100.0,
        };
        match checked_large(v) {
            Ok(value) => {
                if self.try_set_value(value) { self.entering = true; }
            }
            Err(error) => self.fail(&error),
        }
    }

    pub fn paste_expression(&mut self, text: &str) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        self.evaluate_paste(text);
    }

    fn evaluate_scientific_expression(&mut self, text: &str) {
        match eval_expression(text, self.eval_context()) {
            Ok(v) => {
                self.error = None;
                if self.try_set_value(v) {
                    self.accumulator = v;
                    self.pending = None;
                    self.entering = false;
                }
            }
            Err(e) => self.fail(&e),
        }
    }

    fn evaluate_paste(&mut self, text: &str) {
        // The expression parser deliberately treats unprefixed numerals as
        // decimal.  In the classic calculator, however, expressions entered
        // while Hex/Oct/Bin is selected use the selected radix.  Qualify only
        // bare integer tokens here; explicit 0x/0o/0b literals are preserved.
        let expression = qualify_bare_based_numbers(text, self.base);
        // Transactional: unlike CALC.EXE, invalid input cannot partly mutate calculator state.
        match eval_expression(&expression, self.eval_context()) {
            Ok(v) => {
                self.error = None;
                if self.try_set_value(v) {
                    self.accumulator = v;
                    self.pending = None;
                    self.entering = false;
                }
            }
            Err(e) => self.fail(&e),
        }
    }

    pub fn memory_clear(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        self.finish_numeric_entry();
        self.memory = 0.0; self.memory_set = false;
    }
    pub fn memory_recall(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        if self.try_set_value(self.memory) {
            // Recall terminates entry: the recalled value is visible, but a
            // following digit starts a fresh number rather than appending.
            self.entering = true;
            self.decimal_entered = false;
            self.replace_on_next_digit = true;
        }
    }
    pub fn memory_store(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        let value = self.value().unwrap_or(0.0);
        self.memory = value;
        self.memory_set = value != 0.0;
        self.finish_numeric_entry();
    }
    pub fn memory_add(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        let current = self.value().unwrap_or(0.0);
        let candidate = self.memory + current;
        match checked_add_sub(candidate) {
            Ok(value) => {
                self.memory = value;
                self.memory_set = self.memory != 0.0;
                self.finish_numeric_entry();
            }
            Err(error) => self.fail(&error),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.mode == mode { return; }

        // Mode changes are layout/state transitions, not Clear commands. Keep
        // the currently displayed numeric value, while discarding an unfinished
        // mode-specific expression/operator chain.
        let current = self.value().ok();
        self.mode = mode;
        self.pending = None;
        self.repeat_binary = None;
        self.scientific_expr.clear();
        self.paren_frames.clear();
        self.exponent_entry = None;
        self.expression_entry_override = None;
        self.entering = false;
        self.decimal_entered = false;
        self.replace_on_next_digit = false;
        self.entry_negative = false;

        // Standard Calculator has no radix selector. CALC.EXE explicitly
        // dispatches Decimal while entering Standard mode.
        if mode == Mode::Standard {
            self.base = Base::Dec;
        }

        if let Some(value) = current {
            self.accumulator = value;
            self.try_set_value(value);
            self.entering = false;
        }
    }
    pub(crate) fn paren_depth(&self) -> usize { self.paren_frames.len() }
    pub fn set_base(&mut self, base: Base) {
        if self.error.is_some() { return; }
        // Standard mode is always Decimal. This also makes the invariant robust
        // against non-UI callers; F2-F8 are separately intercepted so they do
        // not alter hidden Scientific angle/word-size state either.
        if self.mode == Mode::Standard && base != Base::Dec { return; }
        self.repeat_binary = None;
        let mut v = self.value().unwrap_or(0.0);
        // CALC.EXE commits the newly selected radix before converting the
        // current value.  Therefore an out-of-range Decimal -> Hex/Oct/Bin
        // conversion leaves the new radix selected while showing the error.
        self.base = base;
        self.exponent_entry = None;
        // The Windows 95 non-decimal display routine first calls the CRT
        // floor() path and writes that integerized value back into the
        // calculator state.  Do this before the DWORD range check: for
        // example 4294967295.9 becomes 4294967295 and is still representable,
        // while -1.1 becomes -2 (not -1) when converted to Hex/Oct/Bin.
        if base != Base::Dec {
            v = v.floor();
        }
        if base != Base::Dec && v.abs() > u32::MAX as f64 {
            self.fail(if v.is_sign_negative() { RESULT_TOO_SMALL } else { RESULT_TOO_LARGE });
            return;
        }
        self.set_value(v);
        self.entering = false;
    }

    pub fn set_word_size(&mut self, word_size: WordSize) {
        if self.error.is_some() || self.base == Base::Dec {
            return;
        }
        let value = self.value().unwrap_or(0.0);
        self.word_size = word_size;
        self.set_value(value);
    }

    /// Whether the classic keypad accepts this digit in the current radix /
    /// exponent-entry state.  The UI uses this to produce the reference
    /// Calculator's invalid-input beep without recording an undo snapshot.
    pub fn can_accept_digit(&self, ch: char) -> bool {
        if self.error.is_some() { return false; }
        if let Some(state) = &self.exponent_entry {
            if !ch.is_ascii_digit() {
                return false;
            }
            // 0x40516B compares the *current numeric exponent* with 29 before
            // shifting it left by one decimal digit.  There is no separate
            // three-keystroke counter: leading zeroes may be entered forever,
            // while 28 followed by 9 is the largest accepted magnitude (289).
            return state.exponent < 29;
        }
        match self.base {
            // OpenCalc intentionally keeps Decimal entry more permissive than
            // the original 13-digit buffer limit.
            Base::Dec => ch.is_ascii_digit(),
            Base::Hex => ch.is_ascii_hexdigit(),
            Base::Oct => matches!(ch, '0'..='7'),
            Base::Bin => matches!(ch, '0'|'1'),
        }
    }

    pub fn can_accept_decimal_point(&self) -> bool {
        self.error.is_none() && self.base == Base::Dec && self.exponent_entry.is_none() && !self.decimal_entered
    }

    pub fn can_backspace(&self) -> bool {
        self.error.is_none() && self.entering && !self.replace_on_next_digit
    }

    pub fn can_toggle_sign(&self) -> bool {
        true
    }

    pub fn can_use_pi(&self) -> bool {
        self.error.is_none() && self.mode == Mode::Scientific && self.base == Base::Dec
    }

    /// Enter the PI constant.  PI is Decimal-only in CALC.EXE; Inv+PI emits
    /// 2*pi and consumes Inv.
    pub fn pi(&mut self) -> bool {
        if !self.can_use_pi() { return false; }
        self.repeat_binary = None;
        let multiplier = if self.inv { 2.0 } else { 1.0 };
        self.inv = false;
        self.exponent_entry = None;
        self.set_value(multiplier * std::f64::consts::PI);
        self.expression_entry_override = Some(if multiplier == 2.0 { "2*pi".to_string() } else { "pi".to_string() });
        self.entering = true;
        true
    }

    pub fn can_start_exponent_entry(&self) -> bool {
        if self.error.is_some() || self.mode != Mode::Scientific || self.base != Base::Dec || self.exponent_entry.is_some() {
            return false;
        }
        // Dispatcher 0x403758 checks only Scientific mode and Decimal; the
        // entry helper itself rejects only an already-active exponent field.
        // Therefore Exp is also valid on a completed displayed result.
        true
    }

    /// Start Win95 Exp input mode.  Exp is not the mathematical e^x function.
    /// It appends a signed, three-column decimal exponent field to the current
    /// mantissa.  In the clear state the implicit mantissa is 1.
    pub fn exponent_entry(&mut self) -> bool {
        if !self.can_start_exponent_entry() { return false; }
        self.repeat_binary = None;
        let mantissa = if self.entry_buffer_empty { "1".to_string() } else { self.raw_entry() };
        self.expression_entry_override = None;
        self.exponent_entry = Some(ExponentEntryState {
            mantissa,
            negative: false,
            exponent: 0,
        });
        self.entering = true;
        self.decimal_entered = false;
        self.entry_buffer_empty = false;
        self.refresh_exponent_entry_display();
        true
    }

    fn refresh_exponent_entry_display(&mut self) {
        let Some(state) = &self.exponent_entry else { return; };
        let mut mantissa = state.mantissa.clone();
        if !mantissa.contains('.') {
            mantissa.push('.');
        }
        let sign = if state.negative { '-' } else { '+' };
        self.display = self.number_locale.localize(&format!("{mantissa}e{sign}{:03}", state.exponent));
    }

    pub fn stat_dat(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        if let Ok(value) = self.value() {
            self.stats.push(value);
        }
        self.finish_statistics_action();
    }

    pub fn stat_sum(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        let sum = self.stats.iter().copied().sum::<f64>();
        match checked_large(sum) {
            Ok(value) => { self.set_value(value); }
            Err(error) => self.fail(&error),
        }
        self.finish_statistics_action();
    }

    pub fn stat_avg(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        if self.stats.is_empty() {
            self.fail(DIVIDE_BY_ZERO);
        } else {
            let sum = self.stats.iter().copied().sum::<f64>();
            match checked_large(sum / self.stats.len() as f64) {
                Ok(value) => { self.set_value(value); }
                Err(error) => self.fail(&error),
            }
        }
        self.finish_statistics_action();
    }

    pub fn stat_stddev(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        if self.stats.len() <= 1 {
            self.set_value(0.0);
        } else {
            let mean = self.stats.iter().sum::<f64>() / self.stats.len() as f64;
            let variance = self.stats.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>()
                / (self.stats.len() - 1) as f64;
            match checked_large(variance.sqrt()) {
                Ok(value) => { self.set_value(value); }
                Err(error) => self.fail(&error),
            }
        }
        self.finish_statistics_action();
    }

    /// CALC.EXE clears its common entry-in-progress flag after every
    /// Statistics command.  The displayed value remains visible, but the next
    /// digit starts a fresh entry instead of being appended to that value.
    fn finish_statistics_action(&mut self) {
        self.entering = false;
        self.decimal_entered = false;
    }

    /// Memory commands in CALC.EXE end the current numeric-entry phase without
    /// clearing the visible value.  Resolve an active Exp field to its numeric
    /// value as part of that transition so a later digit cannot continue
    /// editing a stale exponent buffer.
    fn finish_numeric_entry(&mut self) {
        // Preserve OpenCalc's deliberately unrestricted direct-entry text.
        // Only Exp needs materializing because otherwise its separate editor
        // state would keep consuming following digits after the memory action.
        if self.exponent_entry.is_some() {
            if let Ok(value) = self.value() {
                self.try_set_value(value);
            }
        }
        self.decimal_entered = false;
        self.replace_on_next_digit = true;
        self.entry_negative = false;
    }

    pub fn value(&self) -> Result<f64, String> {
        if self.error.is_some() { return Err("Calculator is in error state.".into()); }
        if let Some(state) = &self.exponent_entry {
            let mantissa = state.mantissa.parse::<f64>().map_err(|_| "Invalid number".to_string())?;
            let exponent = state.exponent as i32;
            let exponent = if state.negative { -exponent } else { exponent };
            let value = mantissa * 10.0_f64.powi(exponent);
            return if value.is_finite() { Ok(value) } else { Err(RESULT_TOO_LARGE.into()) };
        }
        if self.base != Base::Dec {
            if let Some(value) = self.non_decimal_value {
                return Ok(value);
            }
        }
        let s = self.raw_entry();
        if self.base == Base::Dec { s.parse::<f64>().map_err(|_| "Invalid number".into()) }
        else {
            let neg=s.starts_with('-'); let d=s.trim_start_matches('-').trim_end_matches('.');
            i64::from_str_radix(d, self.base as u32).map(|v| if neg {-(v as f64)} else {v as f64}).map_err(|_| "Invalid integer".into())
        }
    }

    pub fn set_value(&mut self, v: f64) {
        self.try_set_value(v);
    }

    fn try_set_value(&mut self, v: f64) -> bool {
        self.exponent_entry = None;
        self.expression_entry_override = None;
        // Every non-decimal result is floor()'d before CALC.EXE performs the
        // unsigned-DWORD range check and display mask.  Retain that full
        // integer separately from the Word/Byte-masked text.
        let v = if self.base == Base::Dec { v } else { v.floor() };
        // The original non-decimal display path rejects magnitudes outside one
        // unsigned DWORD *before* applying the selected Dword/Word/Byte mask.
        // Word and Byte may wrap visually inside that DWORD range, but Dword
        // itself never silently wraps past 0xFFFFFFFF.
        if self.base != Base::Dec && v.abs() > u32::MAX as f64 {
            self.fail(if v.is_sign_negative() { RESULT_TOO_SMALL } else { RESULT_TOO_LARGE });
            return false;
        }
        self.display = format_value(v, self.base, self.word_size, self.force_exp, self.number_locale);
        self.non_decimal_value = if self.base == Base::Dec { None } else { Some(v) };
        self.error = None;
        self.decimal_entered = false;
        self.entry_buffer_empty = false;
        true
    }

    /// Recall a completed numeric result from the user-visible History panel.
    /// This deliberately starts a fresh calculation from that value while
    /// preserving mode, base, angle, memory, statistics, and presentation
    /// preferences.  The state matches the useful post-Equals state: pressing
    /// a binary operator next uses the recalled number as its left operand.
    pub fn recall_history_value(&mut self, v: f64) {
        self.error = None;
        self.accumulator = v;
        self.pending = None;
        self.repeat_binary = None;
        self.entering = false;
        self.decimal_entered = false;
        self.scientific_expr.clear();
        self.paren_frames.clear();
        self.exponent_entry = None;
        self.replace_on_next_digit = false;
        self.entry_negative = false;
        self.set_value(v);
    }

    pub fn toggle_fe(&mut self) {
        if self.error.is_some() { return; }
        self.repeat_binary = None;
        let v=self.value().unwrap_or(0.0); self.force_exp=!self.force_exp; self.set_value(v);
    }

    fn raw_entry(&self) -> String {
        if self.error.is_some() { return "0".into(); }
        let mut s = self.number_locale.canonicalize_display(self.display.trim());
        if s.ends_with('.') && !self.decimal_entered { s.pop(); }
        if s.is_empty() || s == "-" { "0".into() } else { s }
    }

    pub fn eval_context(&self) -> EvalContext {
        EvalContext {
            angle: self.angle,
            decimal_separator: self.number_locale.decimal_separator(),
            thousands_separator: self.number_locale.thousands_separator(),
        }
    }

    pub fn decimal_separator(&self) -> char {
        self.number_locale.decimal_separator()
    }

    /// Snapshot the pending standard-mode binary expression in the exact
    /// numeric notation currently visible to the user.  The UI uses this only
    /// to populate the optional calculation-history panel; arithmetic remains
    /// entirely inside Calculator.
    pub(crate) fn pending_standard_history_parts(&self) -> Option<(BinaryOp, String, String)> {
        if self.mode != Mode::Standard || self.error.is_some() {
            return None;
        }
        let (op, rhs) = if let Some(op) = self.pending {
            (op, self.value().ok()?)
        } else {
            self.repeat_binary?
        };
        Some((
            op,
            format_value(self.accumulator, self.base, self.word_size, self.force_exp, self.number_locale),
            format_value(rhs, self.base, self.word_size, self.force_exp, self.number_locale),
        ))
    }

    pub(crate) fn is_entering_value(&self) -> bool {
        self.entering
    }

    /// Build, without mutating state, the expression Scientific mode would
    /// evaluate if '=' were pressed now.  This mirrors equals(): it appends the
    /// current entry when necessary and closes any still-open parentheses.
    pub(crate) fn pending_scientific_history_expression(&self) -> Option<String> {
        if self.mode != Mode::Scientific || self.error.is_some() {
            return None;
        }
        let mut expression = self.scientific_expr.clone();
        if self.entering || expression.is_empty() {
            expression.push_str(&self.expression_entry());
        } else if trailing_scientific_binary_operator(&expression).is_some() {
            // `=` after a pending Scientific operator reuses the displayed left
            // operand as the missing RHS, so History should preview the actual
            // expression that will be evaluated rather than an incomplete `2+`.
            expression.push_str(&self.expression_entry());
        }
        for outer in self.paren_frames.iter().rev() {
            expression = format!("{outer}({expression})");
        }
        if expression.trim().is_empty() {
            None
        } else {
            Some(strip_based_literal_prefixes(&expression, self.base))
        }
    }

    /// Change only the user-facing decimal convention.  Preserve the current
    /// display exactly in canonical form so switching punctuation never changes
    /// the numeric value, entry state, exponent, or trailing radix marker.
    pub fn set_decimal_separator(&mut self, separator: char) {
        let canonical_display = if self.error.is_none() {
            Some(self.number_locale.canonicalize_display(&self.display))
        } else {
            None
        };
        self.number_locale = NumericLocale::with_decimal_separator(separator);
        if let Some(canonical) = canonical_display {
            self.display = self.number_locale.localize(&canonical);
        }
    }

    pub fn format_decimal_value(&self, value: f64) -> String {
        format_value(value, Base::Dec, WordSize::Dword, false, self.number_locale)
    }

    fn push_current_into_expr(&mut self) {
        if self.entering || self.scientific_expr.is_empty() {
            let s = self.expression_entry();
            self.scientific_expr.push_str(&s);
            self.expression_entry_override = None;
        }
    }

    fn expression_entry(&self) -> String {
        if let Some(expression) = &self.expression_entry_override {
            return expression.clone();
        }
        if let Some(state) = &self.exponent_entry {
            let sign = if state.negative { '-' } else { '+' };
            // Keep the classic trailing-dot form purely visual.  The
            // expression parser receives an unambiguous canonical literal.
            return format!("{}e{}{:03}", state.mantissa, sign, state.exponent);
        }
        if self.base == Base::Dec {
            return self.raw_entry();
        }
        let value = self.value().unwrap_or(0.0);
        based_expression_literal(value, self.base)
    }

    fn fail(&mut self, s:&str) { self.error=Some(s.to_string()); self.display=s.to_string(); self.pending=None; self.repeat_binary=None; self.decimal_entered=false; self.scientific_expr.clear(); self.paren_frames.clear(); self.exponent_entry=None; self.expression_entry_override=None; self.entry_negative=false; }
}

fn trailing_scientific_binary_operator(expression: &str) -> Option<BinaryOp> {
    const OPS: [(&str, BinaryOp); 12] = [
        (" root ", BinaryOp::Root),
        (" and ", BinaryOp::And),
        (" xor ", BinaryOp::Xor),
        (" lsh ", BinaryOp::Lsh),
        (" rsh ", BinaryOp::Rsh),
        (" mod ", BinaryOp::Mod),
        (" or ", BinaryOp::Or),
        ("+", BinaryOp::Add),
        ("-", BinaryOp::Sub),
        ("*", BinaryOp::Mul),
        ("/", BinaryOp::Div),
        ("^", BinaryOp::Pow),
    ];
    for (suffix, op) in OPS {
        if expression.ends_with(suffix) {
            return Some(op);
        }
    }
    None
}

fn replace_trailing_scientific_operator(expression: &mut String, replacement: BinaryOp) -> bool {
    // Long textual operators must be tested before one-character operators.
    const OPS: [&str; 11] = [" root ", " and ", " or ", " xor ", " lsh ", " rsh ", " mod ", "+", "-", "*", "/"];
    for old in OPS {
        if expression.ends_with(old) {
            expression.truncate(expression.len() - old.len());
            expression.push_str(op_text(replacement));
            return true;
        }
    }
    if expression.ends_with('^') {
        expression.pop();
        expression.push_str(op_text(replacement));
        return true;
    }
    false
}

fn op_text(op: BinaryOp) -> &'static str { match op { BinaryOp::Add=>"+", BinaryOp::Sub=>"-", BinaryOp::Mul=>"*", BinaryOp::Div=>"/", BinaryOp::Mod=>" mod ", BinaryOp::Pow=>"^", BinaryOp::Root=>" root ", BinaryOp::And=>" and ", BinaryOp::Or=>" or ", BinaryOp::Xor=>" xor ", BinaryOp::Lsh=>" lsh ", BinaryOp::Rsh=>" rsh " } }

fn low_dword(value: f64) -> u32 {
    // CALC.EXE truncates through an x87 FISTP QWORD and then consumes EAX.
    // All callers have already applied the original unsigned-DWORD magnitude
    // guard, so the f64 -> i64 conversion is exact for this domain.
    (value.trunc() as i64) as u32
}

fn signed_dword(value: u32) -> f64 {
    // Bitwise operations store EAX with FILD DWORD, i.e. as signed 32-bit.
    (value as i32) as f64
}

fn apply_binary(op: BinaryOp, a: f64, b: f64) -> Result<f64, String> {
    match op {
        BinaryOp::Add => checked_add_sub(a + b),
        BinaryOp::Sub => checked_add_sub(a - b),
        BinaryOp::Mul => {
            if a != 0.0 && b != 0.0 && a.abs().log10() + b.abs().log10() > 307.0 {
                return Err(RESULT_TOO_LARGE.into());
            }
            checked_large(a * b)
        }
        BinaryOp::Div => {
            if b == 0.0 { return Err(DIVIDE_BY_ZERO.into()); }
            if a != 0.0 && a.abs().log10() - b.abs().log10() > 307.0 {
                return Err(RESULT_TOO_LARGE.into());
            }
            checked_large(a / b)
        }
        BinaryOp::Mod => {
            if b == 0.0 { return Err(DIVIDE_BY_ZERO.into()); }
            checked_large(a % b)
        }
        BinaryOp::Pow => checked_pow(a, b),
        BinaryOp::Root => {
            if b == 0.0 { Err(INVALID_FUNCTION_INPUT.into()) } else { checked_pow(a, 1.0 / b) }
        }
        BinaryOp::And | BinaryOp::Or | BinaryOp::Xor | BinaryOp::Lsh | BinaryOp::Rsh => {
            // IDs 0x56..0x59 share the recovered unsigned-DWORD magnitude
            // guard before entering the individual bitwise operation.
            if a.abs() > u32::MAX as f64 || b.abs() > u32::MAX as f64 {
                return Err(RESULT_TOO_LARGE.into());
            }
            let lhs = low_dword(a);
            let rhs = low_dword(b);
            let value = match op {
                BinaryOp::And => lhs & rhs,
                BinaryOp::Or => lhs | rhs,
                BinaryOp::Xor => lhs ^ rhs,
                // The reference executable executes a 32-bit x86 SHL; the CPU
                // masks CL to five bits, so a count of 32 behaves as zero.
                BinaryOp::Lsh => lhs.wrapping_shl(rhs & 31),
                // Internal operation 7 executes x86 SAR.  Treat the low DWORD
                // as signed before shifting so the sign bit is extended.  As
                // with SHL, x86 masks the count to five bits.
                BinaryOp::Rsh => ((lhs as i32) >> (rhs & 31)) as u32,
                _ => unreachable!(),
            };
            Ok(signed_dword(value))
        }
    }
}

fn checked_add_sub(v: f64) -> Result<f64, String> {
    if v.is_nan() {
        Err(FUNCTION_UNDEFINED.into())
    } else if v.is_infinite() || v.abs() > 1.0e308 {
        Err(RESULT_TOO_LARGE.into())
    } else {
        Ok(v)
    }
}

fn checked_large(v: f64) -> Result<f64, String> {
    if v.is_nan() {
        Err(FUNCTION_UNDEFINED.into())
    } else if v.is_infinite() {
        Err(RESULT_TOO_LARGE.into())
    } else {
        Ok(v)
    }
}

fn checked_exp_like(v: f64) -> Result<f64, String> {
    if v.is_infinite() {
        Err(RESULT_TOO_LARGE.into())
    } else if v == 0.0 {
        Err(RESULT_TOO_SMALL.into())
    } else {
        Ok(v)
    }
}

fn checked_pow(base: f64, exponent: f64) -> Result<f64, String> {
    let v = base.powf(exponent);
    if v.is_nan() {
        Err(INVALID_FUNCTION_INPUT.into())
    } else if v.is_infinite() {
        Err(RESULT_TOO_LARGE.into())
    } else if v == 0.0 && base != 0.0 {
        Err(RESULT_TOO_SMALL.into())
    } else {
        Ok(v)
    }
}

fn zero_display(base: Base, locale: NumericLocale) -> String {
    if base == Base::Dec {
        locale.localize("0.")
    } else {
        "0".into()
    }
}

fn entry_display(s: &str, base: Base, locale: NumericLocale) -> String {
    if base == Base::Dec {
        let canonical = if s.contains('.') { s.to_string() } else { format!("{s}.") };
        locale.localize(&canonical)
    } else {
        s.to_string()
    }
}

fn format_value(v: f64, base: Base, word_size: WordSize, force_exp: bool, locale: NumericLocale) -> String {
    if base != Base::Dec {
        let n = (low_dword(v) & word_size.mask()) as u64;
        let s = match base {
            Base::Hex => format!("{:X}", n),
            Base::Oct => format!("{:o}", n),
            Base::Bin => format!("{:b}", n),
            Base::Dec => unreachable!(),
        };
        return s;
    }
    if v == 0.0 {
        return locale.localize("0.");
    }
    locale.localize(&format_decimal_13(v, force_exp))
}

/// Reproduce the Win95 display routine's 13-significant-digit decimal
/// formatting without reducing Calculator's internal f64 precision.
///
/// CALC.EXE first converts the magnitude to a 13-digit decimal record.  In
/// general mode it keeps fixed notation while the decimal exponent is below
/// 13 and the fixed representation needs at most 16 places from the first
/// significant digit through the decimal point; otherwise it switches to
/// scientific notation. F-E bypasses that decision and always uses the same
/// 13-digit scientific record.
fn format_decimal_13(value: f64, force_exp: bool) -> String {
    debug_assert!(value != 0.0);
    let negative = value.is_sign_negative();
    let normalized = format!("{:.12e}", value.abs());
    let (mantissa, exponent_text) = normalized
        .split_once('e')
        .expect("Rust scientific formatting always contains e");
    let exponent: i32 = exponent_text.parse().expect("valid scientific exponent");

    let mut digits: String = mantissa.chars().filter(|&ch| ch != '.').collect();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }

    let use_exp = force_exp || exponent >= 13 || (digits.len() as i32 - exponent) > 16;
    let mut out = String::new();
    if negative { out.push('-'); }

    if use_exp {
        let mut chars = digits.chars();
        out.push(chars.next().unwrap_or('0'));
        out.push('.');
        out.extend(chars);
        out.push('e');
        if exponent < 0 {
            out.push('-');
        } else {
            out.push('+');
        }
        // The original display template is e+000/e-000.  f64's decimal
        // exponent range fits in three columns, so pad every scientific
        // exponent instead of emitting Rust's variable-width form.
        out.push_str(&format!("{:03}", exponent.abs()));
        return out;
    }

    if exponent < 0 {
        out.push_str("0.");
        for _ in 0..(-exponent - 1) { out.push('0'); }
        out.push_str(&digits);
    } else {
        let decimal_pos = exponent as usize + 1;
        if decimal_pos >= digits.len() {
            out.push_str(&digits);
            for _ in digits.len()..decimal_pos { out.push('0'); }
            out.push('.');
        } else {
            out.push_str(&digits[..decimal_pos]);
            out.push('.');
            out.push_str(&digits[decimal_pos..]);
        }
    }
    out
}

fn split_dms_secondary(total: f64) -> (f64, f64) {
    let mut whole = total.floor();
    let mut fraction = total - whole;
    // CALC.EXE compares only this second modf() remainder with the recovered
    // 0.99999999999 constant. If it is above the threshold, increment the
    // integer component and zero this remainder; do not add further carries.
    if fraction > 0.999_999_999_99 {
        whole += 1.0;
        fraction = 0.0;
    }
    (whole, fraction)
}

fn based_expression_literal(value: f64, base: Base) -> String {
    let integer = value as i64;
    let (sign, magnitude) = if integer < 0 {
        ("-", integer.unsigned_abs())
    } else {
        ("", integer as u64)
    };
    match base {
        Base::Hex => format!("{sign}0x{magnitude:X}"),
        Base::Oct => format!("{sign}0o{magnitude:o}"),
        Base::Bin => format!("{sign}0b{magnitude:b}"),
        Base::Dec => value.to_string(),
    }
}


fn strip_based_literal_prefixes(expression: &str, base: Base) -> String {
    match base {
        Base::Hex => expression.replace("0x", "").replace("0X", ""),
        Base::Oct => expression.replace("0o", "").replace("0O", ""),
        Base::Bin => expression.replace("0b", "").replace("0B", ""),
        Base::Dec => expression.to_string(),
    }
}

fn qualify_bare_based_numbers(expression: &str, base: Base) -> String {
    if base == Base::Dec {
        return expression.to_string();
    }

    fn explicit_based_literal(token: &str) -> bool {
        let bytes = token.as_bytes();
        bytes.len() > 2
            && bytes[0] == b'0'
            && matches!(bytes[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B')
    }

    fn valid_digit(ch: char, base: Base) -> bool {
        match base {
            Base::Hex => ch.is_ascii_hexdigit(),
            Base::Oct => matches!(ch, '0'..='7'),
            Base::Bin => matches!(ch, '0' | '1'),
            Base::Dec => ch.is_ascii_digit(),
        }
    }

    let prefix = match base {
        Base::Hex => "0x",
        Base::Oct => "0o",
        Base::Bin => "0b",
        Base::Dec => unreachable!(),
    };
    let mut result = String::with_capacity(expression.len() + 8);
    let mut chars = expression.char_indices().peekable();

    while let Some((start, ch)) = chars.next() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            let mut end = start + ch.len_utf8();
            while let Some(&(index, next)) = chars.peek() {
                if next.is_ascii_alphanumeric() || next == '_' {
                    chars.next();
                    end = index + next.len_utf8();
                } else {
                    break;
                }
            }
            let token = &expression[start..end];
            if !explicit_based_literal(token)
                && token.chars().all(|digit| valid_digit(digit, base))
            {
                result.push_str(prefix);
            }
            result.push_str(token);
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod locale_tests {
    use super::*;

    #[test]
    fn comma_locale_uses_comma_in_display_but_period_in_arithmetic() {
        let mut calc = Calculator::default();
        calc.number_locale = NumericLocale::new(',', Some('.'));
        calc.clear_all();
        assert_eq!(calc.display, "0,");
        calc.digit('1');
        calc.decimal_point();
        calc.digit('5');
        assert_eq!(calc.display, "1,5");
        assert!((calc.value().unwrap() - 1.5).abs() < 1e-12);
    }

    #[test]
    fn explicit_decimal_survives_the_classic_trailing_separator_display() {
        let mut calc = Calculator::default();
        calc.number_locale = NumericLocale::new(',', Some('.'));
        calc.clear_all();
        calc.digit('3');
        assert_eq!(calc.display, "3,");
        calc.decimal_point();
        // The display is intentionally unchanged here: classic Calculator already
        // shows the radix after an integer. Internal state must still remember that
        // this comma was explicitly entered.
        assert_eq!(calc.display, "3,");
        calc.digit('2');
        assert_eq!(calc.display, "3,2");
        assert!((calc.value().unwrap() - 3.2).abs() < 1e-12);
    }

    #[test]
    fn explicit_period_survives_the_classic_trailing_separator_display() {
        let mut calc = Calculator::default();
        calc.number_locale = NumericLocale::new('.', Some(','));
        calc.clear_all();
        calc.digit('3');
        calc.decimal_point();
        calc.digit('2');
        assert_eq!(calc.display, "3.2");
        assert!((calc.value().unwrap() - 3.2).abs() < 1e-12);
    }

    #[test]
    fn period_locale_uses_period_in_display() {
        let locale = NumericLocale::new('.', Some(','));
        assert_eq!(format_value(12.5, Base::Dec, WordSize::Dword, false, locale), "12.5");
    }

    #[test]
    fn pasted_period_or_comma_is_accepted_under_comma_locale() {
        let mut calc = Calculator::default();
        calc.number_locale = NumericLocale::new(',', Some('.'));
        calc.paste_expression("1.5 + 2");
        assert_eq!(calc.display, "3,5");
        calc.paste_expression("1,5 + 2");
        assert_eq!(calc.display, "3,5");
    }

    #[test]
    fn scientific_selector_model_state_changes_are_effective() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        calc.set_base(Base::Hex);
        assert_eq!(calc.base, Base::Hex);
        calc.digit('A');
        assert_eq!(calc.display, "A");

        calc.set_base(Base::Dec);
        calc.angle = AngleMode::Degrees;
        calc.set_value(90.0);
        calc.unary("sin");
        assert!((calc.value().unwrap() - 1.0).abs() < 1e-10);

        calc.inv = true;
        calc.hyp = false;
        calc.set_value(0.5);
        calc.unary("sin");
        assert!((calc.value().unwrap() - 30.0).abs() < 1e-10);

        calc.inv = false;
        calc.hyp = true;
        calc.set_value(0.0);
        calc.unary("cos");
        assert!((calc.value().unwrap() - 1.0).abs() < 1e-10);
    }

    #[test]
    fn changing_decimal_separator_preserves_current_value_and_entry_state() {
        let mut calc = Calculator::default();
        calc.number_locale = NumericLocale::new('.', Some(','));
        calc.clear_all();
        calc.digit('1');
        calc.decimal_point();
        calc.digit('5');
        assert_eq!(calc.display, "1.5");

        calc.set_decimal_separator(',');
        assert_eq!(calc.display, "1,5");
        assert_eq!(calc.value().unwrap(), 1.5);

        calc.digit('2');
        assert_eq!(calc.display, "1,52");
        assert_eq!(calc.value().unwrap(), 1.52);
    }

    #[test]
    fn win95_unary_errors_use_the_original_categories() {
        let mut calc = Calculator::default();

        calc.set_value(-1.0);
        calc.unary("sqrt");
        assert_eq!(calc.display, FUNCTION_UNDEFINED);

        calc.clear_all();
        calc.set_value(-1.0);
        calc.unary("factorial");
        assert_eq!(calc.display, INVALID_FUNCTION_INPUT);

        calc.clear_all();
        calc.set_value(171.0);
        calc.unary("factorial");
        assert_eq!(calc.display, RESULT_TOO_LARGE);

        calc.clear_all();
        calc.set_value(1.1e154);
        calc.unary("square");
        assert_eq!(calc.display, RESULT_TOO_LARGE);

        calc.clear_all();
        calc.set_value(1.1e102);
        calc.unary("cube");
        assert_eq!(calc.display, RESULT_TOO_LARGE);

        calc.clear_all();
        calc.set_value(4_294_967_296.0);
        calc.unary("not");
        assert_eq!(calc.display, INVALID_FUNCTION_INPUT);
    }

    #[test]
    fn win95_statistics_errors_match_the_original() {
        let mut calc = Calculator::default();
        calc.stat_avg();
        assert_eq!(calc.display, DIVIDE_BY_ZERO);

        calc.clear_all();
        calc.set_value(42.0);
        calc.stat_dat();
        calc.stat_stddev();
        assert_eq!(calc.value().unwrap(), 0.0);
    }

    #[test]
    fn win95_dat_makes_the_next_number_a_fresh_entry_in_both_modes() {
        for mode in [Mode::Standard, Mode::Scientific] {
            let mut calc = Calculator::default();
            calc.set_mode(mode);

            calc.digit('1');
            calc.digit('2');
            calc.stat_dat();
            assert_eq!(calc.stats, vec![12.0]);
            assert_eq!(calc.value().unwrap(), 12.0);

            calc.digit('3');
            assert_eq!(calc.value().unwrap(), 3.0);
            calc.stat_dat();
            assert_eq!(calc.stats, vec![12.0, 3.0]);

            calc.digit('4');
            assert_eq!(calc.value().unwrap(), 4.0);
        }
    }

    #[test]
    fn every_statistics_command_arms_a_fresh_numeric_entry() {
        let mut calc = Calculator::default();
        calc.digit('2');
        calc.stat_dat();
        calc.digit('4');
        calc.stat_dat();

        for action in [
            Calculator::stat_sum as fn(&mut Calculator),
            Calculator::stat_avg,
            Calculator::stat_stddev,
        ] {
            calc.digit('9');
            action(&mut calc);
            calc.digit('7');
            assert_eq!(calc.value().unwrap(), 7.0);
        }
    }

    #[test]
    fn non_decimal_range_uses_large_and_small_messages_by_sign() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_value(4_294_967_296.0);
        calc.set_base(Base::Hex);
        assert_eq!(calc.display, RESULT_TOO_LARGE);

        calc.clear_all();
        calc.set_value(-4_294_967_296.0);
        calc.set_base(Base::Hex);
        assert_eq!(calc.display, RESULT_TOO_SMALL);
    }
    #[test]
    fn win95_binary_overflow_guards_are_preserved() {
        assert_eq!(apply_binary(BinaryOp::Mul, 1.0e154, 1.0e154).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(apply_binary(BinaryOp::Div, 1.0e307, 0.1).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(apply_binary(BinaryOp::Add, 6.0e307, 6.0e307).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(apply_binary(BinaryOp::And, 4_294_967_296.0, 1.0).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(apply_binary(BinaryOp::Mod, 1.0, 0.0).unwrap_err(), DIVIDE_BY_ZERO);
    }

    #[test]
    fn keyboard_double_star_is_exponentiation_in_both_modes() {
        let mut standard = Calculator::default();
        standard.digit('2');
        standard.keyboard_star();
        standard.keyboard_star();
        standard.digit('3');
        standard.equals();
        assert!((standard.value().unwrap() - 8.0).abs() < 1.0e-12);

        let mut scientific = Calculator::default();
        scientific.set_mode(Mode::Scientific);
        scientific.digit('2');
        scientific.keyboard_star();
        scientific.keyboard_star();
        scientific.digit('3');
        scientific.equals();
        assert!((scientific.value().unwrap() - 8.0).abs() < 1.0e-12);
    }

    #[test]
    fn inverse_power_uses_root_semantics_and_zero_is_invalid() {
        assert!((apply_binary(BinaryOp::Root, 27.0, 3.0).unwrap() - 3.0).abs() < 1.0e-12);
        assert_eq!(
            apply_binary(BinaryOp::Root, 8.0, 0.0).unwrap_err(),
            INVALID_FUNCTION_INPUT
        );

        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.digit('8');
        calc.inv = true;
        calc.binary(BinaryOp::Pow);
        assert!(!calc.inv);
        calc.digit('0');
        calc.equals();
        assert_eq!(calc.display, INVALID_FUNCTION_INPUT);
    }


    #[test]
    fn recalled_history_result_becomes_a_fresh_left_operand() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.angle = AngleMode::Radians;
        calc.memory = 12.0;
        calc.stats = vec![1.0, 2.0, 3.0];

        calc.recall_history_value(52.8);
        assert!((calc.value().unwrap() - 52.8).abs() < 1.0e-12);
        assert!(calc.pending.is_none());
        assert!(!calc.entering);
        assert_eq!(calc.mode, Mode::Scientific);
        assert_eq!(calc.angle, AngleMode::Radians);
        assert_eq!(calc.memory, 12.0);
        assert_eq!(calc.stats, vec![1.0, 2.0, 3.0]);

        calc.binary(BinaryOp::Add);
        calc.digit('2');
        calc.equals();
        assert!((calc.value().unwrap() - 54.8).abs() < 1.0e-12);
    }

    #[test]
    fn memory_add_uses_the_original_addition_overflow_guard() {
        let mut calc = Calculator::default();
        calc.memory = 6.0e307;
        calc.set_value(6.0e307);
        calc.memory_add();
        assert_eq!(calc.display, RESULT_TOO_LARGE);
    }

    #[test]
    fn octal_arithmetic_uses_octal_operands_in_scientific_mode() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Oct);
        for digit in "777".chars() { calc.digit(digit); }
        calc.binary(BinaryOp::Add);
        for digit in "77".chars() { calc.digit(digit); }

        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("777+77"));
        calc.equals();
        assert_eq!(calc.display, "1076");
        assert_eq!(calc.value().unwrap(), 574.0);
    }

    #[test]
    fn pasted_octal_expression_uses_the_selected_base() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Oct);
        calc.paste_expression("777+77");
        assert_eq!(calc.display, "1076");
        assert_eq!(calc.value().unwrap(), 574.0);
    }

    #[test]
    fn hexadecimal_digit_only_arithmetic_uses_hex_operands_in_scientific_mode() {
        // Use operands containing only decimal-looking digits so this catches
        // the old bug where Scientific mode parsed them as base 10 and only
        // converted the final result to hexadecimal.
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        for digit in "777".chars() { calc.digit(digit); }
        calc.binary(BinaryOp::Add);
        for digit in "77".chars() { calc.digit(digit); }

        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("777+77"));
        calc.equals();
        assert_eq!(calc.display, "7EE");
        assert_eq!(calc.value().unwrap(), 0x7EE as f64);
    }

    #[test]
    fn hexadecimal_letter_digits_are_preserved_too() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.digit('F');
        calc.digit('F');
        calc.binary(BinaryOp::Add);
        calc.digit('1');
        calc.equals();
        assert_eq!(calc.display, "100");
        assert_eq!(calc.value().unwrap(), 256.0);
    }

    #[test]
    fn pasted_hexadecimal_expression_uses_the_selected_base() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.paste_expression("777+77");
        assert_eq!(calc.display, "7EE");
        assert_eq!(calc.value().unwrap(), 0x7EE as f64);
    }

    #[test]
    fn binary_arithmetic_uses_binary_operands_in_scientific_mode() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Bin);
        for digit in "111".chars() { calc.digit(digit); }
        calc.binary(BinaryOp::Add);
        for digit in "11".chars() { calc.digit(digit); }

        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("111+11"));
        calc.equals();
        assert_eq!(calc.display, "1010");
        assert_eq!(calc.value().unwrap(), 10.0);
    }

    #[test]
    fn pasted_binary_expression_uses_the_selected_base() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Bin);
        calc.paste_expression("111+11");
        assert_eq!(calc.display, "1010");
        assert_eq!(calc.value().unwrap(), 10.0);
    }

    #[test]
    fn word_and_byte_are_low_bit_views_without_changing_the_value() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.set_value(0x12345 as f64);
        assert_eq!(calc.display, "12345");

        calc.set_word_size(WordSize::Word);
        assert_eq!(calc.display, "2345");
        assert_eq!(calc.value().unwrap(), 0x12345 as f64);

        calc.set_word_size(WordSize::Byte);
        assert_eq!(calc.display, "45");
        assert_eq!(calc.value().unwrap(), 0x12345 as f64);

        calc.set_word_size(WordSize::Dword);
        assert_eq!(calc.display, "12345");
        assert_eq!(calc.value().unwrap(), 0x12345 as f64);
    }

    #[test]
    fn direct_entry_is_masked_but_keeps_hidden_dword_bits_in_all_non_decimal_bases() {
        let cases = [
            (Base::Hex, "12345", "2345", "12345"),
            (Base::Oct, "200000", "0", "200000"),
            (Base::Bin, "10000000000000000", "0", "10000000000000000"),
        ];
        for (base, input, word_display, dword_display) in cases {
            let mut calc = Calculator::default();
            calc.set_mode(Mode::Scientific);
            calc.set_base(base);
            calc.set_word_size(WordSize::Word);
            for ch in input.chars() { calc.digit(ch); }
            assert_eq!(calc.display, word_display, "base {base:?}");
            let full = calc.value().unwrap();

            calc.set_word_size(WordSize::Dword);
            assert_eq!(calc.display, dword_display, "base {base:?}");
            assert_eq!(calc.value().unwrap(), full, "base {base:?}");
        }
    }

    #[test]
    fn byte_masks_are_base_correct() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.set_word_size(WordSize::Byte);
        calc.set_value(0x1ff as f64);
        assert_eq!(calc.display, "FF");

        calc.set_base(Base::Oct);
        assert_eq!(calc.display, "377");

        calc.set_base(Base::Bin);
        assert_eq!(calc.display, "11111111");
        assert_eq!(calc.value().unwrap(), 0x1ff as f64);
    }

    #[test]
    fn word_and_byte_arithmetic_masks_are_identical_in_hex_oct_and_bin() {
        let word_cases = [
            (Base::Hex, "FFFF", "2", "FFFE"),
            (Base::Oct, "177777", "2", "177776"),
            (Base::Bin, "1111111111111111", "10", "1111111111111110"),
        ];
        for (base, lhs, rhs, expected) in word_cases {
            let mut calc = Calculator::default();
            calc.set_mode(Mode::Scientific);
            calc.set_base(base);
            calc.set_word_size(WordSize::Word);
            for ch in lhs.chars() { calc.digit(ch); }
            calc.binary(BinaryOp::Mul);
            for ch in rhs.chars() { calc.digit(ch); }
            calc.equals();
            assert_eq!(calc.display, expected, "Word arithmetic in {base:?}");
            assert_eq!(calc.value().unwrap(), 131_070.0, "Word full value in {base:?}");
        }

        let byte_cases = [
            (Base::Hex, "FF", "1"),
            (Base::Oct, "377", "1"),
            (Base::Bin, "11111111", "1"),
        ];
        for (base, lhs, rhs) in byte_cases {
            let mut calc = Calculator::default();
            calc.set_mode(Mode::Scientific);
            calc.set_base(base);
            calc.set_word_size(WordSize::Byte);
            for ch in lhs.chars() { calc.digit(ch); }
            calc.binary(BinaryOp::Add);
            for ch in rhs.chars() { calc.digit(ch); }
            calc.equals();
            assert_eq!(calc.display, "0", "Byte arithmetic in {base:?}");
            assert_eq!(calc.value().unwrap(), 256.0, "Byte full value in {base:?}");
        }
    }

    #[test]
    fn word_overflow_is_visual_only_but_dword_overflow_is_an_error() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.set_word_size(WordSize::Word);
        for ch in "FFFF".chars() { calc.digit(ch); }
        calc.binary(BinaryOp::Mul);
        calc.digit('2');
        calc.equals();
        assert_eq!(calc.display, "FFFE");
        assert_eq!(calc.value().unwrap(), 0x1fffe as f64);

        calc.set_word_size(WordSize::Dword);
        assert_eq!(calc.display, "1FFFE");

        calc.clear_all();
        for ch in "FFFFFFFF".chars() { calc.digit(ch); }
        calc.binary(BinaryOp::Add);
        calc.digit('1');
        calc.equals();
        assert_eq!(calc.display, RESULT_TOO_LARGE);
    }

    #[test]
    fn non_decimal_sign_uses_twos_complement_at_the_selected_width() {
        let cases = [
            (Base::Hex, "FF"),
            (Base::Oct, "377"),
            (Base::Bin, "11111111"),
        ];
        for (base, expected) in cases {
            let mut calc = Calculator::default();
            calc.set_mode(Mode::Scientific);
            calc.set_base(base);
            calc.set_word_size(WordSize::Byte);
            calc.digit('1');
            calc.sign();
            assert_eq!(calc.display, expected, "base {base:?}");
            assert_eq!(calc.value().unwrap(), -1.0, "base {base:?}");
        }
    }

    #[test]
    fn bitwise_operations_are_signed_32_bit_like_calc_exe() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        for ch in "80000000".chars() { calc.digit(ch); }
        calc.binary(BinaryOp::Or);
        calc.digit('0');
        calc.equals();
        assert_eq!(calc.display, "80000000");
        assert_eq!(calc.value().unwrap(), i32::MIN as f64);

        calc.set_word_size(WordSize::Byte);
        calc.clear_all();
        calc.digit('1');
        calc.unary("not");
        assert_eq!(calc.display, "FE");
        assert_eq!(calc.value().unwrap(), -2.0);
        calc.set_word_size(WordSize::Dword);
        assert_eq!(calc.display, "FFFFFFFE");
    }

    #[test]
    fn inverse_lsh_is_signed_right_shift_and_consumes_inv() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        for ch in "80000000".chars() { calc.digit(ch); }
        calc.inv = true;
        calc.binary(BinaryOp::Lsh);
        assert!(!calc.inv);
        calc.digit('1');
        calc.equals();
        assert_eq!(calc.display, "C0000000");
        assert_eq!(calc.value().unwrap(), -1_073_741_824.0);

        // The x86 shift count comes from CL, so only its low five bits count.
        assert_eq!(apply_binary(BinaryOp::Rsh, -16.0, 2.0).unwrap(), -4.0);
        assert_eq!(apply_binary(BinaryOp::Rsh, 1.0, 32.0).unwrap(), 1.0);
    }

    #[test]
    fn non_decimal_results_are_floored_before_dword_validation_and_masking() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        calc.set_value(-1.1);
        calc.set_base(Base::Hex);
        assert_eq!(calc.display, "FFFFFFFE");
        assert_eq!(calc.value().unwrap(), -2.0);
        calc.set_base(Base::Dec);
        assert_eq!(calc.value().unwrap(), -2.0);

        // floor() occurs before the 0xFFFFFFFF magnitude check in CALC.EXE.
        calc.set_value(4_294_967_295.9);
        calc.set_base(Base::Hex);
        assert_eq!(calc.display, "FFFFFFFF");
        assert_eq!(calc.value().unwrap(), 4_294_967_295.0);

        calc.set_base(Base::Dec);
        calc.set_value(-4_294_967_295.1);
        calc.set_base(Base::Hex);
        assert_eq!(calc.display, RESULT_TOO_SMALL);
    }

    #[test]
    fn non_decimal_division_stores_the_integerized_result_not_a_hidden_fraction() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.digit('5');
        calc.binary(BinaryOp::Div);
        calc.digit('2');
        calc.equals();
        assert_eq!(calc.display, "2");
        assert_eq!(calc.value().unwrap(), 2.0);
        calc.set_base(Base::Dec);
        assert_eq!(calc.value().unwrap(), 2.0);
    }

    #[test]
    fn backspace_edits_the_hidden_full_entry_not_the_masked_text() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.set_word_size(WordSize::Word);
        for ch in "12345".chars() { calc.digit(ch); }
        assert_eq!(calc.display, "2345");
        assert_eq!(calc.value().unwrap(), 0x12345 as f64);

        calc.backspace();
        assert_eq!(calc.display, "1234");
        assert_eq!(calc.value().unwrap(), 0x1234 as f64);
    }

    #[test]
    fn radix_selection_commits_before_non_decimal_conversion_error() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_value(4_294_967_296.0);
        calc.set_base(Base::Hex);
        assert_eq!(calc.base, Base::Hex);
        assert_eq!(calc.display, RESULT_TOO_LARGE);

        // Clearing after the failed conversion stays in the newly selected
        // radix, matching CALC.EXE's command-ordering behavior.
        calc.clear_all();
        assert_eq!(calc.base, Base::Hex);
        assert_eq!(calc.display, "0");
    }

    #[test]
    fn standard_mode_forces_decimal_without_clearing_the_current_value() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.set_word_size(WordSize::Byte);
        calc.digit('F');
        calc.digit('F');
        assert_eq!(calc.display, "FF");

        calc.set_mode(Mode::Standard);
        assert_eq!(calc.base, Base::Dec);
        assert_eq!(calc.display, "255.");
        assert_eq!(calc.value().unwrap(), 255.0);
        // The width selection is merely hidden in Standard and can be retained
        // for the next Scientific non-decimal session.
        assert_eq!(calc.word_size, WordSize::Byte);

        calc.set_mode(Mode::Scientific);
        assert_eq!(calc.base, Base::Dec);
        assert_eq!(calc.value().unwrap(), 255.0);
    }

    #[test]
    fn standard_repeated_equals_reuses_the_last_operator_and_rhs() {
        let mut calc = Calculator::default();
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.digit('3');
        calc.equals();
        assert_eq!(calc.value().unwrap(), 5.0);
        assert_eq!(calc.pending_standard_history_parts().map(|(_, lhs, rhs)| (lhs, rhs)), Some(("5.".to_string(), "3.".to_string())));

        calc.equals();
        assert_eq!(calc.value().unwrap(), 8.0);
        calc.equals();
        assert_eq!(calc.value().unwrap(), 11.0);

        // A fresh numeric entry cancels the repeat chain.
        calc.digit('4');
        calc.equals();
        assert_eq!(calc.value().unwrap(), 4.0);
    }

    #[test]
    fn calculator_error_state_is_locked_until_c_or_ce() {
        let mut calc = Calculator::default();
        calc.memory = 7.0;
        calc.memory_set = true;
        calc.digit('1');
        calc.binary(BinaryOp::Div);
        calc.digit('0');
        calc.equals();
        assert_eq!(calc.display, DIVIDE_BY_ZERO);
        assert!(calc.error.is_some());
        assert!(!calc.can_accept_digit('9'));

        calc.digit('9');
        calc.memory_clear();
        calc.toggle_fe();
        assert_eq!(calc.display, DIVIDE_BY_ZERO);
        assert_eq!(calc.memory, 7.0);
        assert!(calc.memory_set);
        assert!(!calc.force_exp);

        calc.clear_entry();
        assert!(calc.error.is_none());
        assert_eq!(calc.display, "0.");
        calc.digit('9');
        assert_eq!(calc.display, "9.");
    }

    #[test]
    fn standard_mode_rejects_non_decimal_base_changes() {
        let mut calc = Calculator::default();
        assert_eq!(calc.mode, Mode::Standard);
        assert_eq!(calc.base, Base::Dec);
        calc.set_base(Base::Hex);
        assert_eq!(calc.base, Base::Dec);
        calc.set_base(Base::Oct);
        assert_eq!(calc.base, Base::Dec);
        calc.set_base(Base::Bin);
        assert_eq!(calc.base, Base::Dec);
    }

    #[test]
    fn scientific_consecutive_binary_operator_replaces_the_pending_operator() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.binary(BinaryOp::Mul);
        calc.digit('3');
        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("2*3"));
        calc.equals();
        assert_eq!(calc.value().unwrap(), 6.0);

        calc.clear_all();
        calc.digit('1');
        calc.digit('0');
        calc.binary(BinaryOp::Mod);
        calc.binary(BinaryOp::Sub);
        calc.digit('3');
        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("10-3"));
        calc.equals();
        assert_eq!(calc.value().unwrap(), 7.0);
    }

    #[test]
    fn decimal_direct_entry_keeps_the_modern_no_legacy_mantissa_cap_behavior() {
        let mut calc = Calculator::default();
        let digits = "1234567890123456789012345678901234567890";
        for ch in digits.chars() {
            assert!(calc.can_accept_digit(ch));
            calc.digit(ch);
        }
        assert_eq!(calc.display, format!("{digits}."));

        calc.clear_all();
        for ch in "123456789012345".chars() { calc.digit(ch); }
        calc.decimal_point();
        for ch in "678901234567890".chars() { calc.digit(ch); }
        assert_eq!(calc.display, "123456789012345.678901234567890");
    }

    #[test]
    fn open_parentheses_are_intentionally_unlimited_but_unmatched_close_is_rejected() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        for _ in 0..100 { calc.open_paren(); }
        assert_eq!(calc.paren_depth(), 100);
        assert!(calc.can_close_paren());
        for _ in 0..100 { calc.close_paren(); }
        assert_eq!(calc.paren_depth(), 0);
        assert!(!calc.can_close_paren());
        calc.close_paren();
        assert_eq!(calc.paren_depth(), 0);
    }

    #[test]
    fn radix_digit_and_decimal_point_acceptance_matches_the_selected_base() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        calc.set_base(Base::Hex);
        assert!(calc.can_accept_digit('F'));
        assert!(calc.can_accept_digit('9'));
        assert!(!calc.can_accept_decimal_point());

        calc.set_base(Base::Oct);
        assert!(calc.can_accept_digit('7'));
        assert!(!calc.can_accept_digit('8'));
        assert!(!calc.can_accept_decimal_point());

        calc.set_base(Base::Bin);
        assert!(calc.can_accept_digit('1'));
        assert!(!calc.can_accept_digit('2'));
        assert!(!calc.can_accept_decimal_point());

        calc.set_base(Base::Dec);
        assert!(calc.can_accept_digit('9'));
        assert!(!calc.can_accept_digit('A'));
        assert!(calc.can_accept_decimal_point());
        calc.decimal_point();
        assert!(!calc.can_accept_decimal_point());
    }

    #[test]
    fn pi_is_decimal_only_and_inverse_pi_is_two_pi() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        assert!(!calc.can_use_pi());
        assert!(!calc.pi());
        assert_eq!(calc.display, "0");

        calc.set_base(Base::Dec);
        calc.inv = true;
        assert!(calc.pi());
        assert!(!calc.inv);
        assert!((calc.value().unwrap() - 2.0 * std::f64::consts::PI).abs() < 1.0e-12);
        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("2*pi"));
    }

    #[test]
    fn exp_is_exponent_entry_not_e_to_the_x() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        // CALC.EXE has a special clear-state path with an implicit mantissa 1.
        assert!(calc.exponent_entry());
        assert_eq!(calc.display, "1.e+000");
        // The three visible columns are a rolling numeric field, not a
        // three-keystroke limit; leading zeroes remain valid indefinitely.
        for _ in 0..5 { assert!(calc.can_accept_digit('0')); calc.digit('0'); }
        assert_eq!(calc.display, "1.e+000");
        calc.digit('2');
        assert_eq!(calc.display, "1.e+002");
        calc.sign();
        assert_eq!(calc.display, "1.e-002");
        assert!((calc.value().unwrap() - 0.01).abs() < 1.0e-15);
        calc.backspace();
        assert_eq!(calc.display, "1.e-000");

        calc.clear_all();
        calc.digit('1');
        calc.digit('2');
        assert!(calc.exponent_entry());
        calc.digit('3');
        assert_eq!(calc.display, "12.e+003");
        calc.equals();
        assert_eq!(calc.value().unwrap(), 12_000.0);

        // Exp is also valid on a completed displayed result; only an actually
        // empty clear buffer gets the implicit mantissa 1.
        assert!(calc.exponent_entry());
        assert_eq!(calc.display, "12000.e+000");
    }

    #[test]
    fn exp_positive_exponent_is_limited_to_289() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.digit('1');
        assert!(calc.exponent_entry());
        calc.digit('2');
        calc.digit('8');
        assert!(calc.can_accept_digit('9'));
        calc.digit('9');
        assert_eq!(calc.display, "1.e+289");

        calc.clear_all();
        calc.digit('1');
        calc.exponent_entry();
        calc.digit('2');
        calc.digit('9');
        assert!(!calc.can_accept_digit('0'));
        calc.digit('0');
        assert_eq!(calc.display, "1.e+029");

        calc.clear_all();
        calc.digit('1');
        calc.exponent_entry();
        calc.sign();
        calc.digit('2');
        calc.digit('8');
        calc.digit('9');
        assert_eq!(calc.display, "1.e-289");
        assert!(!calc.can_accept_digit('0'));
        assert!(calc.can_toggle_sign());
        calc.sign();
        assert_eq!(calc.display, "1.e+289");
    }

    #[test]
    fn memory_commands_end_direct_entry_without_breaking_scientific_operands() {
        let mut calc = Calculator::default();
        calc.digit('1');
        calc.digit('2');
        calc.memory_store();
        assert_eq!(calc.memory, 12.0);
        assert!(calc.memory_set);
        calc.digit('3');
        assert_eq!(calc.value().unwrap(), 3.0);

        calc.memory_recall();
        assert_eq!(calc.value().unwrap(), 12.0);
        calc.digit('4');
        assert_eq!(calc.value().unwrap(), 4.0);

        calc.memory_clear();
        assert!(!calc.memory_set);
        calc.digit('5');
        assert_eq!(calc.value().unwrap(), 5.0);

        calc.memory_store();
        calc.digit('2');
        calc.memory_add();
        assert_eq!(calc.memory, 7.0);
        calc.digit('9');
        assert_eq!(calc.value().unwrap(), 9.0);

        // Storing zero must not illuminate the M indicator.
        calc.clear_all();
        calc.memory_store();
        assert_eq!(calc.memory, 0.0);
        assert!(!calc.memory_set);

        // MR is still a real operand in Scientific mode even though a digit
        // immediately after it replaces the recalled display.
        calc.set_mode(Mode::Scientific);
        calc.memory = 4.0;
        calc.memory_set = true;
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.memory_recall();
        calc.equals();
        assert_eq!(calc.value().unwrap(), 6.0);
    }

    #[test]
    fn inv_and_hyp_are_one_shot_only_for_functions_that_consume_them() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.angle = AngleMode::Radians;

        calc.set_value(0.5);
        calc.inv = true;
        calc.hyp = true;
        calc.unary("sin");
        assert!(!calc.inv);
        assert!(!calc.hyp);

        // ln consumes Inv but not an unrelated Hyp selector.
        calc.set_value(1.0);
        calc.inv = true;
        calc.hyp = true;
        calc.unary("ln");
        assert!(!calc.inv);
        assert!(calc.hyp);

        // An error exits before the original dispatcher clears its one-shot
        // selectors.
        calc.set_value(2.0);
        calc.inv = true;
        calc.hyp = false;
        calc.unary("sin");
        assert!(calc.error.is_some());
        assert!(calc.inv);
    }

    #[test]
    fn int_and_inverse_square_cube_match_win95_without_legacy_limits() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        calc.set_value(-1.5);
        calc.unary("int");
        assert_eq!(calc.value().unwrap(), -1.0);

        calc.set_value(-1.5);
        calc.inv = true;
        calc.unary("int");
        assert!((calc.value().unwrap() + 0.5).abs() < 1.0e-12);
        assert!(!calc.inv);

        calc.set_value(9.0);
        calc.inv = true;
        calc.unary("square");
        assert_eq!(calc.value().unwrap(), 3.0);
        assert!(!calc.inv);

        calc.set_value(-8.0);
        calc.inv = true;
        calc.unary("cube");
        assert!((calc.value().unwrap() + 2.0).abs() < 1.0e-12);
        assert!(!calc.inv);
    }

    #[test]
    fn decimal_result_formatting_uses_thirteen_significant_digits() {
        let locale = NumericLocale::new('.', Some(','));
        assert_eq!(format_value(1.23456789012349, Base::Dec, WordSize::Dword, false, locale), "1.234567890123");
        assert_eq!(format_value(9_999_999_999_999.0, Base::Dec, WordSize::Dword, false, locale), "9999999999999.");
        assert_eq!(format_value(10_000_000_000_000.0, Base::Dec, WordSize::Dword, false, locale), "1.e+013");
        assert_eq!(format_value(1.0e-15, Base::Dec, WordSize::Dword, false, locale), "0.000000000000001");
        assert_eq!(format_value(1.0e-16, Base::Dec, WordSize::Dword, false, locale), "1.e-016");
        assert_eq!(format_value(12.5, Base::Dec, WordSize::Dword, true, locale), "1.25e+001");
    }


    #[test]
    fn standard_percent_uses_the_saved_left_operand_for_multiply_and_divide() {
        let mut calc = Calculator::default();
        calc.digit('2');
        calc.digit('0');
        calc.digit('0');
        calc.binary(BinaryOp::Mul);
        calc.digit('1');
        calc.digit('0');
        calc.percent();
        assert_eq!(calc.display, "20.");
        calc.equals();
        assert_eq!(calc.display, "4000.");

        calc.clear_all();
        calc.digit('2');
        calc.digit('0');
        calc.digit('0');
        calc.binary(BinaryOp::Div);
        calc.digit('1');
        calc.digit('0');
        calc.percent();
        assert_eq!(calc.display, "20.");
        calc.equals();
        assert_eq!(calc.display, "10.");
    }

    #[test]
    fn closing_a_scientific_parenthesis_materializes_the_group_result_for_unary_ops() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.open_paren();
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.digit('3');
        calc.close_paren();
        assert_eq!(calc.display, "5.");
        calc.unary("square");
        assert_eq!(calc.display, "25.");
        calc.equals();
        assert_eq!(calc.display, "25.");
    }


    #[test]
    fn parenthesis_frames_preserve_outer_and_nested_scientific_context() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        // 2 * (3 + 4): closing only the group must show 7 while retaining 2*.
        calc.digit('2');
        calc.binary(BinaryOp::Mul);
        calc.open_paren();
        calc.digit('3');
        calc.binary(BinaryOp::Add);
        calc.digit('4');
        calc.close_paren();
        assert_eq!(calc.display, "7.");
        assert_eq!(calc.paren_depth(), 0);
        assert_eq!(calc.pending_scientific_history_expression().as_deref(), Some("2*(3+4)"));
        calc.equals();
        assert_eq!(calc.display, "14.");

        // Nested frames resolve innermost first without prematurely evaluating
        // the suspended outer expressions.
        calc.clear_all();
        calc.digit('2');
        calc.binary(BinaryOp::Mul);
        calc.open_paren();
        calc.digit('3');
        calc.binary(BinaryOp::Add);
        calc.open_paren();
        calc.digit('4');
        calc.binary(BinaryOp::Mul);
        calc.digit('5');
        calc.close_paren();
        assert_eq!(calc.display, "20.");
        assert_eq!(calc.paren_depth(), 1);
        calc.close_paren();
        assert_eq!(calc.display, "23.");
        assert_eq!(calc.paren_depth(), 0);
        calc.equals();
        assert_eq!(calc.display, "46.");
    }

    #[test]
    fn scientific_repeated_equals_reuses_the_last_operator_and_rhs() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.digit('3');
        calc.equals();
        assert_eq!(calc.display, "5.");
        calc.equals();
        assert_eq!(calc.display, "8.");
        calc.equals();
        assert_eq!(calc.display, "11.");
    }


    #[test]
    fn dms_uses_only_the_recovered_second_split_carry_rule() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);

        // Type the high-precision operand directly so the intentionally modern
        // unrestricted entry buffer reaches DMS before 13-digit result display
        // formatting is applied. Minute 59.999... rounds to minute 60, but the
        // original routine does not add a separate 60-minutes -> degree carry.
        for ch in "10.9999999999999".chars() {
            if ch == '.' { calc.decimal_point(); } else { calc.digit(ch); }
        }
        calc.unary("dms");
        assert_eq!(calc.display, "10.6");

        // 30.999... seconds are not independently rounded by an extra rule:
        // the recovered threshold sees the fractional *minute*, not a third
        // split of that remainder into seconds.
        let (minutes, fraction) = split_dms_secondary(12.0 + 30.999_999_999_999 / 60.0);
        assert_eq!(minutes, 12.0);
        assert!(fraction < 0.999_999_999_99);

        // Inv+DMS applies the same second-split threshold to the packed MMSS
        // component. Its arithmetic naturally yields the next degree when the
        // minute field becomes 60; there is still no extra explicit carry rule.
        calc.clear_all();
        for ch in "10.59999999999999".chars() {
            if ch == '.' { calc.decimal_point(); } else { calc.digit(ch); }
        }
        calc.inv = true;
        calc.unary("dms");
        assert_eq!(calc.display, "11.");
    }

    #[test]
    fn sign_before_a_new_operand_preserves_invisible_negative_zero_state() {
        let mut calc = Calculator::default();
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.sign();
        assert_eq!(calc.display, "2.", "the pending negative-zero sign is intentionally invisible");
        calc.digit('3');
        assert_eq!(calc.display, "-3.");
        calc.equals();
        assert_eq!(calc.display, "-1.");

        calc.clear_all();
        calc.set_mode(Mode::Scientific);
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.sign();
        calc.digit('3');
        calc.equals();
        assert_eq!(calc.display, "-1.");

        // After a completed result, +/- still negates the visible result rather
        // than preparing a hidden sign for a nonexistent pending operand.
        calc.sign();
        assert_eq!(calc.display, "1.");
    }

    #[test]
    fn sign_before_non_decimal_digits_uses_the_same_hidden_entry_sign() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.set_base(Base::Hex);
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.sign();
        calc.digit('3');
        assert_eq!(calc.value().unwrap(), -3.0);
        calc.equals();
        assert_eq!(calc.value().unwrap(), -1.0);
    }

    #[test]
    fn scientific_equals_supplies_a_missing_rhs_and_then_repeats_it() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.digit('2');
        calc.binary(BinaryOp::Add);
        calc.equals();
        assert_eq!(calc.display, "4.");
        calc.equals();
        assert_eq!(calc.display, "6.");

        calc.clear_all();
        calc.digit('5');
        calc.binary(BinaryOp::Mul);
        calc.equals();
        assert_eq!(calc.display, "25.");
        calc.equals();
        assert_eq!(calc.display, "125.");
    }

    #[test]
    fn scientific_operands_keep_the_radix_they_were_entered_under() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.digit('1');
        calc.digit('0');
        calc.binary(BinaryOp::Add); // Decimal ten is now canonicalized as decimal 10.
        calc.set_base(Base::Hex);
        calc.digit('5');
        calc.equals();
        assert_eq!(calc.display, "F");
        assert_eq!(calc.value().unwrap(), 15.0);

        calc.clear_all();
        calc.set_base(Base::Hex);
        calc.digit('1');
        calc.digit('0');
        calc.binary(BinaryOp::Add); // Hex 10 is stored explicitly as 0x10.
        calc.set_base(Base::Dec);
        calc.digit('5');
        calc.equals();
        assert_eq!(calc.display, "21.");
        assert_eq!(calc.value().unwrap(), 21.0);
    }

    #[test]
    fn backspace_is_only_accepted_during_an_editable_entry() {
        let mut calc = Calculator::default();
        assert!(!calc.can_backspace());
        calc.digit('1');
        assert!(calc.can_backspace());
        calc.backspace();
        assert!(!calc.can_backspace());

        calc.digit('4');
        calc.memory_store();
        assert!(!calc.can_backspace(), "memory commands end direct numeric editing");
    }

    #[test]
    fn clear_and_clear_entry_reset_scientific_modifiers_with_classic_fe_distinction() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.inv = true;
        calc.hyp = true;
        calc.force_exp = true;
        calc.set_value(12.5);

        calc.clear_entry();
        assert!(!calc.inv);
        assert!(!calc.hyp);
        assert!(calc.force_exp, "CE preserves F-E");

        calc.inv = true;
        calc.hyp = true;
        calc.clear_all();
        assert!(!calc.inv);
        assert!(!calc.hyp);
        assert!(!calc.force_exp, "C clears F-E");
    }

    #[test]
    fn direct_trig_commands_snap_win95_canonical_zeroes() {
        let mut calc = Calculator::default();
        calc.set_mode(Mode::Scientific);
        calc.angle = AngleMode::Degrees;

        calc.set_value(180.0);
        calc.unary("sin");
        assert_eq!(calc.value().unwrap(), 0.0);

        calc.set_value(90.0);
        calc.unary("cos");
        assert_eq!(calc.value().unwrap(), 0.0);

        calc.set_value(180.0);
        calc.unary("tan");
        assert_eq!(calc.value().unwrap(), 0.0);
    }

}
