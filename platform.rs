//! Bounded user-visible calculation history.
//!
//! This is deliberately separate from the Ctrl+Z/Ctrl+Y state history.  The
//! latter restores calculator snapshots, while this module records completed
//! calculations for the optional History side panel.  Keeping the two concepts
//! separate means clearing the visible log never destroys the user's undo
//! stack, and undoing an action does not pretend that the action was never
//! performed.
//!
//! Numeric punctuation is stored canonically with `.` as the decimal mark.
//! The History pane localizes that canonical text each time it is rendered, so
//! changing the configured separator immediately updates entries that are
//! already visible instead of preserving the punctuation active at creation.

const DEFAULT_LIMIT: usize = 256;

/// Replace a decimal mark only when it belongs to a numeric token.
///
/// Requiring an ASCII digit on both sides avoids changing punctuation in error
/// messages, function argument separators, or ordinary prose.  OpenCalc always
/// writes a leading zero for fractional values and strips a trailing radix mark
/// before recording History, so every decimal point it generates satisfies
/// this test.
fn translate_numeric_decimal_marks(text: &str, from: char, to: char) -> String {
    if from == to {
        return text.to_owned();
    }

    let chars: Vec<char> = text.chars().collect();
    let mut translated = String::with_capacity(text.len());
    for (index, ch) in chars.iter().copied().enumerate() {
        let numeric_mark = ch == from
            && index > 0
            && index + 1 < chars.len()
            && chars[index - 1].is_ascii_digit()
            && chars[index + 1].is_ascii_digit();
        translated.push(if numeric_mark { to } else { ch });
    }
    translated
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalculationLogEntry {
    /// Expression stored with invariant `.` decimal punctuation.
    pub expression: String,
    /// Numeric result stored with invariant `.` punctuation, or an unchanged
    /// runtime error string when `value` is `None`.
    pub result: String,
    /// Exact numeric result captured when the operation completed.  Keeping
    /// this alongside the formatted display text lets the History UI recall a
    /// value without reparsing locale-specific output or re-running the
    /// original expression.
    pub value: Option<f64>,
}

impl CalculationLogEntry {
    pub fn localized_expression(&self, decimal_separator: char) -> String {
        translate_numeric_decimal_marks(&self.expression, '.', decimal_separator)
    }

    pub fn localized_result(&self, decimal_separator: char) -> String {
        if self.value.is_some() {
            translate_numeric_decimal_marks(&self.result, '.', decimal_separator)
        } else {
            self.result.clone()
        }
    }
}

#[derive(Clone, Debug)]
pub struct CalculationLog {
    entries: Vec<CalculationLogEntry>,
    limit: usize,
}

impl Default for CalculationLog {
    fn default() -> Self {
        Self::with_limit(DEFAULT_LIMIT)
    }
}

impl CalculationLog {
    pub fn with_limit(limit: usize) -> Self {
        Self {
            entries: Vec::new(),
            limit: limit.max(1),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Record text that is already in invariant `.` notation.
    pub fn push(
        &mut self,
        expression: impl Into<String>,
        result: impl Into<String>,
        value: Option<f64>,
    ) {
        self.push_localized(expression, result, value, '.');
    }

    /// Record text produced using the currently selected decimal separator.
    /// Numeric marks are normalized once, then localized on every History
    /// refresh.  Error strings are kept verbatim.
    pub fn push_localized(
        &mut self,
        expression: impl Into<String>,
        result: impl Into<String>,
        value: Option<f64>,
        decimal_separator: char,
    ) {
        let expression = expression.into();
        let result = result.into();
        if expression.trim().is_empty() || result.trim().is_empty() {
            return;
        }

        let expression = translate_numeric_decimal_marks(&expression, decimal_separator, '.');
        let result = if value.is_some() {
            translate_numeric_decimal_marks(&result, decimal_separator, '.')
        } else {
            result
        };

        self.entries.push(CalculationLogEntry { expression, result, value });
        if self.entries.len() > self.limit {
            self.entries.remove(0);
        }
    }

    /// Iterate newest first, matching the visual order of the reference panel.
    pub fn newest_first(&self) -> impl Iterator<Item = &CalculationLogEntry> {
        self.entries.iter().rev()
    }

    /// Return one entry by its visual (newest-first) index.
    pub fn newest(&self, index: usize) -> Option<&CalculationLogEntry> {
        self.entries.iter().rev().nth(index)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_entry_is_rendered_first() {
        let mut log = CalculationLog::with_limit(8);
        log.push("1 + 1", "2", Some(2.0));
        log.push("2 * 3", "6", Some(6.0));
        let expressions: Vec<_> = log.newest_first().map(|entry| entry.expression.as_str()).collect();
        assert_eq!(expressions, ["2 * 3", "1 + 1"]);
    }

    #[test]
    fn log_is_bounded() {
        let mut log = CalculationLog::with_limit(2);
        log.push("1", "1", Some(1.0));
        log.push("2", "2", Some(2.0));
        log.push("3", "3", Some(3.0));
        assert_eq!(log.len(), 2);
        let expressions: Vec<_> = log.newest_first().map(|entry| entry.expression.as_str()).collect();
        assert_eq!(expressions, ["3", "2"]);
    }

    #[test]
    fn newest_lookup_keeps_exact_recall_value() {
        let mut log = CalculationLog::default();
        log.push("1 / 3", "0.3333333333333333", Some(1.0 / 3.0));
        assert_eq!(log.newest(0).and_then(|entry| entry.value), Some(1.0 / 3.0));
        assert!(log.newest(1).is_none());
    }

    #[test]
    fn error_entry_has_no_recall_value() {
        let mut log = CalculationLog::default();
        log.push("1 / 0", "Cannot divide by zero.", None);
        assert_eq!(log.newest(0).and_then(|entry| entry.value), None);
    }

    #[test]
    fn clear_removes_every_visible_entry() {
        let mut log = CalculationLog::default();
        log.push("sqrt(9)", "3", Some(3.0));
        log.clear();
        assert!(log.is_empty());
    }

    #[test]
    fn localized_entries_are_stored_canonically_and_rendered_on_demand() {
        let mut log = CalculationLog::default();
        log.push_localized("1,5 + 2,25", "3,75", Some(3.75), ',');
        let entry = log.newest(0).unwrap();

        assert_eq!(entry.expression, "1.5 + 2.25");
        assert_eq!(entry.result, "3.75");
        assert_eq!(entry.localized_expression(','), "1,5 + 2,25");
        assert_eq!(entry.localized_result(','), "3,75");
        assert_eq!(entry.localized_expression('.'), "1.5 + 2.25");
        assert_eq!(entry.localized_result('.'), "3.75");
    }

    #[test]
    fn changing_numeric_marks_does_not_change_non_numeric_punctuation() {
        assert_eq!(
            translate_numeric_decimal_marks("f(1, 2); message.", ',', '.'),
            "f(1, 2); message."
        );
    }

    #[test]
    fn error_message_period_is_never_treated_as_a_decimal_mark() {
        let mut log = CalculationLog::default();
        log.push_localized("1 / 0", "Cannot divide by zero.", None, ',');
        let entry = log.newest(0).unwrap();
        assert_eq!(entry.localized_result(','), "Cannot divide by zero.");
    }
}
