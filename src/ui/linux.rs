//! Linux-only wxGTK frontend policy.
//!
//! Shared calculator behavior and recovered geometry live in `mod.rs`.  This
//! module owns the neutral RGB(240,240,240) surface, Linux font choice, GTK-safe sash
//! decoration, exact client resizing, draggable Linux panes, modal dialogs, and Statistics focus policy.

use super::*;
use wxdragon::dialogs::message_dialog::{MessageDialog, MessageDialogStyle};
use wxdragon::font::{FontFamily, FontStyle};
use wxdragon::widgets::staticbox::StaticBox;

pub(super) fn apply_classic_theme(_widget: &impl WxWidget) {}

pub(super) fn apply_surface(widget: &impl WxWidget) {
    widget.set_background_color(wxdragon::color::Colour::rgb(240, 240, 240));
}

pub(super) fn classic_font(weight: FontWeight) -> Font {
    Font::new_with_details(
        9,
        FontFamily::Swiss.as_i32(),
        FontStyle::Normal.as_i32(),
        weight.as_i32(),
        false,
        "Liberation Sans",
    )
    .unwrap_or_else(Font::new)
}

pub(super) fn build_history_separator<W: WxWidget>(
    parent: &W,
    _panel: &Panel,
    height: i32,
) -> StaticText {
    // wxGTK paints its native splitter sash with the desktop theme, producing
    // a warm grey strip beside the neutral OpenCalc surface. Cover it with the
    // same custom etched handle used by the Graph pane. The Linux resize policy
    // below owns dragging this overlay, so the native sash need not stay visible.
    StaticText::builder(parent)
        .with_label("")
        .with_pos(Point::new(0, 0))
        .with_size(Size::new(history_separator_width(), height))
        .build()
}

pub(super) fn style_history_text(text: &TextCtrl) {
    platform::install_classic_display_painter(text.get_handle());
}

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

pub(super) fn install_selector_bridge(_ui: &Rc<Ui>) {
    // wxGTK delivers the normal wx radio/checkbox events directly.
}

pub(super) fn show_modal_message(
    parent: &impl WxWidget,
    title: &str,
    body: &str,
) {
    let dialog = MessageDialog::builder(parent, body, title)
        .with_style(
            MessageDialogStyle::OK
                | MessageDialogStyle::IconInformation
                | MessageDialogStyle::Centre,
        )
        .build();
    let _ = dialog.show_modal();
}

pub(super) fn position_graph_panel(panel: &Panel, width: i32, height: i32) {
    // The native pixel helper is Win32-only.  wxGTK must receive the logical
    // child rectangle explicitly or Standard mode can leave Graph underneath
    // the Calculator/History splitter.
    panel.set_size_with_pos(0, 0, width, height);
}

pub(super) fn position_splitter(
    splitter: &SplitterWindow,
    graph_width: i32,
    splitter_width: i32,
    height: i32,
) {
    splitter.set_size_with_pos(graph_width, 0, splitter_width, height);
}

pub(super) fn splitter_style() -> SplitterWindowStyle {
    // Leave only the thinnest native sash underneath the custom Linux handle.
    // Windows retains its established native splitter style unchanged.
    SplitterWindowStyle::Vertical | SplitterWindowStyle::ThinSash
}

pub(super) fn history_separator_width() -> i32 {
    graph_separator_width()
}

pub(super) fn history_separator_x(sash_position: i32, _separator_width: i32) -> i32 {
    sash_position.max(0)
}

pub(super) fn lock_frame_size(_frame: &Frame) {
    // gtk_window_set_resizable(false), installed through platform::, blocks
    // user resizing without freezing later Standard/Scientific allocations.
}

pub(super) fn fit_frame_fallback<W: WxWidget>(
    frame: &Frame,
    surface: &W,
    logical_width: i32,
    logical_height: i32,
) {
    // GTK computes a top-level minimum from the currently allocated children.
    // Child panes are already resized before this Linux-only path runs. Apply
    // the exact request twice around a forced allocation so shrinking from
    // Scientific to Standard cannot retain the former child dimensions.
    surface.set_size(Size::new(logical_width, logical_height));
    surface.layout();
    frame.layout();
    frame.set_client_size(Size::new(logical_width, logical_height));
    surface.set_size(Size::new(logical_width, logical_height));
    surface.layout();
    frame.layout();
    frame.set_client_size(Size::new(logical_width, logical_height));
    frame.update();
}

pub(super) fn fit_frame_before_child_layout() -> bool {
    false
}

pub(super) fn graph_separator_width() -> i32 {
    // A two-pixel rule is visually accurate but too narrow to grab reliably on
    // wxGTK. Keep the etched line at the leading edge of a wider transparent
    // hit target so the Graph pane can be resized without changing Windows.
    dp(8)
}

pub(super) fn install_panel_resizing(ui: &Rc<Ui>) {
    // Linux exposes both optional-pane separators as captured drag handles.
    // Resize on release so the pointer delta remains relative to the original
    // separator even while the top-level GTK allocation changes.
    let drag = Rc::new(RefCell::new(None::<(i32, i32)>));

    {
        let separator = ui.graph_panel.separator.clone();
        let drag = Rc::clone(&drag);
        let ui_c = Rc::clone(ui);
        separator.on_mouse_left_down(move |event| {
            if ui_c.settings.borrow().graph_visible {
                if let WindowEventData::MouseButton(mouse) = &event {
                    if let Some(position) = mouse.get_position() {
                        *drag.borrow_mut() = Some((position.x, ui_c.graph_width.get()));
                        separator.capture_mouse();
                    }
                }
            }
            event.skip(false);
        });
    }

    {
        let separator = ui.graph_panel.separator.clone();
        let drag = Rc::clone(&drag);
        let ui_c = Rc::clone(ui);
        separator.on_mouse_left_up(move |event| {
            let start = drag.borrow_mut().take();
            if separator.has_capture() {
                separator.release_mouse();
            }
            if let Some((start_x, start_width)) = start {
                if let WindowEventData::MouseButton(mouse) = &event {
                    if let Some(position) = mouse.get_position() {
                        set_graph_width(&ui_c, start_width + position.x - start_x);
                    }
                }
            }
            event.skip(false);
        });
    }

    // The visible History separator covers wxGTK's themed native sash and owns
    // the Linux drag interaction. Moving it right narrows History; moving it
    // left widens History, while the Calculator surface remains fixed.
    let history_drag = Rc::new(RefCell::new(None::<(i32, i32)>));
    {
        let separator = ui.history_panel.separator.clone();
        let history_drag = Rc::clone(&history_drag);
        let ui_c = Rc::clone(ui);
        separator.on_mouse_left_down(move |event| {
            if ui_c.settings.borrow().history_visible {
                if let WindowEventData::MouseButton(mouse) = &event {
                    if let Some(position) = mouse.get_position() {
                        *history_drag.borrow_mut() = Some((
                            position.x,
                            ui_c.settings.borrow().history_width,
                        ));
                        separator.capture_mouse();
                    }
                }
            }
            event.skip(false);
        });
    }
    {
        let separator = ui.history_panel.separator.clone();
        let history_drag = Rc::clone(&history_drag);
        let ui_c = Rc::clone(ui);
        separator.on_mouse_left_up(move |event| {
            let start = history_drag.borrow_mut().take();
            if separator.has_capture() {
                separator.release_mouse();
            }
            if let Some((start_x, start_width)) = start {
                if let WindowEventData::MouseButton(mouse) = &event {
                    if let Some(position) = mouse.get_position() {
                        let width = (start_width - (position.x - start_x))
                            .clamp(MIN_HISTORY_WIDTH, MAX_HISTORY_WIDTH);
                        if ui_c.settings.borrow().history_width != width {
                            ui_c.settings.borrow_mut().history_width = width;
                            persist_settings(&ui_c);
                        }
                        let mode = ui_c.calc.borrow().mode;
                        sync_mode_surface(&ui_c, mode);
                    }
                }
            }
            event.skip(false);
        });
    }
}

pub(super) fn restore_main_keyboard_focus(_ui: &Ui, _active: bool) {
    // Linux already keeps Calculator as the keyboard sink while Statistics is
    // open; no additional activation rewrite is needed here.
}

pub(super) fn focus_statistics(ui: &Ui, stats: &StatsBox) {
    // wxGTK cannot route Calculator accelerators through a focused secondary
    // frame like CALC.EXE's Win32 message loop. Keep Calculator as the keyboard
    // sink, then retain only the Statistics stacking order without activating
    // it. The Linux backend arms an active-application above hint before the
    // window is shown, so Calculator button presses cannot make the box dip.
    ui.frame.raise();
    ui.frame.set_focus();
    platform::activate_statistics_companion(stats.frame.get_handle());
}

pub(super) fn install_statistics_activation_hook(frame: &Frame) {
    let stats_hwnd = frame.get_handle();
    frame.on_activate(move |event: WindowEventData| {
        if let WindowEventData::Activate(activation) = &event {
            // Both activation and deactivation enter the shared group policy.
            // Linux defers the inactive decision until the event turn settles,
            // so a Statistics -> Calculator click never drops the above hint,
            // while switching to another application still removes it.
            platform::set_companion_application_active(stats_hwnd, activation.is_active());
        }
        event.skip(true);
    });
}
