//! Linux numeric-locale discovery without native FFI.

use std::env;

pub(super) fn numeric_symbols() -> (String, String) {
    let locale = ["LC_ALL", "LC_NUMERIC", "LANG"]
        .into_iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
        .unwrap_or_default()
        .to_ascii_lowercase();

    let language = locale
        .split(['_', '-', '.', '@'])
        .next()
        .unwrap_or_default();

    if uses_decimal_comma(language) {
        (",".to_owned(), ".".to_owned())
    } else {
        (".".to_owned(), ",".to_owned())
    }
}

fn uses_decimal_comma(language: &str) -> bool {
    matches!(
        language,
        "af"
            | "be"
            | "bg"
            | "ca"
            | "cs"
            | "da"
            | "de"
            | "el"
            | "es"
            | "et"
            | "eu"
            | "fi"
            | "fr"
            | "hu"
            | "id"
            | "is"
            | "it"
            | "lt"
            | "lv"
            | "mk"
            | "nl"
            | "no"
            | "pl"
            | "pt"
            | "ro"
            | "ru"
            | "sk"
            | "sl"
            | "sq"
            | "sv"
            | "tr"
            | "uk"
            | "vi"
    )
}

#[cfg(test)]
mod tests {
    use super::uses_decimal_comma;

    #[test]
    fn common_comma_locales_are_detected() {
        assert!(uses_decimal_comma("pt"));
        assert!(uses_decimal_comma("de"));
        assert!(!uses_decimal_comma("en"));
    }
}
