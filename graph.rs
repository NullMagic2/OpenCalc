//! Operating-system numeric locale support.
//!
//! Calculator arithmetic always uses an invariant `.` radix internally.  This
//! module only translates between that canonical representation and the user's
//! configured non-monetary decimal separator.

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
        let (decimal, thousands) = platform_numeric_symbols();
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

#[cfg(target_os = "windows")]
fn platform_numeric_symbols() -> (String, String) {
    use std::ptr::null_mut;

    const LOCALE_NAME_MAX_LENGTH: usize = 85;
    const LOCALE_SDECIMAL: u32 = 0x0000_000E;
    const LOCALE_STHOUSAND: u32 = 0x0000_000F;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetUserDefaultLocaleName(locale_name: *mut u16, count: i32) -> i32;
        fn GetLocaleInfoEx(
            locale_name: *const u16,
            locale_type: u32,
            data: *mut u16,
            count: i32,
        ) -> i32;
    }

    unsafe fn locale_value(locale_name: *const u16, kind: u32) -> Option<String> {
        let needed = GetLocaleInfoEx(locale_name, kind, null_mut(), 0);
        if needed <= 1 {
            return None;
        }
        let mut buffer = vec![0u16; needed as usize];
        let written = GetLocaleInfoEx(locale_name, kind, buffer.as_mut_ptr(), needed);
        if written <= 1 {
            return None;
        }
        Some(String::from_utf16_lossy(&buffer[..written as usize - 1]))
    }

    unsafe {
        let mut name = [0u16; LOCALE_NAME_MAX_LENGTH];
        let written = GetUserDefaultLocaleName(name.as_mut_ptr(), name.len() as i32);
        if written <= 1 {
            return (".".to_owned(), ",".to_owned());
        }
        let decimal = locale_value(name.as_ptr(), LOCALE_SDECIMAL).unwrap_or_else(|| ".".to_owned());
        let thousands = locale_value(name.as_ptr(), LOCALE_STHOUSAND).unwrap_or_default();
        (decimal, thousands)
    }
}

#[cfg(target_os = "linux")]
fn platform_numeric_symbols() -> (String, String) {
    use std::ffi::{c_char, c_int, CStr};

    // Linux/glibc follows the POSIX locale layout; the first three fields of
    // `struct lconv` are the non-monetary numeric fields used here.
    #[repr(C)]
    struct LConvPrefix {
        decimal_point: *mut c_char,
        thousands_sep: *mut c_char,
        grouping: *mut c_char,
    }

    const LC_NUMERIC: c_int = 1;

    unsafe extern "C" {
        fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
        fn localeconv() -> *mut LConvPrefix;
    }

    unsafe fn c_string(ptr: *const c_char) -> String {
        if ptr.is_null() {
            return String::new();
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }

    unsafe {
        // Empty locale selects LC_NUMERIC from the user's environment
        // (LC_ALL/LC_NUMERIC/LANG) instead of remaining in the C locale.
        let _ = setlocale(LC_NUMERIC, b"\0".as_ptr().cast());
        let info = localeconv();
        if info.is_null() {
            return (".".to_owned(), ",".to_owned());
        }
        let decimal = c_string((*info).decimal_point);
        let thousands = c_string((*info).thousands_sep);
        (
            if decimal.is_empty() { ".".to_owned() } else { decimal },
            thousands,
        )
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_numeric_symbols() -> (String, String) {
    (".".to_owned(), ",".to_owned())
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
