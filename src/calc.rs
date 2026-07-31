use crate::errors::{DIVIDE_BY_ZERO, FUNCTION_UNDEFINED, INVALID_FUNCTION_INPUT, RESULT_TOO_LARGE, RESULT_TOO_SMALL};
use crate::expr::{eval_expression, AngleMode, EvalContext};
use crate::locale::NumericLocale;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode { Standard, Scientific }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base { Hex=16, Dec=10, Oct=8, Bin=2 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp { Add, Sub, Mul, Div, Mod, Pow, Root, And, Or, Xor, Lsh }

#[derive(Clone, Debug, PartialEq)]
pub struct Calculator {
    pub mode: Mode,
    pub angle: AngleMode,
    pub base: Base,
    pub inv: bool,
    pub hyp: bool,
    pub memory: f64,
    pub memory_set: bool,
    pub display: String,
    pub error: Option<String>,
    accumulator: f64,
    pending: Option<BinaryOp>,
    entering: bool,
    // True only when the user explicitly entered the decimal separator for the
    // current entry. The classic display always shows a trailing separator for
    // decimal integers, so this state cannot be reconstructed from display text.
    decimal_entered: bool,
    scientific_expr: String,
    paren_depth: usize,
    pub stats: Vec<f64>,
    pub force_exp: bool,
    number_locale: NumericLocale,
}

impl Default for Calculator {
    fn default() -> Self {
        let number_locale = NumericLocale::system();
        Self {
            mode: Mode::Standard, angle: AngleMode::Degrees, base: Base::Dec,
            inv: false, hyp: false, memory: 0.0, memory_set: false,
            display: zero_display(Base::Dec, number_locale), error: None, accumulator: 0.0,
            pending: None, entering: false, decimal_entered: false, scientific_expr: String::new(), paren_depth: 0,
            stats: Vec::new(), force_exp:false, number_locale,
        }
    }
}

impl Calculator {
    pub fn clear_all(&mut self) {
        self.display = zero_display(self.base, self.number_locale); self.error = None; self.accumulator = 0.0;
        self.pending = None; self.entering = false; self.decimal_entered = false; self.scientific_expr.clear(); self.paren_depth = 0;
    }

    pub fn clear_entry(&mut self) { self.display = zero_display(self.base, self.number_locale); self.error = None; self.entering = false; self.decimal_entered = false; }

    pub fn backspace(&mut self) {
        if !self.entering || self.error.is_some() { return; }
        let mut s = self.raw_entry();
        s.pop();
        self.decimal_entered = self.base == Base::Dec && s.contains('.');
        if s.is_empty() || s == "-" {
            self.display = zero_display(self.base, self.number_locale);
            self.entering = false;
            self.decimal_entered = false;
        } else {
            self.display = entry_display(&s, self.base, self.number_locale);
        }
    }

    pub fn digit(&mut self, ch: char) {
        if self.error.is_some() { self.clear_all(); }
        let valid = match self.base {
            Base::Dec => ch.is_ascii_digit(),
            Base::Hex => ch.is_ascii_hexdigit(),
            Base::Oct => matches!(ch, '0'..='7'),
            Base::Bin => matches!(ch, '0'|'1'),
        };
        if !valid { return; }
        let mut s = if self.entering {
            self.raw_entry()
        } else {
            self.decimal_entered = false;
            String::new()
        };
        if s == "0" && !self.decimal_entered { s.clear(); }
        if s.len() < 30 { s.push(ch.to_ascii_uppercase()); }
        self.display = entry_display(&s, self.base, self.number_locale);
        self.entering = true;
    }

    pub fn decimal_point(&mut self) {
        if self.base != Base::Dec { return; }
        if self.error.is_some() { self.clear_all(); }
        let mut s = if self.entering { self.raw_entry() } else { "0".into() };
        if !self.decimal_entered {
            if !s.contains('.') { s.push('.'); }
            self.decimal_entered = true;
        }
        self.display = entry_display(&s, self.base, self.number_locale);
        self.entering = true;
    }

    pub fn sign(&mut self) {
        if self.error.is_some() { return; }
        let mut s = self.raw_entry();
        if s == "0" { return; }
        if s.starts_with('-') { s.remove(0); } else { s.insert(0, '-'); }
        self.display = entry_display(&s, self.base, self.number_locale);
        self.entering = true;
    }

    pub fn binary(&mut self, op: BinaryOp) {
        if self.error.is_some() { return; }
        // CALC.EXE implements Inv+x^y as x^(1/y).  It consumes Inv when the
        // operator is selected and explicitly rejects a zero root/exponent as
        // invalid function input (0x00404FCE..0x00405016).
        let op = if op == BinaryOp::Pow && self.inv {
            self.inv = false;
            BinaryOp::Root
        } else {
            op
        };
        match self.mode {
            Mode::Standard => {
                let cur = self.value().unwrap_or(0.0);
                if self.entering {
                    if let Some(prev) = self.pending.take() {
                        match apply_binary(prev, self.accumulator, cur) {
                            Ok(v) => { self.accumulator = v; self.set_value(v); }
                            Err(e) => { self.error = Some(e.clone()); self.display = e; return; }
                        }
                    } else { self.accumulator = cur; }
                }
                self.pending = Some(op); self.entering = false;
            }
            Mode::Scientific => {
                self.push_current_into_expr();
                self.scientific_expr.push_str(op_text(op));
                self.entering = false;
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
        match self.mode {
            Mode::Standard => {
                if let Some(op) = self.pending.take() {
                    let cur = self.value().unwrap_or(0.0);
                    match apply_binary(op, self.accumulator, cur) {
                        Ok(v) => { self.accumulator = v; self.set_value(v); }
                        Err(e) => { self.error = Some(e.clone()); self.display = e; }
                    }
                }
                self.entering = false;
            }
            Mode::Scientific => {
                self.push_current_into_expr();
                while self.paren_depth > 0 { self.scientific_expr.push(')'); self.paren_depth -= 1; }
                let s = std::mem::take(&mut self.scientific_expr);
                if s.trim().is_empty() { return; }
                self.evaluate_paste(&s);
                self.entering = false;
            }
        }
    }

    pub fn open_paren(&mut self) {
        if self.mode != Mode::Scientific { return; }
        if self.entering { self.push_current_into_expr(); self.scientific_expr.push('*'); }
        self.scientific_expr.push('('); self.paren_depth += 1; self.entering = false;
    }

    pub fn close_paren(&mut self) {
        if self.mode != Mode::Scientific || self.paren_depth == 0 { return; }
        self.push_current_into_expr(); self.scientific_expr.push(')'); self.paren_depth -= 1; self.entering = false;
    }

    pub fn unary(&mut self, name: &str) {
        if self.error.is_some() { return; }
        let x = self.value().unwrap_or(0.0);
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
            "int" => Ok(x.floor()),
            "not" => {
                // Unary Not has its own domain guard in CALC.EXE: unlike the
                // binary logic operators, an operand outside unsigned DWORD
                // magnitude is classified as invalid function input.
                if x.abs() > u32::MAX as f64 {
                    Err(INVALID_FUNCTION_INPUT.into())
                } else {
                    Ok((!(x as i64)) as f64)
                }
            },
            "dms" => {
                let sign = if x < 0.0 { -1.0 } else { 1.0 };
                let a = x.abs();
                let d = a.floor();
                let mf = (a - d) * 60.0;
                let m = mf.floor();
                let sec = (mf - m) * 60.0;
                Ok(sign * (d + m / 100.0 + sec / 10000.0))
            }
            "dms_inv" => {
                let sign = if x < 0.0 { -1.0 } else { 1.0 };
                let a = x.abs();
                let d = a.floor();
                let mmss = (a - d) * 100.0;
                let m = mmss.floor();
                let sec = (mmss - m) * 100.0;
                Ok(sign * (d + m / 60.0 + sec / 3600.0))
            }
            "fe" => Ok(x),
            "pow10" => checked_exp_like(10f64.powf(x)),
            n => eval_expression(&format!("{n}({x})"), self.eval_context()),
        };
        match result {
            Ok(v) => { self.set_value(v); self.entering = true; }
            Err(e) => self.fail(&e),
        }
    }

    pub fn percent(&mut self) {
        if self.error.is_some() { return; }
        let x = self.value().unwrap_or(0.0);
        let v = match self.pending {
            Some(BinaryOp::Add) | Some(BinaryOp::Sub) => self.accumulator * x / 100.0,
            _ => x / 100.0,
        };
        match checked_large(v) {
            Ok(value) => { self.set_value(value); self.entering = true; }
            Err(error) => self.fail(&error),
        }
    }

    pub fn paste_expression(&mut self, text: &str) { self.evaluate_paste(text); }

    fn evaluate_paste(&mut self, text: &str) {
        // Transactional: unlike CALC.EXE, invalid input cannot partly mutate calculator state.
        match eval_expression(text, self.eval_context()) {
            Ok(v) => { self.error = None; self.set_value(v); self.accumulator = v; self.pending=None; self.entering=false; }
            Err(e) => self.fail(&e),
        }
    }

    pub fn memory_clear(&mut self) { self.memory = 0.0; self.memory_set = false; }
    pub fn memory_recall(&mut self) { self.set_value(self.memory); self.entering = true; }
    pub fn memory_store(&mut self) { self.memory = self.value().unwrap_or(0.0); self.memory_set = true; }
    pub fn memory_add(&mut self) {
        if self.error.is_some() { return; }
        let candidate = self.memory + self.value().unwrap_or(0.0);
        match checked_add_sub(candidate) {
            Ok(value) => {
                self.memory = value;
                self.memory_set = self.memory != 0.0;
            }
            Err(error) => self.fail(&error),
        }
    }

    pub fn set_mode(&mut self, mode: Mode) { self.mode = mode; self.clear_all(); }
    pub(crate) fn paren_depth(&self) -> usize { self.paren_depth }
    pub fn set_base(&mut self, base: Base) {
        if self.error.is_some() { return; }
        let v = self.value().unwrap_or(0.0);
        if base != Base::Dec && v.abs() > u32::MAX as f64 {
            self.fail(if v.is_sign_negative() { RESULT_TOO_SMALL } else { RESULT_TOO_LARGE });
            return;
        }
        self.base = base;
        self.set_value(v);
        self.entering = false;
    }

    pub fn stat_dat(&mut self) {
        if let Ok(value) = self.value() {
            self.stats.push(value);
        }
        self.finish_statistics_action();
    }

    pub fn stat_sum(&mut self) {
        let sum = self.stats.iter().copied().sum::<f64>();
        match checked_large(sum) {
            Ok(value) => self.set_value(value),
            Err(error) => self.fail(&error),
        }
        self.finish_statistics_action();
    }

    pub fn stat_avg(&mut self) {
        if self.stats.is_empty() {
            self.fail(DIVIDE_BY_ZERO);
        } else {
            let sum = self.stats.iter().copied().sum::<f64>();
            match checked_large(sum / self.stats.len() as f64) {
                Ok(value) => self.set_value(value),
                Err(error) => self.fail(&error),
            }
        }
        self.finish_statistics_action();
    }

    pub fn stat_stddev(&mut self) {
        if self.stats.len() <= 1 {
            self.set_value(0.0);
        } else {
            let mean = self.stats.iter().sum::<f64>() / self.stats.len() as f64;
            let variance = self.stats.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>()
                / (self.stats.len() - 1) as f64;
            match checked_large(variance.sqrt()) {
                Ok(value) => self.set_value(value),
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

    pub fn value(&self) -> Result<f64, String> {
        if self.error.is_some() { return Err("Calculator is in error state.".into()); }
        let s = self.raw_entry();
        if self.base == Base::Dec { s.parse::<f64>().map_err(|_| "Invalid number".into()) }
        else {
            let neg=s.starts_with('-'); let d=s.trim_start_matches('-').trim_end_matches('.');
            i64::from_str_radix(d, self.base as u32).map(|v| if neg {-(v as f64)} else {v as f64}).map_err(|_| "Invalid integer".into())
        }
    }

    pub fn set_value(&mut self, v: f64) {
        self.display = format_value(v, self.base, self.force_exp, self.number_locale); self.error=None; self.decimal_entered=false;
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
        self.entering = false;
        self.decimal_entered = false;
        self.scientific_expr.clear();
        self.paren_depth = 0;
        self.set_value(v);
    }

    pub fn toggle_fe(&mut self) { let v=self.value().unwrap_or(0.0); self.force_exp=!self.force_exp; self.set_value(v); }

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
        let op = self.pending?;
        let rhs = self.value().ok()?;
        Some((
            op,
            format_value(self.accumulator, self.base, self.force_exp, self.number_locale),
            format_value(rhs, self.base, self.force_exp, self.number_locale),
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
            expression.push_str(&self.raw_entry());
        }
        for _ in 0..self.paren_depth {
            expression.push(')');
        }
        if expression.trim().is_empty() {
            None
        } else {
            Some(expression)
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
        format_value(value, Base::Dec, false, self.number_locale)
    }

    fn push_current_into_expr(&mut self) {
        if self.entering || self.scientific_expr.is_empty() {
            let s=self.raw_entry(); self.scientific_expr.push_str(&s);
        }
    }

    fn fail(&mut self, s:&str) { self.error=Some(s.to_string()); self.display=s.to_string(); self.pending=None; self.decimal_entered=false; self.scientific_expr.clear(); }
}

fn op_text(op: BinaryOp) -> &'static str { match op { BinaryOp::Add=>"+", BinaryOp::Sub=>"-", BinaryOp::Mul=>"*", BinaryOp::Div=>"/", BinaryOp::Mod=>" mod ", BinaryOp::Pow=>"^", BinaryOp::Root=>" root ", BinaryOp::And=>" and ", BinaryOp::Or=>" or ", BinaryOp::Xor=>" xor ", BinaryOp::Lsh=>" lsh " } }

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
        BinaryOp::And | BinaryOp::Or | BinaryOp::Xor | BinaryOp::Lsh => {
            // IDs 0x56..0x59 share the recovered unsigned-DWORD magnitude
            // guard before entering the individual bitwise operation.
            if a.abs() > u32::MAX as f64 || b.abs() > u32::MAX as f64 {
                return Err(RESULT_TOO_LARGE.into());
            }
            let value = match op {
                BinaryOp::And => ((a as i64) & (b as i64)) as f64,
                BinaryOp::Or => ((a as i64) | (b as i64)) as f64,
                BinaryOp::Xor => ((a as i64) ^ (b as i64)) as f64,
                BinaryOp::Lsh => ((a as i64).wrapping_shl((b as u32) & 63)) as f64,
                _ => unreachable!(),
            };
            Ok(value)
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

fn format_value(v: f64, base: Base, force_exp: bool, locale: NumericLocale) -> String {
    if base != Base::Dec {
        let n = v as i64;
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
    let a = v.abs();
    let mut s = if force_exp || a >= 1e14 || a < 1e-10 {
        format!("{:.10e}", v)
    } else {
        format!("{:.12}", v)
    };
    if let Some(e) = s.find('e') {
        let (m, ex) = s.split_at(e);
        let m = m.trim_end_matches('0').trim_end_matches('.');
        s = format!("{m}{ex}");
    } else {
        while s.contains('.') && s.ends_with('0') { s.pop(); }
        if !s.ends_with('.') && !s.contains('.') { s.push('.'); }
    }
    locale.localize(&s)
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
        assert_eq!(format_value(12.5, Base::Dec, false, locale), "12.5");
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

}

