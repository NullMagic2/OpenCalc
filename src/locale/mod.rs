//! Operating-system numeric locale support.
//!
//! Calculator arithmetic always uses an invariant `.` radix internally.  This
//! module only translates between that canonical representation and the user's
//! configured non-monetary decimal separator.

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod imp;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod imp;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[path = "other.rs"]
mod imp;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumericLocale {
    decimal_separator: char,
    thousands_separator: Option<char>,
}

impl Default for NumericLocale {
    fn default() -> Self {
        Self::system()
    }
}

impl NumericLocale {
    pub fn system() -> Self {
        let (decimal, thousands) = imp::numeric_symbols();
        let decimal_separator = decimal
            .chars()
            .next()
            .filter(|ch| !ch.is_whitespace())
            .unwrap_or('.');
        let thousands_separator = thousands
            .chars()
            .next()
            .filter(|ch| *ch != decimal_separator);
        Self {
            decimal_separator,
            thousands_separator,
        }
    }

    /// Build an explicit decimal convention selected by the user.  Calculator
    /// supports the two conventional punctuation pairs exposed by its menu.
    pub const fn with_decimal_separator(decimal_separator: char) -> Self {
        if decimal_separator == ',' {
            Self { decimal_separator: ',', thousands_separator: Some('.') }
        } else {
            Self { decimal_separator: '.', thousands_separator: Some(',') }
        }
    }

    #[cfg(test)]
    pub const fn new(decimal_separator: char, thousands_separator: Option<char>) -> Self {
        Self {
            decimal_separator,
            thousands_separator,
        }
    }

    pub const fn decimal_separator(self) -> char {
        self.decimal_separator
    }

    pub const fn thousands_separator(self) -> Option<char> {
        self.thousands_separator
    }

    /// Convert the calculator's invariant representation into the user's
    /// display representation. No thousands grouping is inserted: classic
    /// Calculator did not implicitly enable digit grouping merely because a
    /// locale defined a grouping character.
    pub fn localize(self, canonical: &str) -> String {
        if self.decimal_separator == '.' {
            canonical.to_owned()
        } else {
            canonical
                .chars()
                .map(|ch| if ch == '.' { self.decimal_separator } else { ch })
                .collect()
        }
    }

    /// Convert text already shown by Calculator back to the invariant form.
    pub fn canonicalize_display(self, localized: &str) -> String {
        let mut out = String::with_capacity(localized.len());
        for ch in localized.chars() {
            if ch == self.decimal_separator {
                out.push('.');
            } else if Some(ch) == self.thousands_separator {
                // We currently do not insert grouping ourselves, but accepting
                // it here makes copied/formatted values safe to feed back in.
            } else {
                out.push(ch);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comma_locale_localizes_and_round_trips() {
        let locale = NumericLocale::new(',', Some('.'));
        assert_eq!(locale.localize("123.5"), "123,5");
        assert_eq!(locale.canonicalize_display("123,5"), "123.5");
    }

    #[test]
    fn period_locale_is_identity() {
        let locale = NumericLocale::new('.', Some(','));
        assert_eq!(locale.localize("0."), "0.");
        assert_eq!(locale.canonicalize_display("12.75"), "12.75");
    }
}
