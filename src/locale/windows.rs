//! Windows numeric-locale backend.

pub(super) fn numeric_symbols() -> (String, String) {
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

