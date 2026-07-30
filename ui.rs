//! A real expression parser for clipboard input.
//!
//! The original Windows 95 CALC.EXE paste handler (around 0x403D51 in the
//! supplied reference binary) translates one character at a time into normal
//! calculator commands.  That is why expressions such as `2*-3` are
//! misinterpreted.  This module deliberately does *not* emulate that bug.

use crate::errors::{DIVIDE_BY_ZERO, FUNCTION_UNDEFINED, INVALID_FUNCTION_INPUT, RESULT_TOO_LARGE, RESULT_TOO_SMALL};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleMode {
    Degrees,
    Radians,
    Grads,
}

#[derive(Clone, Copy, Debug)]
pub struct EvalContext {
    pub angle: AngleMode,
    pub decimal_separator: char,
    pub thousands_separator: Option<char>,
}

impl Default for EvalContext {
    fn default() -> Self {
        Self {
            angle: AngleMode::Degrees,
            decimal_separator: '.',
            thousands_separator: Some(','),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    Bang,
    LParen,
    RParen,
    Eq,
    End,
}

#[derive(Clone, Debug)]
pub struct CompiledExpression {
    toks: Vec<Tok>,
    ctx: EvalContext,
}

impl CompiledExpression {
    pub fn parse(input: &str, ctx: EvalContext) -> Result<Self, String> {
        let toks = Lexer::new(input, ctx.decimal_separator, ctx.thousands_separator).lex()?;
        // Graph evaluations reuse the token stream for every sample instead of
        // repeatedly lexing the same expression. Domain validity is intentionally
        // checked per sample because a valid graph such as sqrt(x-1) need not be
        // defined at x=0.
        Ok(Self { toks, ctx })
    }

    pub fn evaluate_at(&self, x: f64) -> Result<f64, String> {
        self.evaluate_internal(Some(x))
    }

    fn evaluate_internal(&self, x_value: Option<f64>) -> Result<f64, String> {
        let mut p = Parser { toks: &self.toks, pos: 0, ctx: self.ctx, x_value };
        let value = p.parse_expr(0)?;
        while p.eat(&Tok::Eq) {}
        if !matches!(p.peek(), Tok::End) {
            return Err(format!("Unexpected token after expression: {:?}", p.peek()));
        }
        if !value.is_finite() {
            return Err(RESULT_TOO_LARGE.to_string());
        }
        Ok(value)
    }
}

pub fn eval_expression(input: &str, ctx: EvalContext) -> Result<f64, String> {
    let toks = Lexer::new(input, ctx.decimal_separator, ctx.thousands_separator).lex()?;
    let mut p = Parser { toks: &toks, pos: 0, ctx, x_value: None };
    let value = p.parse_expr(0)?;
    while p.eat(&Tok::Eq) {}
    if !matches!(p.peek(), Tok::End) {
        return Err(format!("Unexpected token after expression: {:?}", p.peek()));
    }
    if !value.is_finite() {
        return Err(RESULT_TOO_LARGE.to_string());
    }
    Ok(value)
}

struct Lexer<'a> {
    src: &'a str,
    i: usize,
    decimal_separator: char,
    thousands_separator: Option<char>,
}

impl<'a> Lexer<'a> {
    fn new(s: &'a str, decimal_separator: char, thousands_separator: Option<char>) -> Self {
        Self { src: s, i: 0, decimal_separator, thousands_separator }
    }

    fn peek(&self) -> Option<char> {
        self.src[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.i += ch.len_utf8();
        Some(ch)
    }

    fn starts_with_ascii(&self, prefix: &str) -> bool {
        self.src[self.i..].starts_with(prefix)
    }

    fn lex(mut self) -> Result<Vec<Tok>, String> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
                continue;
            }
            let t = match c {
                '+' => { self.bump(); Tok::Plus }
                '-' | '\u{2212}' => { self.bump(); Tok::Minus }
                '*' => {
                    self.bump();
                    // Accept programming-language exponentiation syntax as a
                    // synonym for the calculator's `^` operator. A single `*`
                    // remains ordinary multiplication.
                    if matches!(self.peek(), Some('*')) {
                        self.bump();
                        Tok::Caret
                    } else {
                        Tok::Star
                    }
                }
                '\u{00d7}' => { self.bump(); Tok::Star }
                '/' | '\u{00f7}' => { self.bump(); Tok::Slash }
                '^' => { self.bump(); Tok::Caret }
                '%' => { self.bump(); Tok::Percent }
                '!' => { self.bump(); Tok::Bang }
                '(' => { self.bump(); Tok::LParen }
                ')' => { self.bump(); Tok::RParen }
                '=' => { self.bump(); Tok::Eq }
                '0'..='9' | '.' | ',' => self.number()?,
                _ if c == self.decimal_separator => self.number()?,
                _ if c.is_ascii_alphabetic() || c == '_' || c == '\u{03c0}' || c == '\u{03a0}' => self.ident()?,
                _ => return Err(format!("Invalid character '{}' in expression.", c)),
            };
            out.push(t);
        }
        out.push(Tok::End);
        Ok(out)
    }

    fn number(&mut self) -> Result<Tok, String> {
        // Based integers remain invariant and do not use locale punctuation.
        if self.starts_with_ascii("0x") || self.starts_with_ascii("0X")
            || self.starts_with_ascii("0b") || self.starts_with_ascii("0B")
            || self.starts_with_ascii("0o") || self.starts_with_ascii("0O")
        {
            let prefix = self.src.as_bytes()[self.i + 1];
            self.i += 2;
            let digits_start = self.i;
            while let Some(ch) = self.peek() {
                if ch.is_ascii_hexdigit() { self.bump(); } else { break; }
            }
            if digits_start == self.i {
                return Err("Missing digits after numeric base prefix.".into());
            }
            let digits = &self.src[digits_start..self.i];
            let radix = match prefix { b'x' | b'X' => 16, b'b' | b'B' => 2, _ => 8 };
            let n = u64::from_str_radix(digits, radix).map_err(|_| "Invalid based integer")?;
            return Ok(Tok::Num(n as f64));
        }

        let start = self.i;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || self.is_number_separator(ch) {
                self.bump();
            } else {
                break;
            }
        }
        let mantissa_end = self.i;

        if matches!(self.peek(), Some('e' | 'E')) {
            let save = self.i;
            self.bump();
            if matches!(self.peek(), Some('+' | '-')) { self.bump(); }
            let digits = self.i;
            while matches!(self.peek(), Some(ch) if ch.is_ascii_digit()) { self.bump(); }
            if digits == self.i { self.i = save; }
        }

        let raw_mantissa = &self.src[start..mantissa_end];
        let mut normalized = normalize_mantissa(
            raw_mantissa,
            self.decimal_separator,
            self.thousands_separator,
        )?;
        if self.i > mantissa_end {
            normalized.push_str(&self.src[mantissa_end..self.i]);
        }
        let v: f64 = normalized.parse().map_err(|_| format!("Invalid number: {}", &self.src[start..self.i]))?;
        if v.is_infinite() {
            return Err(RESULT_TOO_LARGE.into());
        }
        if v == 0.0 && normalized.bytes().any(|byte| matches!(byte, b'1'..=b'9')) {
            // Old Calculator's decimal conversion path can report internal
            // underflow (0x85), which the central dispatcher maps to ID 71.
            return Err(RESULT_TOO_SMALL.into());
        }
        Ok(Tok::Num(v))
    }

    fn is_number_separator(&self, ch: char) -> bool {
        ch == '.'
            || ch == ','
            || ch == self.decimal_separator
            || Some(ch) == self.thousands_separator
            || ch == '\u{00a0}'
            || ch == '\u{202f}'
    }

    fn ident(&mut self) -> Result<Tok, String> {
        let first = self.peek().ok_or("Missing identifier")?;
        if first == '\u{03c0}' || first == '\u{03a0}' {
            self.bump();
            return Ok(Tok::Ident("pi".into()));
        }
        let start = self.i;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' { self.bump(); } else { break; }
        }
        Ok(Tok::Ident(self.src[start..self.i].to_ascii_lowercase()))
    }
}

fn normalize_mantissa(
    raw: &str,
    locale_decimal: char,
    locale_thousands: Option<char>,
) -> Result<String, String> {
    let chars: Vec<char> = raw.chars().collect();
    if chars.is_empty() {
        return Err("Invalid number.".into());
    }

    let dot_positions: Vec<usize> = chars.iter().enumerate().filter_map(|(i, ch)| (*ch == '.').then_some(i)).collect();
    let comma_positions: Vec<usize> = chars.iter().enumerate().filter_map(|(i, ch)| (*ch == ',').then_some(i)).collect();
    let locale_positions: Vec<usize> = if locale_decimal != '.' && locale_decimal != ',' {
        chars.iter().enumerate().filter_map(|(i, ch)| (*ch == locale_decimal).then_some(i)).collect()
    } else {
        Vec::new()
    };

    let decimal_index = if let Some(&pos) = locale_positions.last() {
        if locale_positions.len() > 1 {
            return Err(format!("Invalid number: {raw}"));
        }
        Some(pos)
    } else if !dot_positions.is_empty() && !comma_positions.is_empty() {
        Some(*dot_positions.last().unwrap().max(comma_positions.last().unwrap()))
    } else {
        let positions = if !dot_positions.is_empty() { &dot_positions } else { &comma_positions };
        if positions.is_empty() {
            None
        } else {
            let separator = chars[positions[0]];
            if positions.len() == 1 {
                // A single alternate punctuation mark is accepted as a radix
                // (`1,5` on an en-US machine or `1.5` on a pt-BR machine).
                // The one ambiguous case follows the OS locale: when the mark
                // is the configured grouping symbol and the token has a
                // textbook 1-3 / 3 digit grouping shape, treat it as grouping.
                if Some(separator) == locale_thousands
                    && separator != locale_decimal
                    && looks_like_grouped_integer(&chars, separator)
                {
                    None
                } else {
                    positions.last().copied()
                }
            } else if looks_like_grouped_integer(&chars, separator) {
                None
            } else {
                return Err(format!("Invalid number: {raw}"));
            }
        }
    };

    let mut out = String::with_capacity(raw.len() + 1);
    let mut saw_digit = false;
    for (index, ch) in chars.iter().copied().enumerate() {
        if ch.is_ascii_digit() {
            out.push(ch);
            saw_digit = true;
        } else if Some(index) == decimal_index {
            out.push('.');
        } else if ch == '.'
            || ch == ','
            || ch == locale_decimal
            || Some(ch) == locale_thousands
            || ch == '\u{00a0}'
            || ch == '\u{202f}'
        {
            // Grouping characters are discarded. With both '.' and ',' in a
            // token the right-most punctuation is the radix, so both common
            // pasted forms (1,234.56 and 1.234,56) work on every locale.
        } else {
            return Err(format!("Invalid number: {raw}"));
        }
    }

    if !saw_digit {
        return Err(format!("Invalid number: {raw}"));
    }
    if out.starts_with('.') { out.insert(0, '0'); }
    Ok(out)
}

fn looks_like_grouped_integer(chars: &[char], separator: char) -> bool {
    let groups: Vec<String> = chars
        .iter()
        .collect::<String>()
        .split(separator)
        .map(str::to_owned)
        .collect();
    if groups.len() < 2 || groups[0].is_empty() || groups[0].len() > 3 {
        return false;
    }
    groups.iter().all(|group| group.chars().all(|ch| ch.is_ascii_digit()))
        && groups.iter().skip(1).all(|group| group.len() == 3)
}

struct Parser<'a> {
    toks: &'a [Tok],
    pos: usize,
    ctx: EvalContext,
    x_value: Option<f64>,
}

impl Parser<'_> {
    fn peek(&self) -> &Tok { &self.toks[self.pos] }
    fn next(&mut self) -> Tok { let t = self.toks[self.pos].clone(); self.pos += 1; t }
    fn eat(&mut self, wanted: &Tok) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(wanted) { self.pos += 1; true } else { false }
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<f64, String> {
        let mut lhs = match self.next() {
            Tok::Num(v) => v,
            Tok::Plus => self.parse_expr(11)?,
            Tok::Minus => -self.parse_expr(11)?,
            Tok::LParen => {
                let v = self.parse_expr(0)?;
                if !self.eat(&Tok::RParen) { return Err("Missing closing parenthesis.".into()); }
                v
            }
            Tok::Ident(name) => self.parse_ident(name)?,
            t => return Err(format!("Expected a number, unary sign, function, or '(', got {t:?}.")),
        };

        loop {
            match self.peek() {
                Tok::Bang => {
                    if 13 < min_bp { break; }
                    self.next();
                    lhs = factorial(lhs)?;
                }
                Tok::Percent => {
                    if 13 < min_bp { break; }
                    self.next();
                    lhs /= 100.0;
                }
                _ => {
                    let (l_bp, r_bp, op) = match self.peek() {
                        Tok::Plus => (1, 2, '+'),
                        Tok::Minus => (1, 2, '-'),
                        Tok::Star => (3, 4, '*'),
                        Tok::Slash => (3, 4, '/'),
                        Tok::Caret => (9, 8, '^'), // right associative
                        // Internal UI spelling for Inv+x^y.  This is accepted
                        // by the parser too so the scientific-mode expression
                        // buffer can preserve the classic x^(1/y) operation.
                        Tok::Ident(s) if s == "root" => (9, 8, 'r'),
                        Tok::Ident(s) if s == "mod" => (3, 4, 'm'),
                        Tok::Ident(s) if s == "and" => (0, 1, '&'),
                        Tok::Ident(s) if s == "xor" => (0, 1, 'x'),
                        Tok::Ident(s) if s == "or" => (0, 1, '|'),
                        Tok::Ident(s) if s == "lsh" => (5, 6, '<'),
                        _ => break,
                    };
                    if l_bp < min_bp { break; }
                    self.next();
                    let rhs = self.parse_expr(r_bp)?;
                    lhs = apply_binary(op, lhs, rhs)?;
                }
            }
        }
        Ok(lhs)
    }

    fn parse_ident(&mut self, name: String) -> Result<f64, String> {
        if name == "x" {
            return self.x_value.ok_or_else(|| "Variable x is available only in graph mode.".to_string());
        }
        if name == "pi" {
            // Treat pi as a constant, but also accept the familiar zero-arg
            // spelling pi() when expressions are pasted from other tools.
            if self.eat(&Tok::LParen) && !self.eat(&Tok::RParen) {
                return Err("pi() does not take an argument.".into());
            }
            return Ok(std::f64::consts::PI);
        }
        if name == "e" { return Ok(std::f64::consts::E); }

        // Function calls may be written either f(x) or, for compatibility with
        // classic calculator habits, f x for a simple following primary.
        let arg = if self.eat(&Tok::LParen) {
            let a = self.parse_expr(0)?;
            if !self.eat(&Tok::RParen) { return Err(format!("Missing ')' after {name}.")); }
            a
        } else {
            self.parse_expr(11)?
        };
        apply_function(&name, arg, self.ctx)
    }
}

fn apply_binary(op: char, a: f64, b: f64) -> Result<f64, String> {
    match op {
        '+' => checked_add_sub(a + b),
        '-' => checked_add_sub(a - b),
        '*' => {
            if a != 0.0 && b != 0.0 && a.abs().log10() + b.abs().log10() > 307.0 {
                return Err(RESULT_TOO_LARGE.into());
            }
            checked_finite(a * b)
        }
        '/' => {
            if b == 0.0 { return Err(DIVIDE_BY_ZERO.into()); }
            if a != 0.0 && a.abs().log10() - b.abs().log10() > 307.0 {
                return Err(RESULT_TOO_LARGE.into());
            }
            checked_finite(a / b)
        }
        '^' => checked_pow(a, b),
        'r' => {
            if b == 0.0 { Err(INVALID_FUNCTION_INPUT.into()) } else { checked_pow(a, 1.0 / b) }
        }
        'm' => {
            if b == 0.0 { return Err(DIVIDE_BY_ZERO.into()); }
            checked_finite(a % b)
        }
        '&' | '|' | 'x' | '<' => {
            if a.abs() > u32::MAX as f64 || b.abs() > u32::MAX as f64 {
                return Err(RESULT_TOO_LARGE.into());
            }
            Ok(match op {
                '&' => ((a as i64) & (b as i64)) as f64,
                '|' => ((a as i64) | (b as i64)) as f64,
                'x' => ((a as i64) ^ (b as i64)) as f64,
                '<' => ((a as i64).wrapping_shl((b as u32) & 63)) as f64,
                _ => unreachable!(),
            })
        }
        _ => Err("Unknown operator.".into()),
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

fn checked_finite(v: f64) -> Result<f64, String> {
    if v.is_nan() {
        Err(FUNCTION_UNDEFINED.into())
    } else if v.is_infinite() {
        Err(RESULT_TOO_LARGE.into())
    } else {
        Ok(v)
    }
}

fn checked_exp(v: f64) -> Result<f64, String> {
    if v.is_infinite() {
        Err(RESULT_TOO_LARGE.into())
    } else if v == 0.0 {
        // The original C math-error hook maps UNDERFLOW to string-table ID 71.
        Err(RESULT_TOO_SMALL.into())
    } else {
        Ok(v)
    }
}

fn checked_pow(base: f64, exponent: f64) -> Result<f64, String> {
    let v = base.powf(exponent);
    if v.is_nan() {
        // pow() DOMAIN maps through CALC.EXE's _matherr-style dispatcher to
        // "Invalid input for function." (resource ID 68).
        Err(INVALID_FUNCTION_INPUT.into())
    } else if v.is_infinite() {
        Err(RESULT_TOO_LARGE.into())
    } else if v == 0.0 && base != 0.0 {
        Err(RESULT_TOO_SMALL.into())
    } else {
        Ok(v)
    }
}

fn to_radians(x: f64, mode: AngleMode) -> f64 {
    match mode {
        AngleMode::Degrees => x.to_radians(),
        AngleMode::Radians => x,
        AngleMode::Grads => x * std::f64::consts::PI / 200.0,
    }
}

fn from_radians(x: f64, mode: AngleMode) -> f64 {
    match mode {
        AngleMode::Degrees => x.to_degrees(),
        AngleMode::Radians => x,
        AngleMode::Grads => x * 200.0 / std::f64::consts::PI,
    }
}

fn apply_function(name: &str, x: f64, ctx: EvalContext) -> Result<f64, String> {
    let v = match name {
        "sqrt" => {
            if x < 0.0 { return Err(FUNCTION_UNDEFINED.into()); }
            x.sqrt()
        }
        "sin" => to_radians(x, ctx.angle).sin(),
        "cos" => to_radians(x, ctx.angle).cos(),
        "tan" => {
            let value = to_radians(x, ctx.angle).tan();
            // CALC.EXE explicitly treats the huge tangent produced at an
            // asymptote as an undefined function result (threshold 1e15).
            if value.abs() > 1.0e15 { return Err(FUNCTION_UNDEFINED.into()); }
            value
        }
        "asin" => {
            if !(-1.0..=1.0).contains(&x) { return Err(INVALID_FUNCTION_INPUT.into()); }
            from_radians(x.asin(), ctx.angle)
        }
        "acos" => {
            if !(-1.0..=1.0).contains(&x) { return Err(INVALID_FUNCTION_INPUT.into()); }
            from_radians(x.acos(), ctx.angle)
        }
        "atan" => from_radians(x.atan(), ctx.angle),
        "sinh" => x.sinh(),
        "cosh" => x.cosh(),
        "tanh" => x.tanh(),
        "asinh" => x.asinh(),
        "acosh" => {
            if x < 1.0 { return Err(INVALID_FUNCTION_INPUT.into()); }
            x.acosh()
        }
        "atanh" => {
            if x.abs() >= 1.0 { return Err(INVALID_FUNCTION_INPUT.into()); }
            x.atanh()
        }
        "ln" => {
            if x <= 0.0 { return Err(FUNCTION_UNDEFINED.into()); }
            x.ln()
        }
        "log" | "log10" => {
            if x <= 0.0 { return Err(FUNCTION_UNDEFINED.into()); }
            x.log10()
        }
        "exp" => return checked_exp(x.exp()),
        "abs" => x.abs(),
        "int" | "floor" => x.floor(),
        "ceil" => x.ceil(),
        "fact" | "factorial" => factorial(x)?,
        _ => return Err(format!("Unknown function '{name}'.")),
    };
    checked_finite(v)
}

fn factorial(x: f64) -> Result<f64, String> {
    if x < 0.0 || x.fract() != 0.0 {
        return Err(INVALID_FUNCTION_INPUT.into());
    }
    // A well-formed operand that simply overflows is an overflow, not invalid
    // input -- CALC.EXE reports these with different strings.
    if x > 170.0 {
        return Err(RESULT_TOO_LARGE.into());
    }
    let mut r = 1.0;
    for n in 2..=(x as u64) { r *= n as f64; }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(s: &str) -> f64 { eval_expression(s, EvalContext::default()).unwrap() }

    #[test] fn precedence_and_parentheses() { assert_eq!(e("(2+2)*4"), 16.0); assert_eq!(e("2+3*4"), 14.0); }
    #[test] fn fixed_unary_minus_cases() { assert_eq!(e("2*-3"), -6.0); assert_eq!(e("2--3"), 5.0); assert_eq!(e("-(2+3)"), -5.0); }
    #[test] fn exponent_sign_is_not_confused_with_subtraction() { assert!((e("1e-3") - 0.001).abs() < 1e-15); }
    #[test]
    fn double_star_is_general_right_associative_exponentiation() {
        assert_eq!(e("2**2"), 4.0);
        assert_eq!(e("5**2"), 25.0);
        assert_eq!(e("2**3"), 8.0);
        assert_eq!(e("2**-3"), 0.125);
        assert_eq!(e("9**0.5"), 3.0);
        assert_eq!(e("4**1.5"), 8.0);
        assert_eq!(e("2**(1+2)"), 8.0);
        assert_eq!(e("2**3**2"), 512.0);
        assert_eq!(e("2^3"), 8.0);
    }
    #[test]
    fn common_named_math_forms_are_accepted() {
        assert_eq!(e("sqrt(25)"), 5.0);
        assert!((e("pi") - std::f64::consts::PI).abs() < 1.0e-15);
        assert!((e("pi()") - std::f64::consts::PI).abs() < 1.0e-15);
        assert!((e("sin(30)") - 0.5).abs() < 1.0e-12);
        assert!((e("cos(60)") - 0.5).abs() < 1.0e-12);
        assert!((e("tan(45)") - 1.0).abs() < 1.0e-12);
    }
    #[test]
    fn pasted_factorial_supports_postfix_and_named_forms() {
        assert_eq!(e("0!"), 1.0);
        assert_eq!(e("5!"), 120.0);
        assert_eq!(e("(3+2)!"), 120.0);
        assert_eq!(e("3!+2"), 8.0);
        assert_eq!(e("2^3!"), 64.0);
        assert_eq!(e("factorial(6)"), 720.0);
        assert_eq!(e("fact(4)"), 24.0);
        assert_eq!(eval_expression("(-1)!", EvalContext::default()).unwrap_err(), INVALID_FUNCTION_INPUT);
        assert_eq!(eval_expression("3.5!", EvalContext::default()).unwrap_err(), INVALID_FUNCTION_INPUT);
        assert_eq!(eval_expression("171!", EvalContext::default()).unwrap_err(), RESULT_TOO_LARGE);
    }
    #[test] fn nesting() { assert_eq!(e("3*(4+(5*2))"), 42.0); }
    #[test] fn comma_decimal() { assert_eq!(e("1,5+2"), 3.5); }
    #[test] fn period_decimal() { assert_eq!(e("1.5+2"), 3.5); }
    #[test] fn mixed_grouping_conventions_are_accepted() {
        assert_eq!(e("1,234.5+0.5"), 1235.0);
        assert_eq!(e("1.234,5+0,5"), 1235.0);
    }
    #[test] fn locale_grouped_integer_is_accepted() {
        let ctx = EvalContext { angle: AngleMode::Degrees, decimal_separator: ',', thousands_separator: Some('.') };
        assert_eq!(eval_expression("1.234.567+1", ctx).unwrap(), 1_234_568.0);
    }
    #[test] fn ambiguous_single_group_separator_follows_locale() {
        let comma_decimal = EvalContext { angle: AngleMode::Degrees, decimal_separator: ',', thousands_separator: Some('.') };
        let period_decimal = EvalContext { angle: AngleMode::Degrees, decimal_separator: '.', thousands_separator: Some(',') };
        assert_eq!(eval_expression("1.000", comma_decimal).unwrap(), 1000.0);
        assert_eq!(eval_expression("1,000", period_decimal).unwrap(), 1000.0);
        assert_eq!(eval_expression("1,5", period_decimal).unwrap(), 1.5);
        assert_eq!(eval_expression("1.5", comma_decimal).unwrap(), 1.5);
    }
    #[test] fn invalid_suffix_is_rejected_transactionally() { assert!(eval_expression("12+34@56", EvalContext::default()).is_err()); }

    #[test]
    fn win95_math_errors_are_distinguished() {
        let ctx = EvalContext::default();
        assert_eq!(eval_expression("sqrt(-1)", ctx).unwrap_err(), FUNCTION_UNDEFINED);
        assert_eq!(eval_expression("ln(0)", ctx).unwrap_err(), FUNCTION_UNDEFINED);
        assert_eq!(eval_expression("log(-1)", ctx).unwrap_err(), FUNCTION_UNDEFINED);
        assert_eq!(eval_expression("asin(2)", ctx).unwrap_err(), INVALID_FUNCTION_INPUT);
        assert_eq!(eval_expression("acos(-2)", ctx).unwrap_err(), INVALID_FUNCTION_INPUT);
        assert_eq!(eval_expression("(-2)^0.5", ctx).unwrap_err(), INVALID_FUNCTION_INPUT);
        assert_eq!(eval_expression("10^400", ctx).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(eval_expression("10^-400", ctx).unwrap_err(), RESULT_TOO_SMALL);
        assert_eq!(eval_expression("1e999", ctx).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(eval_expression("1e-999", ctx).unwrap_err(), RESULT_TOO_SMALL);
        assert_eq!(eval_expression("1e154*1e154", ctx).unwrap_err(), RESULT_TOO_LARGE);
        assert_eq!(eval_expression("1e307/0.1", ctx).unwrap_err(), RESULT_TOO_LARGE);
    }

    #[test]
    fn tangent_asymptote_is_reported_as_undefined() {
        assert_eq!(
            eval_expression("tan(90)", EvalContext::default()).unwrap_err(),
            FUNCTION_UNDEFINED
        );
    }
    #[test]
    fn inverse_power_internal_operator_matches_classic_error_path() {
        let ctx = EvalContext::default();
        assert!((eval_expression("27 root 3", ctx).unwrap() - 3.0).abs() < 1.0e-12);
        assert_eq!(
            eval_expression("8 root 0", ctx).unwrap_err(),
            INVALID_FUNCTION_INPUT
        );
    }

    #[test]
    fn compiled_graph_expression_reuses_tokens_and_evaluates_x() {
        let compiled = CompiledExpression::parse("x^2-4", EvalContext::default()).unwrap();
        assert_eq!(compiled.evaluate_at(-2.0).unwrap(), 0.0);
        assert_eq!(compiled.evaluate_at(3.0).unwrap(), 5.0);
        assert!(eval_expression("x+1", EvalContext::default()).is_err());
    }

}
