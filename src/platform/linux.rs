//! Linux-only GTK3 integration used by wxGTK.

use crate::errors::CANNOT_OPEN_CLIPBOARD;
use std::ffi::{c_char, c_void, CStr, CString};
use std::ptr::null_mut;
use std::sync::OnceLock;

type GtkWidget = *mut c_void;
type GtkStyleContext = *mut c_void;
type GtkCssProvider = *mut c_void;
type GtkClipboard = *mut c_void;
type GdkDisplay = *mut c_void;
type GdkWindow = *mut c_void;
const GTK_STYLE_PROVIDER_PRIORITY_APPLICATION: u32 = 600;

// wxDragon 0.9.17 uses wxWidgets' GTK3 backend on Linux. wxWindow::GetHandle()
// exposes the underlying GtkWidget, so we can apply a tiny GTK3 CSS provider
// directly to the native controls without adding a Rust GTK crate. The native
// widgets continue to own labels, mouse/key state, accessibility and events;
// only their chrome is replaced with the fixed Windows 95 palette/bevel order.
#[link(name = "gtk-3")]
unsafe extern "C" {
    fn gtk_widget_get_style_context(widget: GtkWidget) -> GtkStyleContext;
    fn gtk_widget_has_focus(widget: GtkWidget) -> i32;
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
    fn gtk_widget_set_sensitive(widget: GtkWidget, sensitive: i32);
    fn gtk_widget_queue_draw(widget: GtkWidget);
    fn gtk_widget_get_window(widget: GtkWidget) -> GdkWindow;
    fn gtk_widget_get_realized(widget: GtkWidget) -> i32;
    fn gtk_widget_in_destruction(widget: GtkWidget) -> i32;
    fn gtk_window_set_transient_for(window: GtkWidget, parent: GtkWidget);
    fn gtk_window_set_skip_taskbar_hint(window: GtkWidget, setting: i32);
    fn gtk_window_set_keep_above(window: GtkWidget, setting: i32);
    fn gtk_window_get_transient_for(window: GtkWidget) -> GtkWidget;
    fn gtk_window_is_active(window: GtkWidget) -> i32;
    fn gtk_window_set_resizable(window: GtkWidget, setting: i32);
    fn gtk_clipboard_get_default(display: GdkDisplay) -> GtkClipboard;
    fn gtk_clipboard_set_text(clipboard: GtkClipboard, text: *const c_char, length: i32);
    fn gtk_clipboard_store(clipboard: GtkClipboard);
    fn gtk_clipboard_wait_for_text(clipboard: GtkClipboard) -> *mut c_char;
    fn gtk_editable_get_chars(editable: GtkWidget, start_pos: i32, end_pos: i32) -> *mut c_char;
    fn gtk_editable_get_position(editable: GtkWidget) -> i32;
    fn gtk_editable_get_selection_bounds(
        editable: GtkWidget,
        start_pos: *mut i32,
        end_pos: *mut i32,
    ) -> i32;
    fn gtk_editable_delete_text(editable: GtkWidget, start_pos: i32, end_pos: i32);
    fn gtk_editable_insert_text(
        editable: GtkWidget,
        new_text: *const c_char,
        new_text_length: i32,
        position: *mut i32,
    );
    fn gtk_editable_set_position(editable: GtkWidget, position: i32);
}

#[link(name = "gdk-3")]
unsafe extern "C" {
    fn gdk_display_get_default() -> GdkDisplay;
    fn gdk_window_raise(window: GdkWindow);
}

#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_free(memory: *mut c_void);
    fn g_idle_add_full(
        priority: i32,
        function: Option<unsafe extern "C" fn(*mut c_void) -> i32>,
        data: *mut c_void,
        notify: Option<unsafe extern "C" fn(*mut c_void)>,
    ) -> u32;
}

#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_object_ref(object: *mut c_void) -> *mut c_void;
    fn g_object_unref(object: *mut c_void);
}

const G_PRIORITY_DEFAULT_IDLE: i32 = 200;
const G_SOURCE_REMOVE: i32 = 0;

const CLASSIC_GTK_CSS: &str = r#"
#calc95_button_red,
#calc95_button_blue,
#calc95_button_navy,
#calc95_button_magenta,
#calc95_button_maroon,
#calc95_button_black {
background-image: none;
background-color: #f0f0f0;
border-radius: 0;
border-style: solid;
border-width: 1px;
border-color: #ffffff #404040 #404040 #ffffff;
box-shadow:
    inset 1px 1px 0 0 #f0f0f0,
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
border-color: #000000 #f0f0f0 #f0f0f0 #000000;
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
box-shadow: inset 1px 1px 0 0 #808080, inset -1px -1px 0 0 #f0f0f0;
padding: 2px 4px;
}

#calc95_field {
background-image: none;
background-color: #f0f0f0;
color: #000000;
border-radius: 0;
border-style: solid;
border-width: 1px;
border-color: #000000 #ffffff #ffffff #000000;
box-shadow: inset 1px 1px 0 0 #808080, inset -1px -1px 0 0 #f0f0f0;
padding: 0;
}

#calc95_group {
background-image: none;
background-color: transparent;
border-radius: 0;
border-style: solid;
border-width: 1px;
border-color: #808080 #ffffff #ffffff #808080;
box-shadow: none;
padding: 0;
}

#calc95_splitter {
background-image: none;
background-color: #f0f0f0;
border: 0;
box-shadow: none;
}
#calc95_splitter > separator {
background-color: #f0f0f0;
background-image: linear-gradient(
    to right,
    #f0f0f0 0,
    #f0f0f0 3px,
    #808080 3px,
    #808080 4px,
    #ffffff 4px,
    #ffffff 5px,
    #f0f0f0 5px,
    #f0f0f0 100%
);
border: 0;
box-shadow: none;
min-width: 8px;
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
background-color: #f0f0f0;
border: 0;
border-left: 1px solid #808080;
box-shadow: inset 1px 0 0 0 #ffffff;
min-width: 2px;
opacity: 1;
padding: 0;
}
"#;

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

unsafe fn raise_companion_without_activation(companion: GtkWidget) {
    if companion.is_null() {
        return;
    }

    // gdk_window_raise changes only stacking order. Unlike gtk_window_present,
    // it does not request keyboard focus, so Calculator can continue receiving
    // key accelerators while Statistics remains visibly above its owner.
    let native_window = gtk_widget_get_window(companion);
    if !native_window.is_null() {
        gdk_window_raise(native_window);
    }
}

unsafe fn companion_group_is_active(companion: GtkWidget) -> bool {
    if companion.is_null()
        || gtk_widget_in_destruction(companion) != 0
        || gtk_widget_get_realized(companion) == 0
    {
        return false;
    }
    if gtk_window_is_active(companion) != 0 {
        return true;
    }
    let owner = gtk_window_get_transient_for(companion);
    !owner.is_null() && gtk_window_is_active(owner) != 0
}

unsafe extern "C" fn release_retained_widget(data: *mut c_void) {
    if !data.is_null() {
        g_object_unref(data);
    }
}

unsafe extern "C" fn settle_companion_above_state(data: *mut c_void) -> i32 {
    let companion = data as GtkWidget;
    if companion.is_null()
        || gtk_widget_in_destruction(companion) != 0
        || gtk_widget_get_realized(companion) == 0
    {
        return G_SOURCE_REMOVE;
    }
    let active = companion_group_is_active(companion);
    gtk_window_set_keep_above(companion, i32::from(active));
    if active {
        raise_companion_without_activation(companion);
    }
    G_SOURCE_REMOVE
}

unsafe fn defer_companion_above_state(companion: GtkWidget) {
    // A Statistics deactivate event can be delivered after the owner has
    // already been raised but before GTK reports Calculator as active.  Keep
    // the existing above hint through that event turn, then inspect both
    // members of the transient group at idle time.  Retaining the GtkWindow
    // makes closing Statistics before the idle callback safe.
    let retained = g_object_ref(companion as *mut c_void);
    if retained.is_null() {
        return;
    }
    let _ = g_idle_add_full(
        G_PRIORITY_DEFAULT_IDLE,
        Some(settle_companion_above_state),
        retained,
        Some(release_retained_widget),
    );
}

/// Keep Statistics above Calculator for the whole period in which either
/// OpenCalc window is active. The WM-level above hint is armed before any
/// Calculator control receives a click, so the owner cannot briefly jump over
/// the utility and then be corrected one activation event later. Deactivation
/// is settled after the current GTK event turn, which distinguishes a focus
/// transfer inside OpenCalc from a switch to another application.
pub fn set_companion_application_active(companion_hwnd: *mut c_void, active: bool) {
    if companion_hwnd.is_null() {
        return;
    }
    unsafe {
        let companion = companion_hwnd as GtkWidget;
        if active {
            gtk_window_set_keep_above(companion, 1);
            raise_companion_without_activation(companion);
        } else {
            defer_companion_above_state(companion);
        }
    }
}

/// wxFrame parentage does not consistently become a native GTK transient
/// relationship under every wxGTK/window-manager combination. Set it
/// explicitly and arm the active-group above hint before the window is shown,
/// so clicking any Calculator control cannot produce even a one-frame dip.
/// The utility is also kept out of the task switcher, matching an owned Windows
/// companion rather than an independent application window.
pub fn install_companion_activation_guard(
    owner_hwnd: *mut c_void,
    companion_hwnd: *mut c_void,
) {
    unsafe {
        if owner_hwnd.is_null() || companion_hwnd.is_null() {
            return;
        }

        let owner = owner_hwnd as GtkWidget;
        let companion = companion_hwnd as GtkWidget;
        gtk_window_set_transient_for(companion, owner);
        gtk_window_set_skip_taskbar_hint(companion, 1);
        gtk_window_set_keep_above(companion, 1);
        raise_companion_without_activation(companion);
    }
}

pub fn activate_statistics_companion(companion_hwnd: *mut c_void) {
    if companion_hwnd.is_null() {
        return;
    }
    unsafe {
        let companion = companion_hwnd as GtkWidget;
        gtk_window_set_keep_above(companion, 1);
        raise_companion_without_activation(companion);
    }
}

pub fn message(title: &str, body: &str) {
    eprintln!("{title}: {body}");
}

fn default_clipboard() -> Result<GtkClipboard, String> {
    unsafe {
        let display = gdk_display_get_default();
        if display.is_null() {
            return Err(CANNOT_OPEN_CLIPBOARD.into());
        }

        let clipboard = gtk_clipboard_get_default(display);
        if clipboard.is_null() {
            return Err(CANNOT_OPEN_CLIPBOARD.into());
        }

        Ok(clipboard)
    }
}

pub fn copy_text(text: &str) -> Result<(), String> {
    let clipboard = default_clipboard()?;
    let text = CString::new(text).map_err(|_| CANNOT_OPEN_CLIPBOARD.to_string())?;

    unsafe {
        // GTK copies the UTF-8 bytes before this function returns, so the
        // temporary CString does not need to outlive the call.  Store asks the
        // desktop clipboard manager to retain the value after OpenCalc exits.
        gtk_clipboard_set_text(clipboard, text.as_ptr(), -1);
        gtk_clipboard_store(clipboard);
    }

    Ok(())
}

pub fn paste_text() -> Result<Option<String>, String> {
    let clipboard = default_clipboard()?;

    unsafe {
        let raw = gtk_clipboard_wait_for_text(clipboard);
        if raw.is_null() {
            return Ok(None);
        }

        // gtk_clipboard_wait_for_text returns newly allocated GLib memory.
        // Convert it before releasing the buffer with g_free.
        let text = CStr::from_ptr(raw).to_string_lossy().into_owned();
        g_free(raw.cast());
        Ok(Some(text))
    }
}

pub fn set_calculator_icon(_hwnd: *mut c_void) {}

pub fn enable_modern_dpi_awareness() {}

pub fn scale_classic_control_metric(_hwnd: *mut c_void, logical: i32) -> i32 {
    logical
}

pub fn enable_frame_resizing(hwnd: *mut c_void) {
    unsafe {
        if !hwnd.is_null() {
            // Temporarily release GTK's user-resize lock while OpenCalc applies
            // an exact programmatic client size for a mode/pane transition.
            gtk_window_set_resizable(hwnd as GtkWidget, 1);
        }
    }
}

pub fn disable_frame_resizing(hwnd: *mut c_void) {
    unsafe {
        if !hwnd.is_null() {
            // Unlike equal GTK min/max geometry hints, this prevents only
            // user resizing. Programmatic SetClientSize calls remain able
            // to switch between Standard/Scientific dimensions and to add
            // or remove the Graph/History panes.
            gtk_window_set_resizable(hwnd as GtkWidget, 0);
        }
    }
}

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

pub fn install_classic_splitter_painter(hwnd: *mut c_void) {
    apply_classic_name(hwnd, "calc95_splitter");
}

pub fn install_classic_separator_painter(hwnd: *mut c_void) {
    apply_classic_name(hwnd, "calc95_separator");
}

pub fn install_classic_vertical_separator_painter(hwnd: *mut c_void) {
    apply_classic_name(hwnd, "calc95_vertical_separator");
}


pub fn make_pointer_passthrough(hwnd: *mut c_void) {
    unsafe {
        if !hwnd.is_null() {
            // An insensitive GTK child is skipped as an input target and the
            // pointer event continues to the wxSplitterWindow underneath. CSS
            // fixes opacity at 1, so the neutral etched decoration is unchanged.
            gtk_widget_set_sensitive(hwnd as GtkWidget, 0);
        }
    }
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

pub fn has_keyboard_focus(hwnd: *mut c_void) -> bool {
    !hwnd.is_null() && unsafe { gtk_widget_has_focus(hwnd as GtkWidget) != 0 }
}

/// The top-level Edit accelerators defer to the native Function editor
/// whenever that GtkEditable owns keyboard focus.
pub fn editable_owns_clipboard(hwnd: *mut c_void) -> bool {
    has_keyboard_focus(hwnd)
}

/// Return the current UTF-8 selection from a native single-line wxTextCtrl.
pub fn selected_text(hwnd: *mut c_void) -> Option<String> {
    if hwnd.is_null() {
        return None;
    }

    unsafe {
        let editable = hwnd as GtkWidget;
        let mut start = 0;
        let mut end = 0;
        if gtk_editable_get_selection_bounds(editable, &mut start, &mut end) == 0
            || start == end
        {
            return None;
        }
        let raw = gtk_editable_get_chars(editable, start, end);
        if raw.is_null() {
            return None;
        }
        let text = CStr::from_ptr(raw).to_string_lossy().into_owned();
        g_free(raw.cast());
        Some(text)
    }
}

/// Insert UTF-8 text into a native single-line wxTextCtrl, replacing the
/// current selection and preserving its editing focus/caret. wxGTK implements
/// TextCtrl with GtkEntry, which exposes the GtkEditable interface used here.
pub fn insert_text_at_selection(hwnd: *mut c_void, text: &str) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let Ok(text) = CString::new(text) else {
        return false;
    };

    unsafe {
        let editable = hwnd as GtkWidget;
        let mut start = 0;
        let mut end = 0;
        let mut position = if gtk_editable_get_selection_bounds(
            editable,
            &mut start,
            &mut end,
        ) != 0
        {
            gtk_editable_delete_text(editable, start, end);
            start
        } else {
            gtk_editable_get_position(editable)
        };

        gtk_editable_insert_text(editable, text.as_ptr(), -1, &mut position);
        gtk_editable_set_position(editable, position);
    }
    true
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

pub(super) fn viewer_missing_message() -> &'static str {
    "hlp-viewer was not found. Place the native executable beside OpenCalc."
}

pub(super) fn find_viewer() -> Option<std::path::PathBuf> {
    super::executable_dir()
        .map(|dir| dir.join("hlp-viewer"))
        .filter(|path| is_executable_file(path))
}

fn is_executable_file(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

pub(super) fn append_system_help_candidates(
    _candidates: &mut Vec<std::path::PathBuf>,
    _filenames: &[&str],
) {
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn classic_css_uses_the_requested_neutral_face() {
        assert!(CLASSIC_GTK_CSS.contains("background-color: #f0f0f0;"));
        let old_face = ["#d4", "d0c8"].concat();
        let old_highlight = ["#ec", "e9d8"].concat();
        assert!(!CLASSIC_GTK_CSS.contains(&old_face));
        assert!(!CLASSIC_GTK_CSS.contains(&old_highlight));
    }

    #[test]
    fn group_boxes_use_one_even_border_without_a_second_dark_top_band() {
        let group = CLASSIC_GTK_CSS
            .split("#calc95_group {")
            .nth(1)
            .and_then(|tail| tail.split('}').next())
            .expect("group CSS block");
        assert!(group.contains("border-width: 1px;"));
        assert!(group.contains("box-shadow: none;"));
        assert!(!group.contains("#000000"));
    }

    #[test]
    fn splitter_chrome_uses_the_neutral_linux_surface() {
        let splitter = CLASSIC_GTK_CSS
            .split("#calc95_splitter {")
            .nth(1)
            .and_then(|tail| tail.split('}').next())
            .expect("splitter CSS block");
        assert!(splitter.contains("background-color: #f0f0f0;"));
        assert!(splitter.contains("box-shadow: none;"));

        let sash = CLASSIC_GTK_CSS
            .split("#calc95_splitter > separator {")
            .nth(1)
            .and_then(|tail| tail.split('}').next())
            .expect("native sash CSS block");
        assert!(sash.contains("background-color: #f0f0f0;"));
        assert!(sash.contains("linear-gradient"));
        assert!(sash.contains("min-width: 8px;"));
        assert!(sash.contains("box-shadow: none;"));

        let decoration = CLASSIC_GTK_CSS
            .split("#calc95_vertical_separator {")
            .nth(1)
            .and_then(|tail| tail.split('}').next())
            .expect("separator decoration CSS block");
        assert!(decoration.contains("opacity: 1;"));
    }

    #[test]
    fn viewer_must_have_an_executable_bit() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("opencalc-viewer-{suffix}"));
        fs::write(&path, b"viewer").expect("write test viewer");

        let mut permissions = fs::metadata(&path).expect("viewer metadata").permissions();
        permissions.set_mode(0o644);
        fs::set_permissions(&path, permissions).expect("set non-executable mode");
        assert!(!is_executable_file(&path));

        let mut permissions = fs::metadata(&path).expect("viewer metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&path, permissions).expect("set executable mode");
        assert!(is_executable_file(&path));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn viewer_error_names_native_companion() {
        let message = viewer_missing_message();
        assert!(message.contains("hlp-viewer") && !message.contains(".exe"));
    }
}
