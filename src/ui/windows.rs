//! Windows-only wxMSW frontend policy.
//!
//! Shared calculator behavior and recovered geometry live in `mod.rs`.  This
//! module owns every wxMSW-specific choice: classic theming, decorator widgets,
//! selector notification bridging, modal messages, DPI fallback behavior, and
//! Statistics-window focus semantics.

use super::*;
use wxdragon::font::{FontFamily, FontStyle};

#[link(name = "uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(
        hwnd: *mut core::ffi::c_void,
        sub_app_name: *const u16,
        sub_id_list: *const u16,
    ) -> i32;
}

pub(super) fn apply_classic_theme(widget: &impl WxWidget) {
    let empty = [0u16];
    unsafe {
        let _ = SetWindowTheme(
            widget.get_handle() as *mut core::ffi::c_void,
            empty.as_ptr(),
            empty.as_ptr(),
        );
    }
}

pub(super) fn apply_surface(_widget: &impl WxWidget) {}

pub(super) fn classic_font(weight: FontWeight) -> Font {
    Font::new_with_details(
        9,
        FontFamily::Swiss.as_i32(),
        FontStyle::Normal.as_i32(),
        weight.as_i32(),
        false,
        "Microsoft Sans Serif",
    )
    .unwrap_or_else(Font::new)
}

pub(super) fn build_history_separator<W: WxWidget>(
    parent: &W,
    _panel: &Panel,
    height: i32,
) -> StaticText {
    StaticText::builder(parent)
        .with_label("")
        .with_pos(Point::new(0, 0))
        .with_size(Size::new(HISTORY_SEPARATOR_W, height))
        .build()
}

pub(super) fn style_history_text(_text: &TextCtrl) {}

pub(super) fn make_separator_line(parent: &Panel, y: i32, width: i32) {
    let line = StaticText::builder(parent)
        .with_label("")
        .with_pos(Point::new(0, dp(y)))
        .with_size(Size::new(dp(width), 2))
        .build();
    platform::install_classic_separator_painter(line.get_handle());
}

pub(super) fn make_group_box(parent: &Panel, x: i32, y: i32, width: i32, height: i32) {
    // A native empty StaticBox develops small edge overruns on current Windows
    // at high DPI.  Paint the complete etched frame in a wxStaticText instead.
    let group = StaticText::builder(parent)
        .with_label("")
        .with_pos(Point::new(dp(x), dp(y)))
        .with_size(Size::new(dp(width), dp(height)))
        .build();
    platform::install_classic_group_box_painter(group.get_handle());
    group.lower();
    platform::enable_clip_siblings(group.get_handle());
}

pub(super) fn install_selector_bridge(ui: &Rc<Ui>) {
    let mut children: Vec<_> = ui.base_radios.iter().map(|radio| radio.get_handle()).collect();
    children.extend(ui.angle_radios.iter().map(|radio| radio.get_handle()));
    children.push(ui.inv.get_handle());
    children.push(ui.hyp.get_handle());

    let ui_c = Rc::clone(ui);
    platform::install_selector_notifier(
        ui.scientific_panel.get_handle(),
        &children,
        Box::new(move |index| apply_selector(&ui_c, index)),
    );
}

fn apply_selector(ui: &Rc<Ui>, index: usize) {
    const BASES: [Base; 4] = [Base::Hex, Base::Dec, Base::Oct, Base::Bin];
    const ANGLES: [AngleMode; 3] = [
        AngleMode::Degrees,
        AngleMode::Radians,
        AngleMode::Grads,
    ];

    match index {
        0..=3 => select_base(ui, index, BASES[index]),
        4..=6 => select_angle(ui, index - 4, ANGLES[index - 4]),
        7 => {
            let checked = platform::is_button_checked(ui.inv.get_handle());
            mutate_calculator(ui, |calc| calc.inv = checked);
            refresh(ui);
        }
        8 => {
            let checked = platform::is_button_checked(ui.hyp.get_handle());
            mutate_calculator(ui, |calc| calc.hyp = checked);
            refresh(ui);
        }
        _ => {}
    }
}

pub(super) fn show_modal_message(
    _parent: &impl WxWidget,
    title: &str,
    body: &str,
) {
    platform::message(title, body);
}

pub(super) fn position_graph_panel(_panel: &Panel, _width: i32, _height: i32) {}

pub(super) fn position_splitter(
    _splitter: &SplitterWindow,
    _graph_width: i32,
    _splitter_width: i32,
    _height: i32,
) {
}

pub(super) fn splitter_style() -> SplitterWindowStyle {
    SplitterWindowStyle::Vertical
}

pub(super) fn history_separator_width() -> i32 {
    HISTORY_SEPARATOR_W
}

pub(super) fn history_separator_x(sash_position: i32, separator_width: i32) -> i32 {
    (sash_position - separator_width / 2).max(0)
}

pub(super) fn lock_frame_size(_frame: &Frame) {
    // wxMSW uses the removed ThickFrame/MaximizeBox styles.  Reintroducing
    // logical min/max hints here breaks physical sizing at high DPI.
}

pub(super) fn fit_frame_fallback<W: WxWidget>(
    frame: &Frame,
    surface: &W,
    logical_width: i32,
    logical_height: i32,
) {
    // This path is only a last resort if the native Win32 DPI helper fails.
    frame.set_client_size(Size::new(logical_width, logical_height));
    surface.set_size(Size::new(logical_width, logical_height));
}

pub(super) fn fit_frame_before_child_layout() -> bool {
    true
}

pub(super) fn graph_separator_width() -> i32 {
    GRAPH_SEPARATOR_W
}

pub(super) fn install_panel_resizing(_ui: &Rc<Ui>) {
    // Windows retains the established fixed Graph pane and native History sash.
}

pub(super) fn restore_main_keyboard_focus(ui: &Ui, active: bool) {
    if active && !platform::has_keyboard_focus(ui.graph_panel.expression.get_handle()) {
        // TranslateAcceleratorA in CALC.EXE was independent of child focus.
        // Preserve deliberate editing focus in the graph expression field.
        ui.frame.set_focus();
    }
}

pub(super) fn focus_statistics(_ui: &Ui, stats: &StatsBox) {
    platform::activate_statistics_companion(stats.frame.get_handle());
    stats.frame.set_focus();
}

pub(super) fn install_statistics_activation_hook(_frame: &Frame) {
    // The native owner-side WM_MOUSEACTIVATE guard preserves the original
    // companion focus behavior, so wx activation repaint hooks are unnecessary.
}
