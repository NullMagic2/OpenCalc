//! Portable numeric-locale fallback.

pub(super) fn numeric_symbols() -> (String, String) {
    (".".to_owned(), ",".to_owned())
}

