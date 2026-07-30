//! Linux/glibc numeric-locale backend.

pub(super) fn numeric_symbols() -> (String, String) {
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

