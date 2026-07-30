//! Portable fallback frontend policy for non-Windows, non-Linux targets.

use super::*;
use wxdragon::font::{FontFamily, FontStyle};
use wxdragon::widgets::staticbox::StaticBox;

pub(super) fn apply_classic_theme(_widget: &impl WxWidget) {}

pub(super) fn apply_surface(_widget: &impl WxWidget) {}

pub(super) fn classic_font(weight: FontWeight) -> Font {
    Font::new_with_details(
        9,
        FontFamily::Swiss.as_i32(),
        FontStyle::Normal.as_i32(),
        weight.as_i32(),
        false,
        "",
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

pub(super) fn style_graph_expression(_expression: &TextCtrl) {}

pub(super) fn make_separator_line(parent: &Panel, y: i32, width: i32) {
    let line = StaticBox::builder(parent)
        .with_label("")
        .with_pos(Point::new(0, dp(y)))
        .with_size(Size::new(dp(width), 2))
        .build();
    platform::install_classic_separator_painter(line.get_handle());
    line.lower();
}

pub(super) fn make_group_box(parent: &Panel, x: i32, y: i32, width: i32, height: i32) {
    let group = StaticBox::builder(parent)
        .with_label("")
        .with_pos(Point::new(dp(x), dp(y)))
        .with_size(Size::new(dp(width), dp(height)))
        .build();
    platform::install_classic_group_box_painter(group.get_handle());
    group.lower();
}

pub(super) fn install_selector_bridge(_ui: &Rc<Ui>) {}

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


pub(super) fn history_leading_gutter() -> i32 {
    0
}

pub(super) fn history_sash_extent() -> i32 {
    0
}

pub(super) fn history_uses_native_sash() -> bool {
    false
}

pub(super) fn history_separator_width() -> i32 {
    HISTORY_SEPARATOR_W
}

pub(super) fn history_separator_x(sash_position: i32, separator_width: i32) -> i32 {
    (sash_position - separator_width / 2).max(0)
}

pub(super) fn lock_frame_size(frame: &Frame) {
    let size = frame.get_size();
    frame.set_min_size(size);
    frame.set_max_size(size);
}

pub(super) fn fit_frame_fallback<W: WxWidget>(
    frame: &Frame,
    surface: &W,
    logical_width: i32,
    logical_height: i32,
) {
    frame.set_client_size(Size::new(logical_width, logical_height));
    surface.set_size(Size::new(logical_width, logical_height));
}

pub(super) fn fit_frame_before_child_layout() -> bool {
    true
}

pub(super) fn graph_separator_width() -> i32 {
    GRAPH_SEPARATOR_W
}

pub(super) fn install_panel_resizing(_ui: &Rc<Ui>) {}

pub(super) fn restore_main_keyboard_focus(_ui: &Ui, _active: bool) {}

pub(super) fn focus_statistics(_ui: &Ui, stats: &StatsBox) {
    stats.frame.set_focus();
}

pub(super) fn install_statistics_activation_hook(frame: &Frame) {
    let stats_hwnd = frame.get_handle();
    frame.on_activate(move |event: WindowEventData| {
        if let WindowEventData::Activate(activation) = &event {
            platform::set_companion_application_active(stats_hwnd, activation.is_active());
        }
        event.skip(true);
    });
}
