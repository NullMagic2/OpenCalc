//! Small Windows integration helpers kept outside the wxDragon layout code.

use crate::errors::NOT_ENOUGH_MEMORY_FOR_DATA;
use crate::i18n::Language;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(target_os = "windows")]
mod windows {
    use crate::errors::{CANNOT_OPEN_CLIPBOARD, NOT_ENOUGH_MEMORY_FOR_DATA};
    use std::ffi::c_void;
    use std::collections::HashMap;
    use std::ptr::null_mut;
    use std::sync::{Mutex, OnceLock};

    type Bool = i32;
    type Uint = u32;
    type Handle = *mut c_void;
    type Hwnd = Handle;
    type Hglobal = Handle;
    type Hdc = Handle;
    type Hgdiobj = Handle;
    type Hbrush = Handle;
    type Hmenu = Handle;
    type Hmonitor = Handle;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Rect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Point {
        x: i32,
        y: i32,
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct MonitorInfo {
        cb_size: Uint,
        monitor: Rect,
        work: Rect,
        flags: Uint,
    }

    #[repr(C)]
    struct ToolInfoW {
        cb_size: Uint,
        flags: Uint,
        hwnd: Hwnd,
        id: usize,
        rect: Rect,
        instance: Handle,
        text: *mut u16,
        lparam: isize,
        reserved: *mut c_void,
    }

    #[repr(C)]
    struct PaintStruct {
        hdc: Hdc,
        erase: Bool,
        paint: Rect,
        restore: Bool,
        inc_update: Bool,
        reserved: [u8; 32],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct LogFontW {
        height: i32,
        width: i32,
        escapement: i32,
        orientation: i32,
        weight: i32,
        italic: u8,
        underline: u8,
        strike_out: u8,
        char_set: u8,
        out_precision: u8,
        clip_precision: u8,
        quality: u8,
        pitch_and_family: u8,
        face_name: [u16; 32],
    }

    impl Default for LogFontW {
        fn default() -> Self {
            Self {
                height: 0,
                width: 0,
                escapement: 0,
                orientation: 0,
                weight: 0,
                italic: 0,
                underline: 0,
                strike_out: 0,
                char_set: 0,
                out_precision: 0,
                clip_precision: 0,
                quality: 0,
                pitch_and_family: 0,
                face_name: [0; 32],
            }
        }
    }

    type SubclassProc = Option<
        unsafe extern "system" fn(Hwnd, Uint, usize, isize, usize, usize) -> isize,
    >;
    type EnumWindowsProc = Option<unsafe extern "system" fn(Hwnd, isize) -> Bool>;

    const CF_UNICODETEXT: Uint = 13;
    const GMEM_MOVEABLE: Uint = 0x0002;
    const MB_OK: Uint = 0x0000;
    const MB_ICONINFORMATION: Uint = 0x0040;
    const WM_PAINT: Uint = 0x000F;
    const WM_SIZE: Uint = 0x0005;
    const WM_ERASEBKGND: Uint = 0x0014;
    const WM_CONTEXTMENU: Uint = 0x007B;
    const WM_ACTIVATE: Uint = 0x0006;
    const WM_MOUSEACTIVATE: Uint = 0x0021;
    const WA_INACTIVE: usize = 0;
    const MA_NOACTIVATE: isize = 3;
    const WM_LBUTTONDOWN: Uint = 0x0201;
    const WM_RBUTTONDOWN: Uint = 0x0204;
    const WM_MBUTTONDOWN: Uint = 0x0207;
    const WM_XBUTTONDOWN: Uint = 0x020B;
    const WM_NCLBUTTONDOWN: Uint = 0x00A1;
    const WM_NCRBUTTONDOWN: Uint = 0x00A4;
    const WM_NCMBUTTONDOWN: Uint = 0x00A7;
    const WM_NCXBUTTONDOWN: Uint = 0x00AB;
    const WM_KEYDOWN: Uint = 0x0100;
    const WM_TIMER: Uint = 0x0113;
    const VK_ESCAPE: usize = 0x1B;
    const WM_GETFONT: Uint = 0x0031;
    const EM_POSFROMCHAR: Uint = 0x00D6;
    const EM_CHARFROMPOS: Uint = 0x00D7;
    const EM_LINEINDEX: Uint = 0x00BB;
    const WM_SETICON: Uint = 0x0080;
    const WM_NCDESTROY: Uint = 0x0082;
    const WM_PRINTCLIENT: Uint = 0x0318;
    const WM_NCHITTEST: Uint = 0x0084;
    const WS_CLIPSIBLINGS: i32 = 0x0400_0000;
    // Purely decorative statics must never claim the mouse.  wxStaticText is
    // created with SS_NOTIFY on MSW, so DefSubclassProc answers WM_NCHITTEST
    // with HTCLIENT and the control swallows every click inside its rectangle.
    // The group box is a full-size rectangle drawn *behind* the radio buttons
    // and checkboxes it frames, so that swallows exactly the clicks meant for
    // the selectors.  HTTRANSPARENT makes Windows keep searching underneath.
    const HTTRANSPARENT: isize = -1;
    const BM_GETSTATE: Uint = 0x00F2;
    const BM_SETSTATE: Uint = 0x00F3;
    const BST_PUSHED: usize = 0x0004;
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;
    const LR_DEFAULTCOLOR: Uint = 0x0000;
    const COLOR_BTNFACE: i32 = 15;
    const COLOR_BTNSHADOW: i32 = 16;
    const COLOR_BTNHIGHLIGHT: i32 = 20;
    const COLOR_3DDKSHADOW: i32 = 21;
    const COLOR_3DLIGHT: i32 = 22;
    const COLOR_WINDOW: i32 = 5;
    const COLOR_WINDOWTEXT: i32 = 8;
    const COLOR_GRAYTEXT: i32 = 17;
    const EDGE_ETCHED: Uint = 0x0006;
    const EDGE_SUNKEN: Uint = 0x000A;
    const BF_LEFT: Uint = 0x0001;
    const BF_BOTTOM: Uint = 0x0002;
    const BF_RECT: Uint = 0x000F;
    const DT_CENTER: Uint = 0x0001;
    const DT_RIGHT: Uint = 0x0002;
    const DT_VCENTER: Uint = 0x0004;
    const DT_SINGLELINE: Uint = 0x0020;
    const DT_NOPREFIX: Uint = 0x0800;
    const TRANSPARENT: i32 = 1;
    const OPAQUE: i32 = 2;
    const CLEARTYPE_NATURAL_QUALITY: u8 = 6;
    const CLASSIC_BUTTON_SUBCLASS_ID: usize = 0xCA1C_9501;
    const CLASSIC_BUTTON_KEY_TIMER_ID: usize = 0xCA1C_9511;
    const CLASSIC_BUTTON_KEY_PRESS_MS: Uint = 85;
    const CLASSIC_FIELD_SUBCLASS_ID: usize = 0xCA1C_9502;
    const CLASSIC_GROUP_SUBCLASS_ID: usize = 0xCA1C_9505;
    const CLASSIC_SEPARATOR_SUBCLASS_ID: usize = 0xCA1C_9504;
    const CLASSIC_VERTICAL_SEPARATOR_SUBCLASS_ID: usize = 0xCA1C_950A;
    const CONTEXT_HELP_SUBCLASS_ID: usize = 0xCA1C_9503;
    const CONTEXT_HELP_DISMISS_SUBCLASS_ID: usize = 0xCA1C_9507;
    const WINDOW_STATE_SUBCLASS_ID: usize = 0xCA1C_9508;
    const COMPANION_ACTIVE_SUBCLASS_ID: usize = 0xCA1C_9509;
    const COMPANION_OWNER_SUBCLASS_ID: usize = 0xCA1C_950B;
    const ID_WHATS_THIS: usize = 0xCA1C;
    const MF_STRING: Uint = 0x0000;
    const TPM_RIGHTBUTTON: Uint = 0x0002;
    const TPM_RETURNCMD: Uint = 0x0100;
    const WS_POPUP: Uint = 0x8000_0000;
    const WS_EX_TOPMOST: Uint = 0x0000_0008;
    const TTS_ALWAYSTIP: Uint = 0x0001;
    const TTS_NOPREFIX: Uint = 0x0002;
    const TTF_IDISHWND: Uint = 0x0001;
    const TTF_TRACK: Uint = 0x0020;
    const TTF_ABSOLUTE: Uint = 0x0080;
    const TTF_TRANSPARENT: Uint = 0x0100;
    const WM_USER: Uint = 0x0400;
    const TTM_ADDTOOLW: Uint = WM_USER + 50;
    const TTM_TRACKACTIVATE: Uint = WM_USER + 17;
    const TTM_TRACKPOSITION: Uint = WM_USER + 18;
    const TTM_SETTIPBKCOLOR: Uint = WM_USER + 19;
    const TTM_SETTIPTEXTCOLOR: Uint = WM_USER + 20;
    const TTM_SETMAXTIPWIDTH: Uint = WM_USER + 24;
    const TTM_SETWINDOWTHEME: Uint = 0x2000 + 11;

    // TTM_ADDTOOL validates TOOLINFO.cbSize and rejects any value it does not
    // recognise, returning FALSE without adding the tool.  lpReserved was only
    // added to TOOLINFO in comctl32 version 6.  This executable ships without a
    // Common Controls v6 manifest -- that is exactly why ui.rs has to set the
    // "msw.no-manifest-check" wxWidgets option -- so it binds against comctl32
    // v5, whose accepted TOOLINFO stops after lParam.  Reporting the full
    // size_of::<ToolInfoW>() therefore failed the size check, TTM_ADDTOOLW
    // returned 0, show_context_tooltip destroyed the tooltip window and
    // returned silently, and "What's This?" produced no popup at all.
    //
    // comctl32 v6 still accepts the pre-v6 size for backward compatibility, so
    // reporting the size up to and including lParam is correct on both.
    const TTTOOLINFO_V2_SIZE: Uint =
        (std::mem::size_of::<ToolInfoW>() - std::mem::size_of::<*mut c_void>()) as Uint;
    const SWP_NOSIZE: Uint = 0x0001;
    const SWP_NOMOVE: Uint = 0x0002;
    const SWP_NOZORDER: Uint = 0x0004;
    const SWP_NOACTIVATE: Uint = 0x0010;
    const SWP_FRAMECHANGED: Uint = 0x0020;
    const GWL_STYLE: i32 = -16;
    const GWLP_HWNDPARENT: i32 = -8;
    const WS_THICKFRAME: i32 = 0x0004_0000;
    const WS_MAXIMIZEBOX: i32 = 0x0001_0000;
    const MONITOR_DEFAULTTONEAREST: Uint = 0x0000_0002;
    const SIZE_RESTORED: usize = 0;
    const SIZE_MINIMIZED: usize = 1;
    const SIZE_MAXIMIZED: usize = 2;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn OpenClipboard(hwnd: Hwnd) -> Bool;
        fn CloseClipboard() -> Bool;
        fn EmptyClipboard() -> Bool;
        fn GetClipboardData(format: Uint) -> Handle;
        fn SetClipboardData(format: Uint, memory: Handle) -> Handle;
        fn MessageBoxW(hwnd: Hwnd, text: *const u16, caption: *const u16, kind: Uint) -> i32;
        fn CreateIconFromResourceEx(
            bits: *mut u8,
            size: Uint,
            icon: Bool,
            version: Uint,
            width: i32,
            height: i32,
            flags: Uint,
        ) -> Handle;
        fn SendMessageW(hwnd: Hwnd, message: Uint, wparam: usize, lparam: isize) -> isize;
        fn BeginPaint(hwnd: Hwnd, paint: *mut PaintStruct) -> Hdc;
        fn EndPaint(hwnd: Hwnd, paint: *const PaintStruct) -> Bool;
        fn InvalidateRect(hwnd: Hwnd, rect: *const Rect, erase: Bool) -> Bool;
        fn GetClientRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
        fn GetWindowRect(hwnd: Hwnd, rect: *mut Rect) -> Bool;
        fn MonitorFromWindow(hwnd: Hwnd, flags: Uint) -> Hmonitor;
        fn GetMonitorInfoW(monitor: Hmonitor, info: *mut MonitorInfo) -> Bool;
        fn ClientToScreen(hwnd: Hwnd, point: *mut Point) -> Bool;
        fn GetDpiForWindow(hwnd: Hwnd) -> Uint;
        fn GetWindowLongW(hwnd: Hwnd, index: i32) -> i32;
        fn SetWindowLongW(hwnd: Hwnd, index: i32, value: i32) -> i32;
        #[cfg(target_pointer_width = "64")]
        fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;
        #[cfg(target_pointer_width = "32")]
        #[link_name = "SetWindowLongW"]
        fn SetWindowLongPtrW(hwnd: Hwnd, index: i32, value: isize) -> isize;
        fn SetWindowPos(
            hwnd: Hwnd,
            insert_after: Hwnd,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            flags: Uint,
        ) -> Bool;
        fn GetWindowTextLengthW(hwnd: Hwnd) -> i32;
        fn GetWindowTextW(hwnd: Hwnd, text: *mut u16, max_count: i32) -> i32;
        fn GetSysColorBrush(index: i32) -> Hbrush;
        fn GetSysColor(index: i32) -> u32;
        fn FillRect(hdc: Hdc, rect: *const Rect, brush: Hbrush) -> i32;
        fn DrawEdge(hdc: Hdc, rect: *mut Rect, edge: Uint, flags: Uint) -> Bool;
        fn DrawTextW(hdc: Hdc, text: *const u16, count: i32, rect: *mut Rect, format: Uint) -> i32;
        fn IsWindowEnabled(hwnd: Hwnd) -> Bool;
        fn SetProcessDpiAwarenessContext(value: isize) -> Bool;
        fn CreatePopupMenu() -> Hmenu;
        fn AppendMenuW(menu: Hmenu, flags: Uint, id: usize, text: *const u16) -> Bool;
        fn TrackPopupMenu(
            menu: Hmenu,
            flags: Uint,
            x: i32,
            y: i32,
            reserved: i32,
            owner: Hwnd,
            rect: *const Rect,
        ) -> i32;
        fn DestroyMenu(menu: Hmenu) -> Bool;
        fn GetCursorPos(point: *mut Point) -> Bool;
        fn GetParent(hwnd: Hwnd) -> Hwnd;
        fn GetForegroundWindow() -> Hwnd;
        fn GetActiveWindow() -> Hwnd;
        fn GetFocus() -> Hwnd;
        fn SetForegroundWindow(hwnd: Hwnd) -> Bool;
        fn SetActiveWindow(hwnd: Hwnd) -> Hwnd;
        fn EnumChildWindows(parent: Hwnd, callback: EnumWindowsProc, lparam: isize) -> Bool;
        fn SetTimer(hwnd: Hwnd, id: usize, elapse_ms: Uint, timer_proc: *const c_void) -> usize;
        fn KillTimer(hwnd: Hwnd, id: usize) -> Bool;
        fn IsWindowVisible(hwnd: Hwnd) -> Bool;
        fn CreateWindowExW(
            ex_style: Uint,
            class_name: *const u16,
            window_name: *const u16,
            style: Uint,
            x: i32,
            y: i32,
            width: i32,
            height: i32,
            parent: Hwnd,
            menu: Hmenu,
            instance: Handle,
            param: *mut c_void,
        ) -> Hwnd;
        fn DestroyWindow(hwnd: Hwnd) -> Bool;
    }

    #[link(name = "gdi32")]
    unsafe extern "system" {
        fn SelectObject(hdc: Hdc, object: Hgdiobj) -> Hgdiobj;
        fn SetBkMode(hdc: Hdc, mode: i32) -> i32;
        fn SetBkColor(hdc: Hdc, colour: u32) -> u32;
        fn SetTextColor(hdc: Hdc, colour: u32) -> u32;
        fn GetObjectW(object: Hgdiobj, bytes: i32, data: *mut c_void) -> i32;
        fn CreateFontIndirectW(log_font: *const LogFontW) -> Hgdiobj;
        fn DeleteObject(object: Hgdiobj) -> Bool;
    }

    #[link(name = "comctl32")]
    unsafe extern "system" {
        fn SetWindowSubclass(
            hwnd: Hwnd,
            proc: SubclassProc,
            id: usize,
            ref_data: usize,
        ) -> Bool;
        fn RemoveWindowSubclass(hwnd: Hwnd, proc: SubclassProc, id: usize) -> Bool;
        fn DefSubclassProc(hwnd: Hwnd, message: Uint, wparam: usize, lparam: isize) -> isize;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(module_name: *const u16) -> Handle;
        fn GlobalAlloc(flags: Uint, bytes: usize) -> Hglobal;
        fn GlobalLock(memory: Hglobal) -> *mut c_void;
        fn GlobalUnlock(memory: Hglobal) -> Bool;
        fn GlobalFree(memory: Hglobal) -> Hglobal;
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn colour_ref(red: u8, green: u8, blue: u8) -> u32 {
        red as u32 | ((green as u32) << 8) | ((blue as u32) << 16)
    }

    #[derive(Default)]
    struct ActiveTooltip {
        hwnd: usize,
        text: Option<Box<[u16]>>,
    }

    #[derive(Debug)]
    struct ContextHelpData {
        body: String,
        menu_label: String,
    }

    fn active_tooltip() -> &'static Mutex<ActiveTooltip> {
        static ACTIVE: OnceLock<Mutex<ActiveTooltip>> = OnceLock::new();
        ACTIVE.get_or_init(|| Mutex::new(ActiveTooltip::default()))
    }

    /// Track the heap allocation currently supplied to each subclass.  Calling
    /// SetWindowSubclass again with the same proc/id updates dwRefData but does
    /// not free the old pointer, so language switching needs to retire it here.
    fn context_help_bindings() -> &'static Mutex<HashMap<usize, usize>> {
        static BINDINGS: OnceLock<Mutex<HashMap<usize, usize>>> = OnceLock::new();
        BINDINGS.get_or_init(|| Mutex::new(HashMap::new()))
    }

    unsafe fn dismiss_context_tooltip_impl() {
        let Ok(mut active) = active_tooltip().lock() else {
            return;
        };
        if active.hwnd != 0 {
            let _ = DestroyWindow(active.hwnd as Hwnd);
            active.hwnd = 0;
        }
        // The tooltip window has been destroyed before the backing UTF-16 text is released.
        active.text = None;
    }

    pub fn dismiss_context_tooltip() {
        unsafe { dismiss_context_tooltip_impl() }
    }

    fn packed_point(x: i32, y: i32) -> isize {
        let lo = (x as u32) & 0xFFFF;
        let hi = ((y as u32) & 0xFFFF) << 16;
        (lo | hi) as isize
    }

    unsafe fn show_context_tooltip(source: Hwnd, body: &str) {
        dismiss_context_tooltip_impl();
        if body.trim().is_empty() {
            return;
        }

        // TTF_IDISHWND has a non-obvious contract: TOOLINFO.hwnd is the
        // *containing* window and TOOLINFO.uId is the child HWND being described.
        let parent = GetParent(source);
        let owner = if parent.is_null() { source } else { parent };

        let module = GetModuleHandleW(std::ptr::null());
        let class_name = wide("tooltips_class32");
        let tooltip = CreateWindowExW(
            WS_EX_TOPMOST,
            class_name.as_ptr(),
            std::ptr::null(),
            WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
            0,
            0,
            0,
            0,
            owner,
            null_mut(),
            module,
            null_mut(),
        );
        if tooltip.is_null() {
            return;
        }

        const EMPTY_THEME: [u16; 1] = [0];
        let _ = SendMessageW(
            tooltip,
            TTM_SETWINDOWTHEME,
            0,
            EMPTY_THEME.as_ptr() as isize,
        );
        let _ = SendMessageW(
            tooltip,
            TTM_SETTIPBKCOLOR,
            colour_ref(255, 255, 225) as usize,
            0,
        );
        let _ = SendMessageW(
            tooltip,
            TTM_SETTIPTEXTCOLOR,
            colour_ref(0, 0, 0) as usize,
            0,
        );

        let mut text = wide(body).into_boxed_slice();
        let mut tool = ToolInfoW {
            cb_size: TTTOOLINFO_V2_SIZE,
            flags: TTF_IDISHWND | TTF_TRACK | TTF_ABSOLUTE | TTF_TRANSPARENT,
            hwnd: owner,
            id: source as usize,
            rect: Rect::default(),
            instance: module,
            text: text.as_mut_ptr(),
            lparam: 0,
            reserved: null_mut(),
        };

        if SendMessageW(
            tooltip,
            TTM_ADDTOOLW,
            0,
            &mut tool as *mut ToolInfoW as isize,
        ) == 0
        {
            let _ = DestroyWindow(tooltip);
            return;
        }

        let dpi = GetDpiForWindow(source).max(96);
        let max_width = ((420_i64 * dpi as i64 + 48) / 96).clamp(160, 1000) as isize;
        let _ = SendMessageW(tooltip, TTM_SETMAXTIPWIDTH, 0, max_width);
        let mut cursor = Point::default();
        let _ = GetCursorPos(&mut cursor);
        let offset = ((10_i64 * dpi as i64 + 48) / 96).clamp(6, 32) as i32;
        let _ = SendMessageW(
            tooltip,
            TTM_TRACKPOSITION,
            0,
            packed_point(cursor.x + offset, cursor.y + offset),
        );
        let _ = SendMessageW(
            tooltip,
            TTM_TRACKACTIVATE,
            1,
            &mut tool as *mut ToolInfoW as isize,
        );

        if let Ok(mut active) = active_tooltip().lock() {
            active.hwnd = tooltip as usize;
            active.text = Some(text);
        } else {
            let _ = DestroyWindow(tooltip);
        }
    }

    unsafe fn show_whats_this_menu(source: Hwnd, body: &str, menu_label: &str) {
        dismiss_context_tooltip_impl();
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        let label = wide(menu_label);
        if AppendMenuW(menu, MF_STRING, ID_WHATS_THIS, label.as_ptr()) == 0 {
            let _ = DestroyMenu(menu);
            return;
        }
        let mut cursor = Point::default();
        let _ = GetCursorPos(&mut cursor);
        let command = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_RETURNCMD,
            cursor.x,
            cursor.y,
            0,
            source,
            std::ptr::null(),
        );
        let _ = DestroyMenu(menu);
        if command as usize == ID_WHATS_THIS {
            show_context_tooltip(source, body);
        }
    }

    unsafe extern "system" fn context_help_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        ref_data: usize,
    ) -> isize {
        match message {
            WM_CONTEXTMENU => {
                if ref_data != 0 {
                    let data = &*(ref_data as *const ContextHelpData);
                    show_whats_this_menu(hwnd, &data.body, &data.menu_label);
                }
                0
            }
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN => {
                dismiss_context_tooltip_impl();
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_KEYDOWN if wparam == VK_ESCAPE => {
                dismiss_context_tooltip_impl();
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_NCDESTROY => {
                dismiss_context_tooltip_impl();
                let _ = RemoveWindowSubclass(
                    hwnd,
                    Some(context_help_proc),
                    CONTEXT_HELP_SUBCLASS_ID,
                );
                if let Ok(mut bindings) = context_help_bindings().lock() {
                    let ptr = bindings.remove(&(hwnd as usize)).unwrap_or(ref_data);
                    if ptr != 0 {
                        drop(Box::from_raw(ptr as *mut ContextHelpData));
                    }
                } else if ref_data != 0 {
                    drop(Box::from_raw(ref_data as *mut ContextHelpData));
                }
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    /// Install or replace context help for a control.  Reusing this function is
    /// how a live language change updates already-created controls safely.
    pub fn install_context_help(hwnd: *mut c_void, text: &str, menu_label: &str) {
        if hwnd.is_null() || text.trim().is_empty() {
            return;
        }
        let binding = Box::into_raw(Box::new(ContextHelpData {
            body: text.to_owned(),
            menu_label: menu_label.to_owned(),
        }));
        unsafe {
            if SetWindowSubclass(
                hwnd as Hwnd,
                Some(context_help_proc),
                CONTEXT_HELP_SUBCLASS_ID,
                binding as usize,
            ) == 0
            {
                drop(Box::from_raw(binding));
                return;
            }

            if let Ok(mut bindings) = context_help_bindings().lock() {
                if let Some(old) = bindings.insert(hwnd as usize, binding as usize) {
                    if old != binding as usize {
                        drop(Box::from_raw(old as *mut ContextHelpData));
                    }
                }
            }
        }
    }

    unsafe extern "system" fn context_help_dismiss_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        _ref_data: usize,
    ) -> isize {
        match message {
            WM_LBUTTONDOWN
            | WM_RBUTTONDOWN
            | WM_MBUTTONDOWN
            | WM_XBUTTONDOWN
            | WM_NCLBUTTONDOWN
            | WM_NCRBUTTONDOWN
            | WM_NCMBUTTONDOWN
            | WM_NCXBUTTONDOWN => {
                dismiss_context_tooltip_impl();
            }
            WM_ACTIVATE if (wparam & 0xFFFF) == WA_INACTIVE => {
                // Clicking another application deactivates the Calculator; the
                // tracking tooltip should disappear just like classic WinHelp UI.
                dismiss_context_tooltip_impl();
            }
            WM_KEYDOWN if wparam == VK_ESCAPE => {
                dismiss_context_tooltip_impl();
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(
                    hwnd,
                    Some(context_help_dismiss_proc),
                    CONTEXT_HELP_DISMISS_SUBCLASS_ID,
                );
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }

    /// Extend tooltip dismissal to empty panel/chrome areas and non-client
    /// clicks, rather than only the control that originally opened the popup.
    pub fn install_context_help_dismissal(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let _ = SetWindowSubclass(
                hwnd as Hwnd,
                Some(context_help_dismiss_proc),
                CONTEXT_HELP_DISMISS_SUBCLASS_ID,
                0,
            );
        }
    }

    struct WindowStateBinding {
        callback: Box<dyn Fn(bool)>,
    }

    unsafe extern "system" fn window_state_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        ref_data: usize,
    ) -> isize {
        if message == WM_NCDESTROY {
            let _ = RemoveWindowSubclass(
                hwnd,
                Some(window_state_proc),
                WINDOW_STATE_SUBCLASS_ID,
            );
            if ref_data != 0 {
                drop(Box::from_raw(ref_data as *mut WindowStateBinding));
            }
            return DefSubclassProc(hwnd, message, wparam, lparam);
        }

        // Let wxWidgets process the native state transition first.  In
        // particular, on SIZE_RESTORED the Calculator's final outer rectangle
        // is then available to the companion-window positioning code.
        let result = DefSubclassProc(hwnd, message, wparam, lparam);
        if message == WM_SIZE && ref_data != 0 {
            let binding = &*(ref_data as *const WindowStateBinding);
            match wparam {
                SIZE_MINIMIZED => (binding.callback)(true),
                SIZE_RESTORED | SIZE_MAXIMIZED => (binding.callback)(false),
                _ => {}
            }
        }
        result
    }

    /// Observe native minimize/restore transitions for a top-level window.
    /// The callback receives true while entering the minimized state and false
    /// after the window has been restored/maximized.  This is deliberately
    /// separate from visibility: owned companion windows retain their logical
    /// open/hidden state while Windows temporarily suppresses them with the
    /// minimized owner.
    pub fn install_window_state_notifier(
        hwnd: *mut c_void,
        callback: Box<dyn Fn(bool)>,
    ) {
        if hwnd.is_null() {
            return;
        }
        let binding = Box::into_raw(Box::new(WindowStateBinding { callback }));
        unsafe {
            if SetWindowSubclass(
                hwnd as Hwnd,
                Some(window_state_proc),
                WINDOW_STATE_SUBCLASS_ID,
                binding as usize,
            ) == 0
            {
                drop(Box::from_raw(binding));
            }
        }
    }

    // --- Native selector notification -------------------------------------
    //
    // Win32 auto radio buttons and checkboxes (BS_AUTORADIOBUTTON /
    // BS_AUTOCHECKBOX) update their own check state and then post
    // WM_COMMAND/BN_CLICKED to their *parent*.  Watching the parent panel for
    // that notification is the authoritative way to learn that a selector was
    // operated: it does not depend on how the wx layer chooses to translate or
    // route the notification, and it works for mouse clicks, Space and the
    // arrow-key navigation Windows performs inside a radio group.
    //
    // The subclass is installed ahead of the existing window procedure and
    // always forwards through DefSubclassProc, so wxWidgets still sees every
    // message exactly as before.
    const WM_COMMAND: Uint = 0x0111;
    const BM_GETCHECK: Uint = 0x00F0;
    const BST_CHECKED: isize = 0x0001;
    const BN_CLICKED: u16 = 0;
    const SELECTOR_NOTIFY_SUBCLASS_ID: usize = 0xCA1C_9506;

    struct SelectorBinding {
        // Child HWNDs stored as usize so the table is a plain integer lookup.
        children: Vec<usize>,
        callback: Box<dyn Fn(usize)>,
    }

    unsafe extern "system" fn selector_notify_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        ref_data: usize,
    ) -> isize {
        match message {
            WM_COMMAND if ref_data != 0 && lparam != 0 => {
                let notification = ((wparam >> 16) & 0xFFFF) as u16;
                if notification == BN_CLICKED {
                    let binding = &*(ref_data as *const SelectorBinding);
                    let child = lparam as usize;
                    if let Some(index) = binding.children.iter().position(|h| *h == child) {
                        (binding.callback)(index);
                    }
                }
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(
                    hwnd,
                    Some(selector_notify_proc),
                    SELECTOR_NOTIFY_SUBCLASS_ID,
                );
                if ref_data != 0 {
                    drop(Box::from_raw(ref_data as *mut SelectorBinding));
                }
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    /// Stop this window from painting over siblings that sit above it.
    ///
    /// Windows dispatches WM_PAINT to overlapping children in top-to-bottom
    /// z-order, so a *lowered* window paints last.  draw_classic_group_box
    /// fills its whole client rectangle with COLOR_BTNFACE before stroking the
    /// etched frame, and that fill covers the radio buttons and checkboxes the
    /// frame is drawn around -- they only reappear when a click invalidates an
    /// individual control and it repaints itself.  WS_CLIPSIBLINGS removes the
    /// siblings above this window from its update region, so the frame paints
    /// around the selectors instead of over them.
    pub fn enable_clip_siblings(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let hwnd = hwnd as Hwnd;
            let style = GetWindowLongW(hwnd, GWL_STYLE);
            if style != 0 && (style & WS_CLIPSIBLINGS) == 0 {
                let _ = SetWindowLongW(hwnd, GWL_STYLE, style | WS_CLIPSIBLINGS);
            }
        }
    }

    /// Watch `parent` for BN_CLICKED from any of `children`, reporting the
    /// index within `children` to `callback`.
    pub fn install_selector_notifier(
        parent: *mut c_void,
        children: &[*mut c_void],
        callback: Box<dyn Fn(usize)>,
    ) {
        if parent.is_null() || children.is_empty() {
            return;
        }
        let binding = Box::into_raw(Box::new(SelectorBinding {
            children: children.iter().map(|h| *h as usize).collect(),
            callback,
        }));
        unsafe {
            if SetWindowSubclass(
                parent as Hwnd,
                Some(selector_notify_proc),
                SELECTOR_NOTIFY_SUBCLASS_ID,
                binding as usize,
            ) == 0
            {
                drop(Box::from_raw(binding));
            }
        }
    }

    /// Read a radio button's or checkbox's real check state from the control
    /// itself, rather than inferring it from a cached model value.
    pub fn is_button_checked(hwnd: *mut c_void) -> bool {
        if hwnd.is_null() {
            return false;
        }
        unsafe { SendMessageW(hwnd as Hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED }
    }

    unsafe fn select_cleartype_font(hwnd: Hwnd, hdc: Hdc) -> (Hgdiobj, Hgdiobj) {
        let base_font = SendMessageW(hwnd, WM_GETFONT, 0, 0) as Hgdiobj;
        if base_font.is_null() {
            return (null_mut(), null_mut());
        }

        // wxWidgets gives us the correct face/size/weight. Clone that LOGFONT
        // and request ClearType Natural explicitly for our coloured Win95
        // labels. This prevents the custom classic-edge painter from falling
        // back to the softer default GDI rasterization path.
        let mut logical = LogFontW::default();
        let copied = GetObjectW(
            base_font,
            std::mem::size_of::<LogFontW>() as i32,
            &mut logical as *mut LogFontW as *mut c_void,
        );
        if copied == 0 {
            return (SelectObject(hdc, base_font), null_mut());
        }
        logical.quality = CLEARTYPE_NATURAL_QUALITY;
        let smooth_font = CreateFontIndirectW(&logical);
        if smooth_font.is_null() {
            return (SelectObject(hdc, base_font), null_mut());
        }
        (SelectObject(hdc, smooth_font), smooth_font)
    }

    unsafe fn draw_control_text(
        hwnd: Hwnd,
        hdc: Hdc,
        rect: Rect,
        text_colour: u32,
        background_colour: u32,
        pressed_offset: i32,
    ) {
        let len = GetWindowTextLengthW(hwnd).max(0) as usize;
        let mut text = vec![0u16; len + 1];
        let copied = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32).max(0) as usize;

        let (old_font, smooth_font) = select_cleartype_font(hwnd, hdc);
        let old_mode = SetBkMode(hdc, OPAQUE);
        let old_background = SetBkColor(hdc, background_colour);
        let colour = if IsWindowEnabled(hwnd) != 0 {
            text_colour
        } else {
            GetSysColor(COLOR_GRAYTEXT)
        };
        let old_colour = SetTextColor(hdc, colour);

        let inset = button_metric(hwnd, 3);
        let mut text_rect = Rect {
            left: rect.left + inset,
            top: rect.top + inset,
            right: rect.right - inset,
            bottom: rect.bottom - inset,
        };
        if pressed_offset > 0 {
            text_rect.left += pressed_offset;
            text_rect.top += pressed_offset;
            text_rect.right += pressed_offset;
            text_rect.bottom += pressed_offset;
        }
        DrawTextW(
            hdc,
            text.as_ptr(),
            copied as i32,
            &mut text_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );

        SetTextColor(hdc, old_colour);
        SetBkColor(hdc, old_background);
        SetBkMode(hdc, old_mode);
        if !old_font.is_null() {
            SelectObject(hdc, old_font);
        }
        if !smooth_font.is_null() {
            DeleteObject(smooth_font);
        }
    }

    #[derive(Clone, Copy)]
    enum ClassicButtonColour {
        Face,
        Highlight,
        Light,
        Shadow,
        DarkShadow,
    }

    fn classic_button_colour_index(colour: ClassicButtonColour) -> i32 {
        match colour {
            // Preserve the active Windows button face.  Only the bevel geometry
            // is custom; the fill itself must not be replaced by a fixed Win95
            // grey on a modern desktop.
            ClassicButtonColour::Face => COLOR_BTNFACE,
            ClassicButtonColour::Highlight => COLOR_BTNHIGHLIGHT,
            ClassicButtonColour::Light => COLOR_3DLIGHT,
            ClassicButtonColour::Shadow => COLOR_BTNSHADOW,
            ClassicButtonColour::DarkShadow => COLOR_3DDKSHADOW,
        }
    }

    unsafe fn classic_button_colour(colour: ClassicButtonColour) -> u32 {
        GetSysColor(classic_button_colour_index(colour))
    }

    unsafe fn classic_button_brush(colour: ClassicButtonColour) -> Hbrush {
        GetSysColorBrush(classic_button_colour_index(colour))
    }

    unsafe fn fill_classic_rect(
        hdc: Hdc,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        colour: ClassicButtonColour,
    ) {
        if right <= left || bottom <= top {
            return;
        }
        let rect = Rect { left, top, right, bottom };
        FillRect(hdc, &rect, classic_button_brush(colour));
    }

    unsafe fn button_metric(hwnd: Hwnd, logical: i32) -> i32 {
        let dpi = GetDpiForWindow(hwnd).max(96);
        ((logical as i64 * dpi as i64 + 48) / 96).max(1) as i32
    }

    unsafe fn draw_raised_button(hwnd: Hwnd, hdc: Hdc, rect: Rect) -> Rect {
        let edge = button_metric(hwnd, 1);

        // Keep the current Windows button-face colour over the entire client.
        // The stronger relief comes solely from two DPI-scaled edge layers.
        fill_classic_rect(
            hdc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            ClassicButtonColour::Face,
        );

        // Outer bright/dark pair.
        fill_classic_rect(
            hdc,
            rect.left,
            rect.top,
            rect.right - edge,
            rect.top + edge,
            ClassicButtonColour::Highlight,
        );
        fill_classic_rect(
            hdc,
            rect.left,
            rect.top,
            rect.left + edge,
            rect.bottom - edge,
            ClassicButtonColour::Highlight,
        );
        fill_classic_rect(
            hdc,
            rect.right - edge,
            rect.top,
            rect.right,
            rect.bottom,
            ClassicButtonColour::DarkShadow,
        );
        fill_classic_rect(
            hdc,
            rect.left,
            rect.bottom - edge,
            rect.right,
            rect.bottom,
            ClassicButtonColour::DarkShadow,
        );

        // Inner light/shadow pair. At 200% DPI, each logical one-pixel band is
        // two physical pixels, so the complete bevel is four physical pixels
        // thick without changing the face colour or adding a black projection.
        fill_classic_rect(
            hdc,
            rect.left + edge,
            rect.top + edge,
            rect.right - edge * 2,
            rect.top + edge * 2,
            ClassicButtonColour::Light,
        );
        fill_classic_rect(
            hdc,
            rect.left + edge,
            rect.top + edge,
            rect.left + edge * 2,
            rect.bottom - edge * 2,
            ClassicButtonColour::Light,
        );
        fill_classic_rect(
            hdc,
            rect.right - edge * 2,
            rect.top + edge,
            rect.right - edge,
            rect.bottom - edge,
            ClassicButtonColour::Shadow,
        );
        fill_classic_rect(
            hdc,
            rect.left + edge,
            rect.bottom - edge * 2,
            rect.right - edge,
            rect.bottom - edge,
            ClassicButtonColour::Shadow,
        );

        rect
    }

    unsafe fn draw_pressed_button(hwnd: Hwnd, hdc: Hdc, rect: Rect) -> Rect {
        let edge = button_metric(hwnd, 1);

        fill_classic_rect(
            hdc,
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            ClassicButtonColour::Face,
        );

        // Invert the same two DPI-scaled layers for the pressed state.
        fill_classic_rect(
            hdc,
            rect.left,
            rect.top,
            rect.right - edge,
            rect.top + edge,
            ClassicButtonColour::DarkShadow,
        );
        fill_classic_rect(
            hdc,
            rect.left,
            rect.top,
            rect.left + edge,
            rect.bottom - edge,
            ClassicButtonColour::DarkShadow,
        );
        fill_classic_rect(
            hdc,
            rect.left + edge,
            rect.top + edge,
            rect.right - edge * 2,
            rect.top + edge * 2,
            ClassicButtonColour::Shadow,
        );
        fill_classic_rect(
            hdc,
            rect.left + edge,
            rect.top + edge,
            rect.left + edge * 2,
            rect.bottom - edge * 2,
            ClassicButtonColour::Shadow,
        );
        fill_classic_rect(
            hdc,
            rect.right - edge,
            rect.top,
            rect.right,
            rect.bottom,
            ClassicButtonColour::Highlight,
        );
        fill_classic_rect(
            hdc,
            rect.left,
            rect.bottom - edge,
            rect.right,
            rect.bottom,
            ClassicButtonColour::Highlight,
        );
        fill_classic_rect(
            hdc,
            rect.right - edge * 2,
            rect.top + edge,
            rect.right - edge,
            rect.bottom - edge,
            ClassicButtonColour::Light,
        );
        fill_classic_rect(
            hdc,
            rect.left + edge,
            rect.bottom - edge * 2,
            rect.right - edge,
            rect.bottom - edge,
            ClassicButtonColour::Light,
        );

        rect
    }

    unsafe fn draw_classic_button(hwnd: Hwnd, hdc: Hdc, text_colour: u32) {
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }

        let pushed = (SendMessageW(hwnd, BM_GETSTATE, 0, 0) as usize & BST_PUSHED) != 0;
        let text_rect = if pushed {
            draw_pressed_button(hwnd, hdc, rect)
        } else {
            draw_raised_button(hwnd, hdc, rect)
        };
        draw_control_text(
            hwnd,
            hdc,
            text_rect,
            text_colour,
            classic_button_colour(ClassicButtonColour::Face),
            if pushed { button_metric(hwnd, 1) } else { 0 },
        );
    }

    unsafe fn draw_classic_display(hwnd: Hwnd, hdc: Hdc) {
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }

        FillRect(hdc, &rect, GetSysColorBrush(COLOR_WINDOW));
        let mut edge_rect = rect;
        DrawEdge(hdc, &mut edge_rect, EDGE_SUNKEN, BF_RECT);

        let len = GetWindowTextLengthW(hwnd).max(0) as usize;
        let mut text = vec![0u16; len + 1];
        let copied = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32).max(0) as usize;
        let (old_font, smooth_font) = select_cleartype_font(hwnd, hdc);
        let old_mode = SetBkMode(hdc, TRANSPARENT);
        let colour = if IsWindowEnabled(hwnd) != 0 {
            GetSysColor(COLOR_WINDOWTEXT)
        } else {
            GetSysColor(COLOR_GRAYTEXT)
        };
        let old_colour = SetTextColor(hdc, colour);
        let mut text_rect = Rect {
            left: rect.left + 5,
            top: rect.top + 3,
            right: rect.right - 5,
            bottom: rect.bottom - 3,
        };
        DrawTextW(
            hdc,
            text.as_ptr(),
            copied as i32,
            &mut text_rect,
            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
        );
        SetTextColor(hdc, old_colour);
        SetBkMode(hdc, old_mode);
        if !old_font.is_null() {
            SelectObject(hdc, old_font);
        }
        if !smooth_font.is_null() {
            DeleteObject(smooth_font);
        }
    }

    unsafe fn draw_classic_sunken_field(hwnd: Hwnd, hdc: Hdc) {
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }
        FillRect(hdc, &rect, GetSysColorBrush(COLOR_BTNFACE));
        let mut edge_rect = rect;
        DrawEdge(hdc, &mut edge_rect, EDGE_SUNKEN, BF_RECT);
        draw_control_text(
            hwnd,
            hdc,
            rect,
            GetSysColor(COLOR_WINDOWTEXT),
            GetSysColor(COLOR_BTNFACE),
            0,
        );
    }

    unsafe fn draw_classic_separator(hwnd: Hwnd, hdc: Hdc) {
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }

        FillRect(hdc, &rect, GetSysColorBrush(COLOR_BTNFACE));
        // The original Scientific-mode separator is DrawEdge(EDGE_ETCHED,
        // BF_BOTTOM), recovered at CALC.EXE 0x00402804.
        let mut edge_rect = rect;
        DrawEdge(hdc, &mut edge_rect, EDGE_ETCHED, BF_BOTTOM);
    }

    unsafe extern "system" fn classic_separator_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        _ref_data: usize,
    ) -> isize {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    erase: 0,
                    paint: Rect::default(),
                    restore: 0,
                    inc_update: 0,
                    reserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                if !hdc.is_null() {
                    draw_classic_separator(hwnd, hdc);
                }
                EndPaint(hwnd, &paint);
                0
            }
            WM_PRINTCLIENT => {
                let hdc = wparam as Hdc;
                if !hdc.is_null() {
                    draw_classic_separator(hwnd, hdc);
                }
                0
            }
            WM_ERASEBKGND => 1,
            WM_NCHITTEST => HTTRANSPARENT,
            WM_NCDESTROY => {
                RemoveWindowSubclass(hwnd, Some(classic_separator_proc), CLASSIC_SEPARATOR_SUBCLASS_ID);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    pub fn install_classic_separator_painter(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let _ = SetWindowSubclass(
                hwnd,
                Some(classic_separator_proc),
                CLASSIC_SEPARATOR_SUBCLASS_ID,
                0,
            );
        }
    }

    unsafe fn draw_classic_vertical_separator(hwnd: Hwnd, hdc: Hdc) {
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }

        FillRect(hdc, &rect, GetSysColorBrush(COLOR_BTNFACE));
        // History is a true splitter pane, so the native sash remains the drag
        // target. This tiny child draws only the visible Win95-style boundary
        // immediately inside History without adding a gap between the panes.
        let mut edge_rect = rect;
        DrawEdge(hdc, &mut edge_rect, EDGE_ETCHED, BF_LEFT);
    }

    unsafe extern "system" fn classic_vertical_separator_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        _ref_data: usize,
    ) -> isize {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    erase: 0,
                    paint: Rect::default(),
                    restore: 0,
                    inc_update: 0,
                    reserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                if !hdc.is_null() {
                    draw_classic_vertical_separator(hwnd, hdc);
                }
                EndPaint(hwnd, &paint);
                0
            }
            WM_PRINTCLIENT => {
                let hdc = wparam as Hdc;
                if !hdc.is_null() {
                    draw_classic_vertical_separator(hwnd, hdc);
                }
                0
            }
            WM_ERASEBKGND => 1,
            WM_NCHITTEST => HTTRANSPARENT,
            WM_NCDESTROY => {
                RemoveWindowSubclass(
                    hwnd,
                    Some(classic_vertical_separator_proc),
                    CLASSIC_VERTICAL_SEPARATOR_SUBCLASS_ID,
                );
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    pub fn install_classic_vertical_separator_painter(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let _ = SetWindowSubclass(
                hwnd,
                Some(classic_vertical_separator_proc),
                CLASSIC_VERTICAL_SEPARATOR_SUBCLASS_ID,
                0,
            );
        }
    }

    unsafe fn draw_classic_group_box(hwnd: Hwnd, hdc: Hdc) {
        let mut rect = Rect::default();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }
        FillRect(hdc, &rect, GetSysColorBrush(COLOR_BTNFACE));
        // CALC.EXE's selector/status framing table uses EDGE_ETCHED with
        // BF_RECT (edge value 0x06, flags 0x0f).
        let mut edge_rect = rect;
        DrawEdge(hdc, &mut edge_rect, EDGE_ETCHED, BF_RECT);
    }

    unsafe extern "system" fn classic_group_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        _ref_data: usize,
    ) -> isize {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    erase: 0,
                    paint: Rect::default(),
                    restore: 0,
                    inc_update: 0,
                    reserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                if !hdc.is_null() {
                    draw_classic_group_box(hwnd, hdc);
                }
                EndPaint(hwnd, &paint);
                0
            }
            WM_PRINTCLIENT => {
                let hdc = wparam as Hdc;
                if !hdc.is_null() {
                    draw_classic_group_box(hwnd, hdc);
                }
                0
            }
            WM_ERASEBKGND => 1,
            WM_NCHITTEST => HTTRANSPARENT,
            WM_NCDESTROY => {
                RemoveWindowSubclass(hwnd, Some(classic_group_proc), CLASSIC_GROUP_SUBCLASS_ID);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    pub fn install_classic_group_box_painter(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let _ = SetWindowSubclass(
                hwnd,
                Some(classic_group_proc),
                CLASSIC_GROUP_SUBCLASS_ID,
                0,
            );
        }
    }

    unsafe extern "system" fn classic_button_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        ref_data: usize,
    ) -> isize {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    erase: 0,
                    paint: Rect::default(),
                    restore: 0,
                    inc_update: 0,
                    reserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                if !hdc.is_null() {
                    draw_classic_button(hwnd, hdc, ref_data as u32);
                }
                EndPaint(hwnd, &paint);
                0
            }
            WM_PRINTCLIENT => {
                let hdc = wparam as Hdc;
                if !hdc.is_null() {
                    draw_classic_button(hwnd, hdc, ref_data as u32);
                }
                0
            }
            WM_ERASEBKGND => 1,
            WM_TIMER if wparam == CLASSIC_BUTTON_KEY_TIMER_ID => {
                let _ = KillTimer(hwnd, CLASSIC_BUTTON_KEY_TIMER_ID);
                let _ = SendMessageW(hwnd, BM_SETSTATE, 0, 0);
                let _ = InvalidateRect(hwnd, null_mut(), 0);
                0
            }
            WM_NCDESTROY => {
                let _ = KillTimer(hwnd, CLASSIC_BUTTON_KEY_TIMER_ID);
                RemoveWindowSubclass(hwnd, Some(classic_button_proc), CLASSIC_BUTTON_SUBCLASS_ID);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    pub fn install_classic_button_painter(hwnd: *mut c_void, red: u8, green: u8, blue: u8) {
        if hwnd.is_null() {
            return;
        }
        let colour = colour_ref(red, green, blue) as usize;
        unsafe {
            let _ = SetWindowSubclass(
                hwnd,
                Some(classic_button_proc),
                CLASSIC_BUTTON_SUBCLASS_ID,
                colour,
            );
        }
    }

    /// Briefly depress a classic Calculator button in response to a keyboard
    /// accelerator. Re-arming the timer on key repeat keeps the face down while
    /// typing rapidly without blocking the UI thread.
    pub fn pulse_classic_button(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let hwnd = hwnd as Hwnd;
            let _ = KillTimer(hwnd, CLASSIC_BUTTON_KEY_TIMER_ID);
            let _ = SendMessageW(hwnd, BM_SETSTATE, 1, 0);
            let _ = InvalidateRect(hwnd, null_mut(), 0);
            let _ = SetTimer(
                hwnd,
                CLASSIC_BUTTON_KEY_TIMER_ID,
                CLASSIC_BUTTON_KEY_PRESS_MS,
                null_mut(),
            );
        }
    }

    /// True only when this exact child owns the thread's keyboard focus.
    /// Used to avoid stealing focus from the graph expression editor when the
    /// Calculator top-level window is reactivated.
    pub fn has_keyboard_focus(hwnd: *mut c_void) -> bool {
        !hwnd.is_null() && unsafe { GetFocus() == hwnd as Hwnd }
    }


    unsafe extern "system" fn classic_display_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        _ref_data: usize,
    ) -> isize {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    erase: 0,
                    paint: Rect::default(),
                    restore: 0,
                    inc_update: 0,
                    reserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                if !hdc.is_null() {
                    draw_classic_display(hwnd, hdc);
                    EndPaint(hwnd, &paint);
                    return 0;
                }
            }
            WM_PRINTCLIENT => {
                let hdc = wparam as Hdc;
                if !hdc.is_null() {
                    draw_classic_display(hwnd, hdc);
                    return 0;
                }
            }
            WM_NCDESTROY => {
                RemoveWindowSubclass(hwnd, Some(classic_display_proc), CLASSIC_FIELD_SUBCLASS_ID + 1);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, message, wparam, lparam)
    }

    pub fn install_classic_display_painter(hwnd: *mut c_void) {
        unsafe {
            if hwnd.is_null() {
                return;
            }
            SetWindowSubclass(
                hwnd as Hwnd,
                Some(classic_display_proc),
                CLASSIC_FIELD_SUBCLASS_ID + 1,
                0,
            );
            InvalidateRect(hwnd as Hwnd, null_mut(), 1);
        }
    }

    unsafe extern "system" fn classic_field_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        _ref_data: usize,
    ) -> isize {
        match message {
            WM_PAINT => {
                let mut paint = PaintStruct {
                    hdc: null_mut(),
                    erase: 0,
                    paint: Rect::default(),
                    restore: 0,
                    inc_update: 0,
                    reserved: [0; 32],
                };
                let hdc = BeginPaint(hwnd, &mut paint);
                if !hdc.is_null() {
                    draw_classic_sunken_field(hwnd, hdc);
                }
                EndPaint(hwnd, &paint);
                0
            }
            WM_PRINTCLIENT => {
                let hdc = wparam as Hdc;
                if !hdc.is_null() {
                    draw_classic_sunken_field(hwnd, hdc);
                }
                0
            }
            WM_ERASEBKGND => 1,
            WM_NCDESTROY => {
                RemoveWindowSubclass(hwnd, Some(classic_field_proc), CLASSIC_FIELD_SUBCLASS_ID);
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    pub fn install_classic_sunken_field_painter(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let _ = SetWindowSubclass(
                hwnd,
                Some(classic_field_proc),
                CLASSIC_FIELD_SUBCLASS_ID,
                0,
            );
        }
    }

    /// Resize the realized Calculator frame and its active panel using native
    /// physical pixels. wxWidgets may expose logical sizes through its public
    /// API while Windows has already DPI-scaled child HWND positions, so using
    /// wx-level `get_size()`/`set_client_size()` can leave the parent at half
    /// the required physical width on a 200% desktop.
    ///
    /// `logical_width`/`logical_height` are the already design-scaled 96-DPI
    /// client dimensions used by the wxDragon layout. We multiply them by the
    /// actual monitor DPI, then grow the top-level HWND by the exact difference
    /// between its current outer and client rectangles. A second pass absorbs
    /// any menu/non-client rounding after the first resize.
    pub fn fit_calculator_surface(
        frame_hwnd: *mut c_void,
        panel_hwnd: *mut c_void,
        logical_width: i32,
        logical_height: i32,
    ) -> bool {
        if frame_hwnd.is_null()
            || panel_hwnd.is_null()
            || logical_width <= 0
            || logical_height <= 0
        {
            return false;
        }

        unsafe {
            let frame = frame_hwnd as Hwnd;
            let panel = panel_hwnd as Hwnd;
            let dpi = GetDpiForWindow(frame).max(96);
            let desired_width =
                ((logical_width as i64 * dpi as i64 + 48) / 96).clamp(1, i32::MAX as i64) as i32;
            let desired_height =
                ((logical_height as i64 * dpi as i64 + 48) / 96).clamp(1, i32::MAX as i64) as i32;

            let resize_frame_client = |target_width: i32, target_height: i32| -> bool {
                let mut client = Rect::default();
                let mut outer = Rect::default();
                if GetClientRect(frame, &mut client) == 0 || GetWindowRect(frame, &mut outer) == 0 {
                    return false;
                }
                let client_width = (client.right - client.left).max(0);
                let client_height = (client.bottom - client.top).max(0);
                let outer_width = (outer.right - outer.left).max(1);
                let outer_height = (outer.bottom - outer.top).max(1);
                let target_outer_width =
                    (outer_width + target_width - client_width).max(1);
                let target_outer_height =
                    (outer_height + target_height - client_height).max(1);
                SetWindowPos(
                    frame,
                    null_mut(),
                    0,
                    0,
                    target_outer_width,
                    target_outer_height,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                ) != 0
            };

            if !resize_frame_client(desired_width, desired_height) {
                return false;
            }

            // Re-read the client rectangle after WM_NCCALCSIZE/menu metrics
            // have run. If Windows rounded the first request, correct the exact
            // remaining physical-pixel delta once more.
            let mut actual = Rect::default();
            if GetClientRect(frame, &mut actual) == 0 {
                return false;
            }
            let actual_width = actual.right - actual.left;
            let actual_height = actual.bottom - actual.top;
            if actual_width != desired_width || actual_height != desired_height {
                if !resize_frame_client(desired_width, desired_height) {
                    return false;
                }
            }

            // The panels have no sizer: make the active native panel occupy the
            // same physical client surface explicitly. This is the part that
            // buildfix6 never did, so its children remained larger than their
            // clipping parent even after the frame-size attempt.
            if SetWindowPos(
                panel,
                null_mut(),
                0,
                0,
                desired_width,
                desired_height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            ) == 0
            {
                return false;
            }

            true
        }
    }

    /// Center an already-realized top-level window inside the work area of the
    /// monitor nearest that window.  This runs after Calculator's DPI-aware
    /// client fitting, so centering uses the final native outer rectangle.
    pub fn center_window_on_work_area(hwnd: *mut c_void) -> bool {
        if hwnd.is_null() {
            return false;
        }

        unsafe {
            let hwnd = hwnd as Hwnd;
            let mut outer = Rect::default();
            if GetWindowRect(hwnd, &mut outer) == 0 {
                return false;
            }

            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
            if monitor.is_null() {
                return false;
            }
            let mut info = MonitorInfo {
                cb_size: std::mem::size_of::<MonitorInfo>() as Uint,
                ..MonitorInfo::default()
            };
            if GetMonitorInfoW(monitor, &mut info) == 0 {
                return false;
            }

            let width = (outer.right - outer.left).max(1);
            let height = (outer.bottom - outer.top).max(1);
            let work_width = (info.work.right - info.work.left).max(1);
            let work_height = (info.work.bottom - info.work.top).max(1);
            let x = info.work.left + (work_width - width) / 2;
            let y = info.work.top + (work_height - height) / 2;

            SetWindowPos(
                hwnd,
                null_mut(),
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            ) != 0
        }
    }

    /// Convert a mouse position in the native multiline Edit control into the
    /// UTF-16 character offset used by the rendered History-entry range table.
    /// EM_CHARFROMPOS accounts for wrapping/scrolling, while EM_POSFROMCHAR is
    /// used to reject clicks in the empty area below the final text line (where
    /// EM_CHARFROMPOS otherwise reports the nearest final character).
    pub fn history_text_position_from_point(
        text_hwnd: *mut c_void,
        x: i32,
        y: i32,
    ) -> Option<usize> {
        if text_hwnd.is_null() || x < 0 || y < 0 {
            return None;
        }

        let x_word = x.clamp(0, u16::MAX as i32) as u16 as u32;
        let y_word = y.clamp(0, u16::MAX as i32) as u16 as u32;
        let packed = (x_word | (y_word << 16)) as isize;
        let result = unsafe {
            SendMessageW(text_hwnd as Hwnd, EM_CHARFROMPOS, 0, packed)
        } as usize;
        let char_index = result & 0xFFFF;
        let line_index = (result >> 16) & 0xFFFF;
        if char_index == 0xFFFF || line_index == 0xFFFF {
            return None;
        }

        let line_start = unsafe {
            SendMessageW(text_hwnd as Hwnd, EM_LINEINDEX, line_index, 0)
        };
        if line_start < 0 {
            return None;
        }
        let line_pos = unsafe {
            SendMessageW(text_hwnd as Hwnd, EM_POSFROMCHAR, line_start as usize, 0)
        };
        if line_pos == -1 {
            return None;
        }
        let signed_hi = |value: isize| -> i32 {
            (((value as usize >> 16) & 0xFFFF) as u16 as i16) as i32
        };
        let line_top = signed_hi(line_pos);

        // Every History entry has an expression and result line, so there is a
        // neighbouring visual line from which we can obtain the native line
        // height even for the final entry. Wrapped lines are handled naturally.
        let next_start = unsafe {
            SendMessageW(text_hwnd as Hwnd, EM_LINEINDEX, line_index + 1, 0)
        };
        let neighbour_top = if next_start >= 0 {
            let pos = unsafe {
                SendMessageW(text_hwnd as Hwnd, EM_POSFROMCHAR, next_start as usize, 0)
            };
            if pos == -1 { None } else { Some(signed_hi(pos)) }
        } else if line_index > 0 {
            let prev_start = unsafe {
                SendMessageW(text_hwnd as Hwnd, EM_LINEINDEX, line_index - 1, 0)
            };
            if prev_start < 0 {
                None
            } else {
                let pos = unsafe {
                    SendMessageW(text_hwnd as Hwnd, EM_POSFROMCHAR, prev_start as usize, 0)
                };
                if pos == -1 { None } else { Some(signed_hi(pos)) }
            }
        } else {
            None
        };

        if let Some(neighbour_top) = neighbour_top {
            let line_height = (neighbour_top - line_top).abs().max(1);
            if y < line_top || y >= line_top.saturating_add(line_height) {
                return None;
            }
        }

        Some(char_index)
    }

    /// Center the titled Statistics Box over Calculator using their realized
    /// outer HWND rectangles.  The resulting position is clamped to the
    /// Calculator monitor's work area so the utility window never opens partly
    /// behind the taskbar or off-screen.
    pub fn position_statistics_companion(
        owner_hwnd: *mut c_void,
        stats_hwnd: *mut c_void,
    ) -> bool {
        if owner_hwnd.is_null() || stats_hwnd.is_null() {
            return false;
        }

        unsafe {
            let owner = owner_hwnd as Hwnd;
            let stats = stats_hwnd as Hwnd;
            let mut owner_outer = Rect::default();
            let mut stats_outer = Rect::default();
            if GetWindowRect(owner, &mut owner_outer) == 0
                || GetWindowRect(stats, &mut stats_outer) == 0
            {
                return false;
            }

            let width = (stats_outer.right - stats_outer.left).max(1);
            let height = (stats_outer.bottom - stats_outer.top).max(1);
            let owner_width = (owner_outer.right - owner_outer.left).max(1);
            let owner_height = (owner_outer.bottom - owner_outer.top).max(1);
            let mut x = owner_outer.left + (owner_width - width) / 2;
            let mut y = owner_outer.top + (owner_height - height) / 2;

            let monitor = MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST);
            if !monitor.is_null() {
                let mut info = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as Uint,
                    ..MonitorInfo::default()
                };
                if GetMonitorInfoW(monitor, &mut info) != 0 {
                    let max_x = info.work.right.saturating_sub(width);
                    let max_y = info.work.bottom.saturating_sub(height);
                    x = if width <= info.work.right.saturating_sub(info.work.left) {
                        x.clamp(info.work.left, max_x)
                    } else {
                        info.work.left
                    };
                    y = if height <= info.work.bottom.saturating_sub(info.work.top) {
                        y.clamp(info.work.top, max_y)
                    } else {
                        info.work.top
                    };
                }
            }

            SetWindowPos(
                stats,
                null_mut(),
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            ) != 0
        }
    }

    /// Return a realized HWND's client size in physical pixels.
    pub fn client_size_pixels(hwnd: *mut c_void) -> Option<(i32, i32)> {
        if hwnd.is_null() {
            return None;
        }
        unsafe {
            let mut rect = Rect::default();
            if GetClientRect(hwnd as Hwnd, &mut rect) == 0 {
                return None;
            }
            Some(((rect.right - rect.left).max(0), (rect.bottom - rect.top).max(0)))
        }
    }

    /// Move/resize one already-realized HWND using physical-pixel coordinates.
    pub fn set_window_rect_pixels(
        hwnd: *mut c_void,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) -> bool {
        if hwnd.is_null() || width <= 0 || height <= 0 {
            return false;
        }
        unsafe {
            SetWindowPos(
                hwnd as Hwnd,
                null_mut(),
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            ) != 0
        }
    }

    /// Scale a wx logical metric into the physical-pixel coordinate space used
    /// by classic/theming-disabled child controls on wxMSW.  Normal wxButton
    /// and wxTextCtrl children are DPI-realized by Windows in this application,
    /// while controls switched to the classic theme (radio/check/static-box)
    /// otherwise retain their logical coordinates.  Applying the monitor DPI
    /// here keeps both families in the same coordinate system.
    pub fn scale_classic_control_metric(hwnd: *mut c_void, logical: i32) -> i32 {
        if hwnd.is_null() {
            return logical;
        }
        unsafe {
            let dpi = GetDpiForWindow(hwnd as Hwnd).max(96);
            ((logical as i64 * dpi as i64 + 48) / 96)
                .clamp(i32::MIN as i64, i32::MAX as i64) as i32
        }
    }

    pub fn disable_frame_resizing(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            let hwnd = hwnd as Hwnd;
            let style = GetWindowLongW(hwnd, GWL_STYLE);
            let fixed = style & !WS_THICKFRAME & !WS_MAXIMIZEBOX;
            if fixed != style {
                let _ = SetWindowLongW(hwnd, GWL_STYLE, fixed);
                // Recalculate non-client metrics so the disabled sizing border and
                // maximize box disappear immediately without changing client size.
                let _ = SetWindowPos(
                    hwnd,
                    null_mut(),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED
                        | 0x0001, // SWP_NOSIZE
                );
            }
        }
    }

    pub fn enable_modern_dpi_awareness() {
        // PER_MONITOR_AWARE_V2. This must run before wxWidgets creates any
        // window so Windows never falls back to bitmap scaling the process.
        const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4;
        unsafe {
            let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        }
    }

    fn embedded_icon_image(width: u8) -> Option<&'static [u8]> {
        let ico = include_bytes!("../calc95.ico");
        if ico.len() < 6 || &ico[0..4] != [0, 0, 1, 0] {
            return None;
        }
        let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
        for index in 0..count {
            let entry = 6 + index * 16;
            if entry + 16 > ico.len() || ico[entry] != width {
                continue;
            }
            let bytes = u32::from_le_bytes(ico[entry + 8..entry + 12].try_into().ok()?) as usize;
            let offset = u32::from_le_bytes(ico[entry + 12..entry + 16].try_into().ok()?) as usize;
            let end = offset.checked_add(bytes)?;
            if end <= ico.len() {
                return Some(&ico[offset..end]);
            }
        }
        None
    }

    pub fn set_calculator_icon(hwnd: *mut c_void) {
        if hwnd.is_null() {
            return;
        }
        unsafe {
            for (width, slot) in [(16u8, ICON_SMALL), (32u8, ICON_BIG)] {
                let Some(image) = embedded_icon_image(width) else { continue; };
                let icon = CreateIconFromResourceEx(
                    image.as_ptr() as *mut u8,
                    image.len() as Uint,
                    1,
                    0x0003_0000,
                    width as i32,
                    width as i32,
                    LR_DEFAULTCOLOR,
                );
                if !icon.is_null() {
                    // Keep the icon alive for the process lifetime; the frame may
                    // continue to reference it after this message returns.
                    let _ = SendMessageW(hwnd, WM_SETICON, slot, icon as isize);
                }
            }
        }
    }

    unsafe fn keep_companion_above_owner(companion: Hwnd) {
        if companion.is_null() || IsWindowVisible(companion) == 0 {
            return;
        }
        // An owned top-level window must remain in front of its owner, but it
        // must not become globally topmost over unrelated applications.
        let _ = SetWindowPos(
            companion,
            null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }

    unsafe fn companion_has_activation(companion: Hwnd) -> bool {
        !companion.is_null()
            && IsWindowVisible(companion) != 0
            && (GetForegroundWindow() == companion || GetActiveWindow() == companion)
    }

    /// Mouse activation is sent to the clicked child HWND on some wxMSW paths
    /// rather than reliably reaching only the top-level frame. Therefore the
    /// same guard is installed on Calculator and all of its current descendants.
    /// Returning MA_NOACTIVATE keeps Statistics active while allowing the
    /// original click message to continue to the Calculator control.
    unsafe extern "system" fn companion_owner_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        ref_data: usize,
    ) -> isize {
        match message {
            WM_MOUSEACTIVATE => {
                let companion = ref_data as Hwnd;
                if companion_has_activation(companion) {
                    keep_companion_above_owner(companion);
                    return MA_NOACTIVATE;
                }
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            WM_NCDESTROY => {
                let _ = RemoveWindowSubclass(
                    hwnd,
                    Some(companion_owner_proc),
                    COMPANION_OWNER_SUBCLASS_ID,
                );
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    unsafe extern "system" fn install_companion_child_guard(hwnd: Hwnd, lparam: isize) -> Bool {
        let companion = lparam as Hwnd;
        let _ = SetWindowSubclass(
            hwnd,
            Some(companion_owner_proc),
            COMPANION_OWNER_SUBCLASS_ID,
            companion as usize,
        );
        1
    }

    unsafe extern "system" fn remove_companion_child_guard(hwnd: Hwnd, _lparam: isize) -> Bool {
        let _ = RemoveWindowSubclass(
            hwnd,
            Some(companion_owner_proc),
            COMPANION_OWNER_SUBCLASS_ID,
        );
        1
    }

    /// The companion subclass removes every Calculator-side mouse-activation
    /// guard when Statistics is destroyed. This prevents stale HWND references
    /// if the Statistics Box is closed and later rebuilt.
    unsafe extern "system" fn companion_active_proc(
        hwnd: Hwnd,
        message: Uint,
        wparam: usize,
        lparam: isize,
        _id: usize,
        ref_data: usize,
    ) -> isize {
        match message {
            WM_NCDESTROY => {
                let owner = ref_data as Hwnd;
                if !owner.is_null() {
                    let _ = RemoveWindowSubclass(
                        owner,
                        Some(companion_owner_proc),
                        COMPANION_OWNER_SUBCLASS_ID,
                    );
                    let _ = EnumChildWindows(owner, Some(remove_companion_child_guard), 0);
                }
                let _ = RemoveWindowSubclass(
                    hwnd,
                    Some(companion_active_proc),
                    COMPANION_ACTIVE_SUBCLASS_ID,
                );
                DefSubclassProc(hwnd, message, wparam, lparam)
            }
            _ => DefSubclassProc(hwnd, message, wparam, lparam),
        }
    }

    pub fn install_companion_activation_guard(
        owner_hwnd: *mut c_void,
        companion_hwnd: *mut c_void,
    ) {
        unsafe {
            if owner_hwnd.is_null() || companion_hwnd.is_null() {
                return;
            }
            let owner = owner_hwnd as Hwnd;
            let companion = companion_hwnd as Hwnd;

            // wxFrame parentage is not sufficient to guarantee a native owned
            // top-level relationship on every wxMSW build. Set it explicitly so
            // Windows itself keeps Statistics above Calculator in the z-order.
            let _ = SetWindowLongPtrW(companion, GWLP_HWNDPARENT, owner as isize);

            let _ = SetWindowSubclass(
                owner,
                Some(companion_owner_proc),
                COMPANION_OWNER_SUBCLASS_ID,
                companion as usize,
            );
            let _ = EnumChildWindows(
                owner,
                Some(install_companion_child_guard),
                companion as isize,
            );
            let _ = SetWindowSubclass(
                companion,
                Some(companion_active_proc),
                COMPANION_ACTIVE_SUBCLASS_ID,
                owner as usize,
            );
            keep_companion_above_owner(companion);
        }
    }

    /// Explicitly activate the Statistics utility after it is shown. A wxFrame
    /// SetFocus call alone can focus a child without making the top-level HWND
    /// the foreground/active window, which made the old WM_MOUSEACTIVATE guard
    /// ineffective because Windows still considered Calculator active.
    pub fn activate_statistics_companion(companion_hwnd: *mut c_void) {
        if companion_hwnd.is_null() {
            return;
        }
        unsafe {
            let companion = companion_hwnd as Hwnd;
            keep_companion_above_owner(companion);
            let _ = SetForegroundWindow(companion);
            let _ = SetActiveWindow(companion);
        }
    }

    /// Whenever Calculator or Statistics is synchronized, reaffirm only the
    /// owned-window z-order. This is deliberately not WS_EX_TOPMOST: Statistics
    /// stays above Calculator but does not float above unrelated applications.
    pub fn set_companion_application_active(companion_hwnd: *mut c_void, active: bool) {
        if !active || companion_hwnd.is_null() {
            return;
        }
        unsafe {
            keep_companion_above_owner(companion_hwnd as Hwnd);
        }
    }

    pub fn message(title: &str, body: &str) {
        let title = wide(title);
        let body = wide(body);
        unsafe {
            MessageBoxW(
                null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
    }

    pub fn copy_text(text: &str) -> Result<(), String> {
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return Err(CANNOT_OPEN_CLIPBOARD.into());
            }
            if EmptyClipboard() == 0 {
                CloseClipboard();
                return Err(CANNOT_OPEN_CLIPBOARD.into());
            }

            let wide_text = wide(text);
            let bytes = wide_text.len() * std::mem::size_of::<u16>();
            let memory = GlobalAlloc(GMEM_MOVEABLE, bytes);
            if memory.is_null() {
                CloseClipboard();
                return Err(NOT_ENOUGH_MEMORY_FOR_DATA.into());
            }

            let target = GlobalLock(memory) as *mut u16;
            if target.is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return Err(NOT_ENOUGH_MEMORY_FOR_DATA.into());
            }
            std::ptr::copy_nonoverlapping(wide_text.as_ptr(), target, wide_text.len());
            GlobalUnlock(memory);

            if SetClipboardData(CF_UNICODETEXT, memory).is_null() {
                GlobalFree(memory);
                CloseClipboard();
                return Err(NOT_ENOUGH_MEMORY_FOR_DATA.into());
            }
            // Ownership of memory transfers to the system after SetClipboardData succeeds.
            CloseClipboard();
        }
        Ok(())
    }

    pub fn paste_text() -> Result<Option<String>, String> {
        unsafe {
            if OpenClipboard(null_mut()) == 0 {
                return Err(CANNOT_OPEN_CLIPBOARD.into());
            }
            let memory = GetClipboardData(CF_UNICODETEXT);
            if memory.is_null() {
                CloseClipboard();
                return Ok(None);
            }
            let source = GlobalLock(memory) as *const u16;
            if source.is_null() {
                CloseClipboard();
                return Err(NOT_ENOUGH_MEMORY_FOR_DATA.into());
            }
            let mut len = 0usize;
            while *source.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(source, len));
            GlobalUnlock(memory);
            CloseClipboard();
            Ok(Some(text))
        }
    }
}

#[cfg(not(target_os = "windows"))]
mod windows {
    use std::ffi::c_void;

    #[cfg(target_os = "linux")]
    use std::ffi::{c_char, CString};
    #[cfg(target_os = "linux")]
    use std::ptr::null_mut;
    #[cfg(target_os = "linux")]
    use std::sync::OnceLock;

    #[cfg(target_os = "linux")]
    type GtkWidget = *mut c_void;
    #[cfg(target_os = "linux")]
    type GtkStyleContext = *mut c_void;
    #[cfg(target_os = "linux")]
    type GtkCssProvider = *mut c_void;
    #[cfg(target_os = "linux")]
    const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: u32 = 600;

    // wxDragon 0.9.17 uses wxWidgets' GTK3 backend on Linux. wxWindow::GetHandle()
    // exposes the underlying GtkWidget, so we can apply a tiny GTK3 CSS provider
    // directly to the native controls without adding a Rust GTK crate. The native
    // widgets continue to own labels, mouse/key state, accessibility and events;
    // only their chrome is replaced with the fixed Windows 95 palette/bevel order.
    #[cfg(target_os = "linux")]
    #[link(name = "gtk-3")]
    unsafe extern "C" {
        fn gtk_widget_get_style_context(widget: GtkWidget) -> GtkStyleContext;
        fn gtk_css_provider_new() -> GtkCssProvider;
        fn gtk_css_provider_load_from_data(
            provider: GtkCssProvider,
            data: *const c_char,
            length: isize,
            error: *mut *mut c_void,
        ) -> i32;
        fn gtk_style_context_add_provider(
            context: GtkStyleContext,
            provider: *mut c_void,
            priority: u32,
        );
        fn gtk_widget_set_name(widget: GtkWidget, name: *const c_char);
        fn gtk_widget_queue_draw(widget: GtkWidget);
        fn gtk_window_set_keep_above(window: GtkWidget, setting: i32);
    }

    #[cfg(target_os = "linux")]
    const CLASSIC_GTK_CSS: &str = r#"
#calc95_button_red,
#calc95_button_blue,
#calc95_button_navy,
#calc95_button_magenta,
#calc95_button_maroon,
#calc95_button_black {
    background-image: none;
    background-color: #c0c0c0;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #ffffff #404040 #404040 #ffffff;
    box-shadow:
        inset 1px 1px 0 0 #dfdfdf,
        inset -1px -1px 0 0 #808080,
        1px 1px 0 0 #808080,
        2px 2px 0 0 #000000;
    padding: 0;
}
#calc95_button_red:active,
#calc95_button_blue:active,
#calc95_button_navy:active,
#calc95_button_magenta:active,
#calc95_button_maroon:active,
#calc95_button_black:active {
    border-color: #000000 #dfdfdf #dfdfdf #000000;
    box-shadow: inset 1px 1px 0 0 #808080, inset -1px -1px 0 0 #ffffff;
    padding: 1px 0 0 1px;
}
#calc95_button_red { color: #ff0000; }
#calc95_button_blue { color: #0000ff; }
#calc95_button_navy { color: #000080; }
#calc95_button_magenta { color: #800080; }
#calc95_button_maroon { color: #800000; }
#calc95_button_black { color: #000000; }
#calc95_button_red:disabled,
#calc95_button_blue:disabled,
#calc95_button_navy:disabled,
#calc95_button_magenta:disabled,
#calc95_button_maroon:disabled,
#calc95_button_black:disabled { color: #808080; }

#calc95_display {
    background-image: none;
    background-color: #ffffff;
    color: #000000;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #000000 #ffffff #ffffff #000000;
    box-shadow: inset 1px 1px 0 0 #808080, inset -1px -1px 0 0 #dfdfdf;
    padding: 2px 4px;
}

#calc95_field {
    background-image: none;
    background-color: #c0c0c0;
    color: #000000;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #000000 #ffffff #ffffff #000000;
    box-shadow: inset 1px 1px 0 0 #808080, inset -1px -1px 0 0 #dfdfdf;
    padding: 0;
}

#calc95_group {
    background-image: none;
    background-color: transparent;
    border-radius: 0;
    border-style: solid;
    border-width: 1px;
    border-color: #808080 #ffffff #ffffff #808080;
    box-shadow: inset 1px 1px 0 0 #000000, inset -1px -1px 0 0 #dfdfdf;
    padding: 0;
}

#calc95_separator {
    background-image: none;
    background-color: transparent;
    border: 0;
    border-bottom: 1px solid #ffffff;
    box-shadow: inset 0 -1px 0 0 #808080;
    min-height: 2px;
    padding: 0;
}

#calc95_vertical_separator {
    background-image: none;
    background-color: transparent;
    border: 0;
    border-left: 1px solid #808080;
    box-shadow: inset 1px 0 0 0 #ffffff;
    min-width: 2px;
    padding: 0;
}
"#;

    #[cfg(target_os = "linux")]
    fn classic_provider() -> Option<GtkCssProvider> {
        static PROVIDER: OnceLock<usize> = OnceLock::new();
        let ptr = *PROVIDER.get_or_init(|| unsafe {
            let provider = gtk_css_provider_new();
            if provider.is_null() {
                return 0;
            }
            if let Ok(css) = CString::new(CLASSIC_GTK_CSS) {
                if gtk_css_provider_load_from_data(provider, css.as_ptr(), -1, null_mut()) == 0 {
                    return 0;
                }
            }
            provider as usize
        });
        if ptr == 0 { None } else { Some(ptr as GtkCssProvider) }
    }

    #[cfg(target_os = "linux")]
    fn apply_classic_name(hwnd: *mut c_void, name: &str) {
        unsafe {
            if hwnd.is_null() {
                return;
            }
            let Some(provider) = classic_provider() else { return; };
            let Ok(cname) = CString::new(name) else { return; };
            gtk_widget_set_name(hwnd as GtkWidget, cname.as_ptr());
            let context = gtk_widget_get_style_context(hwnd as GtkWidget);
            if !context.is_null() {
                gtk_style_context_add_provider(
                    context,
                    provider,
                    GTK_STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
            gtk_widget_queue_draw(hwnd as GtkWidget);
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn apply_classic_name(_hwnd: *mut c_void, _name: &str) {}

    /// Match the Windows active-companion policy without stealing keyboard
    /// focus. GTK/window managers cannot portably paint two top-level title bars
    /// as simultaneously active, so keep the Statistics utility above its owner
    /// while the Calculator application is active and remove the hint when it
    /// loses application focus.
    #[cfg(target_os = "linux")]
    pub fn set_companion_application_active(companion_hwnd: *mut c_void, active: bool) {
        unsafe {
            if companion_hwnd.is_null() {
                return;
            }
            gtk_window_set_keep_above(companion_hwnd as GtkWidget, if active { 1 } else { 0 });
        }
    }

    #[cfg(not(target_os = "linux"))]
    pub fn set_companion_application_active(
        _companion_hwnd: *mut c_void,
        _active: bool,
    ) {
    }

    pub fn install_companion_activation_guard(
        _owner_hwnd: *mut c_void,
        _companion_hwnd: *mut c_void,
    ) {
    }

    pub fn activate_statistics_companion(_companion_hwnd: *mut c_void) {}

    pub fn message(title: &str, body: &str) {
        eprintln!("{title}: {body}");
    }

    pub fn copy_text(_text: &str) -> Result<(), String> {
        Err("Clipboard integration is currently implemented for Windows only.".into())
    }

    pub fn paste_text() -> Result<Option<String>, String> {
        Err("Clipboard integration is currently implemented for Windows only.".into())
    }

    pub fn set_calculator_icon(_hwnd: *mut c_void) {}

    pub fn enable_modern_dpi_awareness() {}

    pub fn scale_classic_control_metric(_hwnd: *mut c_void, logical: i32) -> i32 {
        logical
    }

    pub fn disable_frame_resizing(_hwnd: *mut c_void) {}

    pub fn fit_calculator_surface(
        _frame_hwnd: *mut c_void,
        _panel_hwnd: *mut c_void,
        _logical_width: i32,
        _logical_height: i32,
    ) -> bool {
        false
    }

    pub fn center_window_on_work_area(_hwnd: *mut c_void) -> bool {
        false
    }

    pub fn history_text_position_from_point(
        _text_hwnd: *mut c_void,
        _x: i32,
        _y: i32,
    ) -> Option<usize> {
        None
    }

    pub fn position_statistics_companion(
        _owner_hwnd: *mut c_void,
        _stats_hwnd: *mut c_void,
    ) -> bool {
        false
    }

    pub fn client_size_pixels(_hwnd: *mut c_void) -> Option<(i32, i32)> {
        None
    }

    pub fn set_window_rect_pixels(
        _hwnd: *mut c_void,
        _x: i32,
        _y: i32,
        _width: i32,
        _height: i32,
    ) -> bool {
        false
    }

    pub fn install_classic_sunken_field_painter(hwnd: *mut c_void) {
        apply_classic_name(hwnd, "calc95_field");
    }

    pub fn install_classic_display_painter(hwnd: *mut c_void) {
        apply_classic_name(hwnd, "calc95_display");
    }

    pub fn install_classic_group_box_painter(hwnd: *mut c_void) {
        apply_classic_name(hwnd, "calc95_group");
    }

    pub fn install_classic_separator_painter(hwnd: *mut c_void) {
        apply_classic_name(hwnd, "calc95_separator");
    }

    pub fn install_classic_vertical_separator_painter(hwnd: *mut c_void) {
        apply_classic_name(hwnd, "calc95_vertical_separator");
    }

    pub fn install_classic_button_painter(
        hwnd: *mut c_void,
        red: u8,
        green: u8,
        blue: u8,
    ) {
        let style = match (red, green, blue) {
            (255, 0, 0) => "calc95_button_red",
            (0, 0, 255) => "calc95_button_blue",
            (0, 0, 128) => "calc95_button_navy",
            (128, 0, 128) => "calc95_button_magenta",
            (128, 0, 0) => "calc95_button_maroon",
            _ => "calc95_button_black",
        };
        apply_classic_name(hwnd, style);
    }

    pub fn pulse_classic_button(_hwnd: *mut c_void) {}

    pub fn has_keyboard_focus(_hwnd: *mut c_void) -> bool {
        false
    }

    pub fn enable_clip_siblings(_hwnd: *mut c_void) {}

    pub fn install_selector_notifier(
        _parent: *mut c_void,
        _children: &[*mut c_void],
        _callback: Box<dyn Fn(usize)>,
    ) {
    }

    pub fn is_button_checked(_hwnd: *mut c_void) -> bool {
        false
    }

    pub fn install_context_help(_hwnd: *mut c_void, _text: &str, _menu_label: &str) {}

    pub fn install_context_help_dismissal(_hwnd: *mut c_void) {}

    pub fn install_window_state_notifier(
        _hwnd: *mut c_void,
        _callback: Box<dyn Fn(bool)>,
    ) {
    }

    pub fn dismiss_context_tooltip() {}
}

pub use windows::{
    center_window_on_work_area, client_size_pixels, copy_text, disable_frame_resizing, dismiss_context_tooltip, enable_clip_siblings,
    enable_modern_dpi_awareness, fit_calculator_surface, history_text_position_from_point,
    install_classic_button_painter, install_classic_display_painter, install_classic_group_box_painter,
    install_classic_separator_painter, install_classic_sunken_field_painter,
    install_classic_vertical_separator_painter,
    activate_statistics_companion, install_companion_activation_guard, install_context_help, install_context_help_dismissal, install_selector_notifier,
    install_window_state_notifier,
    has_keyboard_focus, is_button_checked, message, paste_text, pulse_classic_button,
    position_statistics_companion, scale_classic_control_metric, set_calculator_icon,
    set_window_rect_pixels, set_companion_application_active,
};

pub fn launch_help(language: Language) -> Result<(), String> {
    let viewer = find_viewer().ok_or_else(viewer_missing_message)?;
    let help = find_calc_help(language).ok_or_else(|| {
        "The Help file for the selected language was not found. Keep the localized HLP/CNT files in the Help directory beside OpenCalc.".to_string()
    })?;

    Command::new(&viewer)
        .arg(&help)
        .spawn()
        // CALC.EXE routes a failed WinHelpA call to resource ID 74. The Rust
        // port uses an external HLP viewer, but once both files are resolved a
        // spawn failure is the closest equivalent to that original path.
        .map_err(|_| NOT_ENOUGH_MEMORY_FOR_DATA.to_string())?;
    Ok(())
}

fn viewer_missing_message() -> String {
    #[cfg(target_os = "windows")]
    {
        "hlp-viewer.exe was not found. Place it beside the Calculator executable.".to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "hlp-viewer was not found. Place the native executable beside OpenCalc.".to_string()
    }
}

fn find_viewer() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let mut candidates = Vec::new();
        if let Some(dir) = executable_dir() {
            candidates.push(dir.join("hlp-viewer.exe"));
        }
        if let Ok(dir) = std::env::current_dir() {
            candidates.push(dir.join("hlp-viewer.exe"));
        }
        return first_file(candidates);
    }

    #[cfg(target_os = "linux")]
    {
        // Linux must launch a native ELF companion, not the bundled Windows PE
        // executable. Keep discovery portable and deterministic by requiring
        // an extensionless `hlp-viewer` beside the running OpenCalc binary.
        return executable_dir()
            .map(|dir| dir.join("hlp-viewer"))
            .filter(|path| path.is_file());
    }

    #[cfg(all(not(target_os = "windows"), not(target_os = "linux")))]
    {
        let mut candidates = Vec::new();
        if let Some(dir) = executable_dir() {
            candidates.push(dir.join("hlp-viewer"));
        }
        if let Ok(dir) = std::env::current_dir() {
            candidates.push(dir.join("hlp-viewer"));
        }
        first_file(candidates)
    }
}

fn help_filenames(language: Language) -> &'static [&'static str] {
    match language {
        Language::English => &["CALC_EN.HLP", "calc_en.hlp"],
        Language::Portuguese => &[
            "CALC_PT-BR.HLP",
            "calc_pt-br.hlp",
            "CALC_EN.HLP",
            "calc_en.hlp",
        ],
        Language::Spanish => &[
            "CALC_ES.HLP",
            "calc_es.hlp",
            "CALC_EN.HLP",
            "calc_en.hlp",
        ],
    }
}

fn find_calc_help(language: Language) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(dir) = executable_dir() {
        let help = dir.join("Help");
        candidates.extend(help_filenames(language).iter().map(|name| help.join(name)));
    }
    if let Ok(dir) = std::env::current_dir() {
        let help = dir.join("Help");
        candidates.extend(help_filenames(language).iter().map(|name| help.join(name)));
    }
    #[cfg(target_os = "windows")]
    if let Some(windir) = std::env::var_os("WINDIR") {
        let help = PathBuf::from(windir).join("HELP");
        candidates.extend(help_filenames(language).iter().map(|name| help.join(name)));
    }
    first_file(candidates)
}

fn executable_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn first_file(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_file_ignores_missing_paths() {
        let missing = std::env::temp_dir().join("definitely-not-a-real-calc-help-file.hlp");
        assert_eq!(first_file(vec![missing]), None);
    }

    #[test]
    fn help_filenames_prefer_the_selected_language() {
        assert_eq!(help_filenames(Language::English)[0], "CALC_EN.HLP");
        assert_eq!(help_filenames(Language::Portuguese)[0], "CALC_PT-BR.HLP");
        assert_eq!(help_filenames(Language::Spanish)[0], "CALC_ES.HLP");
    }

    #[test]
    fn localized_help_can_fall_back_to_english() {
        assert!(help_filenames(Language::Portuguese).contains(&"CALC_EN.HLP"));
        assert!(help_filenames(Language::Spanish).contains(&"CALC_EN.HLP"));
    }

    #[test]
    fn viewer_error_names_the_platform_native_companion() {
        let message = viewer_missing_message();
        #[cfg(target_os = "windows")]
        assert!(message.contains("hlp-viewer.exe"));
        #[cfg(not(target_os = "windows"))]
        assert!(message.contains("hlp-viewer") && !message.contains(".exe"));
    }
}
