//! wxDragon front end for the Windows 95 Calculator reimplementation.
//!
//! The previous proof-of-concept painted its own title bar, menus and button
//! bevels inside a WS_POPUP window.  That made the client area much taller
//! than the real calculator and prevented native menu/control metrics from
//! matching the reference.  This module keeps the recovered Calculator child
//! control coordinates but lets wxWidgets own the frame, menu bar and controls.

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod frontend;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod frontend;
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
#[path = "other.rs"]
mod frontend;

use crate::calc::{Base, BinaryOp, Calculator, Mode};
use crate::calculation_log::CalculationLog;
use crate::expr::AngleMode;
use crate::history::History;
use crate::graph::{format_root_values, ExportFormat, GraphModel, RootResult, Viewport};
use crate::i18n::{Language, Strings};
use crate::platform;
use crate::settings::{DecimalSeparator, Settings, MAX_HISTORY_WIDTH, MIN_HISTORY_WIDTH};
use crate::tooltip::TooltipCatalog;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::path::{Path, PathBuf};
use wxdragon::color::colours::{BLACK, WHITE};
use wxdragon::dc::auto_buffered_paint_dc::AutoBufferedPaintDC;
use wxdragon::dialogs::file_dialog::{FileDialog, FileDialogStyle};
use wxdragon::id::ID_OK;
use wxdragon::window::BackgroundStyle;
use plotters::drawing::IntoDrawingArea;
use plotters_wxdragon::WxBackend;
use wxdragon::font::{Font, FontWeight};
use wxdragon::menus::menuitem::{ItemKind, MenuItem};
use wxdragon::prelude::*;
use wxdragon::widgets::radio_button::RadioButtonStyle;
use wxdragon::widgets::splitter_window::{SplitterWindow, SplitterWindowStyle};
use wxdragon::widgets::static_text::{StaticText, StaticTextStyle};
use wxdragon::window::WxWidget;
use wxdragon::widgets::textctrl::TextCtrlStyle;

// The recovered child-control coordinates are 96-DPI source units.  Render
// them at a 120-DPI design scale (125%) so the entire Calculator surface is
// comfortably larger on a modern display without changing the relative Win95
// layout.  Windows may still apply its normal monitor scaling on top of this.
const SOURCE_DPI: i32 = 96;
const DESIGN_DPI: i32 = 120;

const fn dp(value: i32) -> i32 {
    (value * DESIGN_DPI + SOURCE_DPI / 2) / SOURCE_DPI
}

// These are *client* sizes.  The old renderer included 43 pixels of fake
// title/menu chrome in its 302/355 pixel window heights.  wxDragon owns that
// non-client UI, so only the recovered client surface is scaled here.
const STD_W: i32 = dp(260);
const STD_H: i32 = dp(204);
const SCI_W: i32 = dp(500);
const SCI_H: i32 = dp(304);

// Optional calculation-history pane.  Buildfix40 makes this a genuine child
// pane of wxSplitterWindow rather than a second top-level frame.  The recovered
// Calculator controls stay in a fixed-size left host while History occupies the
// right pane. Both native frontends use the real non-live sash. Windows keeps
// its established pointer-transparent decoration; Linux styles and exposes the
// native GTK sash itself so dragging cannot be blocked. On release both paths
// resize History and restore the Calculator pane exactly.
const HISTORY_MARGIN: i32 = dp(8);
const HISTORY_HEADER_H: i32 = dp(18);
const HISTORY_BUTTON_W: i32 = dp(94);
const HISTORY_BUTTON_H: i32 = dp(24);
const HISTORY_BUTTON_BOTTOM: i32 = dp(8);
const HISTORY_GAP: i32 = dp(6);
const HISTORY_SEPARATOR_W: i32 = 2;

// Optional graph pane. It is a fixed-width child on the left of the existing
// Calculator/History splitter, so the recovered Calculator geometry never
// stretches or reflows when graphing is enabled.
const GRAPH_W: i32 = dp(330);
const GRAPH_MIN_W: i32 = dp(220);
const GRAPH_MAX_W: i32 = dp(520);
const GRAPH_MARGIN: i32 = dp(8);
const GRAPH_LABEL_H: i32 = dp(17);
const GRAPH_FIELD_H: i32 = dp(24);
const GRAPH_PLOT_W: i32 = dp(62);
const GRAPH_BUTTON_W: i32 = dp(92);
const GRAPH_BUTTON_H: i32 = dp(24);
const GRAPH_ROOTS_H: i32 = dp(48);
const GRAPH_GAP: i32 = dp(6);
const GRAPH_SEPARATOR_W: i32 = 2;

// Scientific-mode vertical geometry in recovered 96-DPI source units.
// Keep these rows coordinated: previous build fixes moved individual controls
// independently and caused the selector/status/keypad bands to overlap.
const SCI_SEPARATOR_Y: i32 = 16;
const SCI_DISPLAY_Y: i32 = 24;
const SCI_SELECTOR_BOX_Y: i32 = 58;
const SCI_SELECTOR_Y: i32 = 66;
const SCI_COMMAND_BOX_Y: i32 = 94;
const SCI_COMMAND_Y: i32 = 99;
// Native Windows checkboxes paint an opaque control background.  Keep them
// fully inside the Inv/Hyp decorator on *every* side, or that background
// erases the etched frame wherever the control overhangs it.
//
// The decorator is the group box at (13, SCI_COMMAND_BOX_Y) with width 127,
// so its right edge sits at 13 + 127 = 140.  Inv starts at x=20 and Hyp at
// x=86, which leaves 140 - 86 = 54 units for the widest checkbox.  The former
// width of 60 pushed Hyp's right edge to 146 and painted over the frame for
// exactly the checkbox's height -- the reported "cut on the right".  48 keeps
// a 6-unit right margin, matching the 7-unit left margin at x=20, and is still
// far wider than the "Hyp" label needs.
const SCI_CHECK_Y: i32 = 103;
const SCI_CHECK_H: i32 = 18;
const SCI_CHECK_W: i32 = 48;
const SCI_KEYPAD_Y: i32 = 133;
const SCI_KEYPAD_STEP: i32 = 34;

const ID_COPY: i32 = 6001;
const ID_PASTE: i32 = 6002;
const ID_UNDO: i32 = 6003;
const ID_REDO: i32 = 6004;
const ID_SCIENTIFIC: i32 = 6010;
const ID_STANDARD: i32 = 6011;
const ID_HELP_TOPICS: i32 = 6020;
const ID_ABOUT: i32 = 6021;
const ID_LANGUAGE_ENGLISH: i32 = 6030;
const ID_LANGUAGE_PORTUGUESE: i32 = 6031;
const ID_LANGUAGE_SPANISH: i32 = 6032;
const ID_SEPARATOR_PERIOD: i32 = 6040;
const ID_SEPARATOR_COMMA: i32 = 6041;
const ID_HISTORY_PANEL: i32 = 6050;
const ID_GRAPH_PANEL: i32 = 6051;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Digit(char),
    Dot,
    Back,
    CE,
    C,
    Sign,
    Eq,
    Percent,
    Bin(BinaryOp),
    KeyboardStar,
    Unary(&'static str),
    MemC,
    MemR,
    MemS,
    MemAdd,
    Pi,
    Open,
    Close,
    StatsOpen,
    StatsDat,
    StatsAvg,
    StatsSum,
    StatsDev,
    ToggleFE,
    Copy,
    Paste,
    About,
    Help,
}

#[derive(Clone, Copy)]
enum Tone {
    Red,
    Blue,
    Navy,
    Magenta,
    Maroon,
}

#[derive(Clone, Copy)]
struct ButtonDef {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &'static str,
    action: Action,
    tone: Tone,
}

struct MenuHandles {
    undo_item: MenuItem,
    redo_item: MenuItem,
    standard_item: MenuItem,
    scientific_item: MenuItem,
    history_item: MenuItem,
    graph_item: MenuItem,
    separator_submenu_item: MenuItem,
    language_submenu_item: MenuItem,
    separator_items: [MenuItem; 2],
}

#[derive(Clone, Copy)]
struct TooltipTarget {
    hwnd: *mut core::ffi::c_void,
    key: &'static str,
}

struct HistoryPanel {
    panel: Panel,
    separator: StaticText,
    title: StaticText,
    text: TextCtrl,
    clear_button: Button,
}

struct GraphPanel {
    panel: Panel,
    separator: StaticText,
    function_label: StaticText,
    expression: TextCtrl,
    plot_button: Button,
    canvas_frame: StaticText,
    canvas: Panel,
    roots: StaticText,
    reset_button: Button,
    export_button: Button,
    model: Rc<RefCell<GraphModel>>,
    drag: Rc<RefCell<Option<(Point, Viewport)>>>,
}

#[derive(Clone, Copy, Debug)]
struct HistoryEntryRange {
    /// UTF-16 character offsets because the native Windows edit control hit
    /// test reports positions in UTF-16 code units.
    start: usize,
    end: usize,
    newest_index: usize,
}

struct Ui {
    frame: Frame,
    root_surface: Panel,
    splitter: SplitterWindow,
    graph_panel: GraphPanel,
    graph_width: Cell<i32>,
    calculator_host: Panel,
    standard_panel: Panel,
    scientific_panel: Panel,
    history_panel: HistoryPanel,
    history_split: Cell<bool>,
    splitter_adjusting: Cell<bool>,
    standard_display: TextCtrl,
    scientific_display: TextCtrl,
    standard_memory: StaticText,
    scientific_memory: StaticText,
    scientific_parens: StaticText,
    inv: CheckBox,
    hyp: CheckBox,
    base_radios: [RadioButton; 4],
    angle_radios: [RadioButton; 3],
    menus: RefCell<MenuHandles>,
    calc: RefCell<Calculator>,
    history: RefCell<History<Calculator>>,
    calculation_log: RefCell<CalculationLog>,
    history_entry_ranges: RefCell<Vec<HistoryEntryRange>>,
    settings: RefCell<Settings>,
    tooltips: TooltipCatalog,
    tooltip_targets: Vec<TooltipTarget>,
    action_buttons: Vec<(Button, Action)>,
    stats_box: RefCell<Option<StatsBox>>,
    main_was_minimized: Cell<bool>,
}

pub fn run() -> Result<(), String> {
    platform::enable_modern_dpi_awareness();
    SystemOptions::set_option_by_int("msw.no-manifest-check", 1);

    wxdragon::main(|_app| {
        // Preserve the pre-existing system-locale default on first run, then let
        // the .cfg preference take precedence from that point onward.
        let mut calculator = Calculator::default();
        let settings = Settings::load(calculator.decimal_separator());
        calculator.set_decimal_separator(settings.decimal_separator.as_char());
        let strings = Strings::new(settings.language);

        // Calculator and History now share one native top-level frame.  A real
        // wxSplitterWindow owns a fixed-layout Calculator host on the left and
        // the optional History pane on the right.  The splitter itself fills the
        // client area, so there can be no top-level-window gap between them.
        let initial_history_width = dp(settings.history_width.clamp(
            MIN_HISTORY_WIDTH,
            MAX_HISTORY_WIDTH,
        ));
        let initial_graph_width = if settings.graph_visible { GRAPH_W } else { 0 };
        let initial_history_gutter = if settings.history_visible {
            frontend::history_leading_gutter()
        } else {
            0
        };
        let initial_sash_extent = if settings.history_visible {
            frontend::history_sash_extent()
        } else {
            0
        };
        let initial_calculator_pane_width = STD_W + initial_history_gutter;
        let initial_splitter_width = initial_calculator_pane_width
            + if settings.history_visible {
                initial_sash_extent + initial_history_width
            } else {
                0
            };
        let initial_frame_width = initial_graph_width + initial_splitter_width;
        let frame = Frame::builder()
            .with_title(strings.calculator_title())
            .with_size(Size::new(initial_frame_width, STD_H))
            .build();
        frame.remove_style(WindowStyle::ThickFrame | WindowStyle::MaximizeBox);
        platform::set_calculator_icon(frame.get_handle());
        platform::install_context_help_dismissal(frame.get_handle());

        let font = frontend::classic_font(FontWeight::Normal);
        let button_font = frontend::classic_font(FontWeight::Bold);
        let tooltips = TooltipCatalog::load_default();
        let initial_display = calculator.display.clone();
        let decimal_label = calculator.decimal_separator().to_string();
        frame.set_font(&font);
        frontend::apply_surface(&frame);

        let root_surface = Panel::builder(&frame)
            .with_pos(Point::new(0, 0))
            .with_size(Size::new(initial_frame_width, STD_H))
            .build();
        root_surface.set_font(&font);
        frontend::apply_surface(&root_surface);
        platform::install_context_help_dismissal(root_surface.get_handle());

        let menu_handles = install_menu_bar(
            &frame,
            strings,
            calculator.mode,
            settings.language,
            settings.decimal_separator,
            settings.history_visible,
            settings.graph_visible,
        );

        // Do not use LiveUpdate: the classic splitter guide moves while the
        // user drags, but the recovered Calculator controls never reflow or
        // clip.  The sash-release handler converts the drag into a persistent
        // History-width change and restores the left pane to its canonical
        // Standard/Scientific width.
        let splitter = SplitterWindow::builder(&root_surface)
            .with_pos(Point::new(initial_graph_width, 0))
            .with_size(Size::new(initial_splitter_width, STD_H))
            .with_style(frontend::splitter_style())
            .build();
        splitter.set_minimum_pane_size(platform::scale_classic_control_metric(
            splitter.get_handle(),
            dp(48),
        ));
        frontend::apply_classic_theme(&splitter);
        frontend::apply_surface(&splitter);
        platform::install_classic_splitter_painter(splitter.get_handle());
        platform::install_context_help_dismissal(splitter.get_handle());

        let calculator_host = Panel::builder(&splitter)
            .with_pos(Point::new(0, 0))
            .with_size(Size::new(initial_calculator_pane_width, STD_H))
            .build();
        calculator_host.set_font(&font);
        frontend::apply_surface(&calculator_host);
        platform::install_context_help_dismissal(calculator_host.get_handle());

        // The recovered Calculator panels keep their exact coordinates and
        // dimensions inside the fixed left host.  Only the host participates in
        // the splitter; the calculator controls themselves are never resized.
        let standard_panel = Panel::builder(&calculator_host)
            .with_pos(Point::new(0, 0))
            .with_size(Size::new(STD_W, STD_H))
            .build();
        standard_panel.set_font(&font);
        frontend::apply_surface(&standard_panel);
        platform::install_context_help_dismissal(standard_panel.get_handle());

        let scientific_panel = Panel::builder(&calculator_host)
            .with_pos(Point::new(0, 0))
            .with_size(Size::new(SCI_W, SCI_H))
            .build();
        scientific_panel.set_font(&font);
        frontend::apply_surface(&scientific_panel);
        scientific_panel.show(false);
        platform::install_context_help_dismissal(scientific_panel.get_handle());

        let graph_panel = build_graph_panel(
            &root_surface,
            GRAPH_W,
            STD_H,
            &font,
            &button_font,
            strings,
        );
        graph_panel.panel.show(settings.graph_visible);

        let history_panel = build_history_panel(
            &splitter,
            initial_history_width,
            STD_H,
            &font,
            &button_font,
            strings,
        );

        let history_split = if settings.history_visible {
            history_panel.panel.show(true);
            let sash = platform::scale_classic_control_metric(
                splitter.get_handle(),
                initial_calculator_pane_width,
            );
            splitter.split_vertically(&calculator_host, &history_panel.panel, sash)
        } else {
            history_panel.panel.show(false);
            splitter.initialize(&calculator_host);
            false
        };

        let standard_display = make_display(&standard_panel, 10, 5, 245, &font, &initial_display);
        let scientific_display = make_display(&scientific_panel, 244, SCI_DISPLAY_Y, 240, &font, &initial_display);
        let standard_memory = make_indicator(&standard_panel, &font, 10, 39, 38, 27);
        let scientific_parens = make_indicator(&scientific_panel, &font, 145, SCI_COMMAND_Y + 2, 35, 27);
        let scientific_memory = make_indicator(&scientific_panel, &font, 197, SCI_COMMAND_Y + 2, 35, 27);

        let mut tooltip_targets = Vec::new();
        attach_context_help(&standard_display, &tooltips, settings.language, strings.whats_this(), "display", &mut tooltip_targets);
        attach_context_help(&scientific_display, &tooltips, settings.language, strings.whats_this(), "display", &mut tooltip_targets);
        attach_context_help(&standard_memory, &tooltips, settings.language, strings.whats_this(), "memory_indicator", &mut tooltip_targets);
        attach_context_help(&scientific_memory, &tooltips, settings.language, strings.whats_this(), "memory_indicator", &mut tooltip_targets);
        attach_context_help(&scientific_parens, &tooltips, settings.language, strings.whats_this(), "paren_indicator", &mut tooltip_targets);

        frontend::make_separator_line(&scientific_panel, SCI_SEPARATOR_Y, 500);
        frontend::make_group_box(&scientific_panel, 13, SCI_SELECTOR_BOX_Y, 266, 34);
        frontend::make_group_box(&scientific_panel, 286, SCI_SELECTOR_BOX_Y, 198, 34);
        frontend::make_group_box(&scientific_panel, 13, SCI_COMMAND_BOX_Y, 127, 36);

        let mut action_buttons: Vec<(Button, Action)> = Vec::new();
        for def in standard_button_defs() {
            let button = make_button(&standard_panel, &button_font, def, &decimal_label);
            attach_context_help(&button, &tooltips, settings.language, strings.whats_this(), action_help_key(def.action), &mut tooltip_targets);
            action_buttons.push((button, def.action));
        }
        for def in scientific_button_defs() {
            let button = make_button(&scientific_panel, &button_font, def, &decimal_label);
            attach_context_help(&button, &tooltips, settings.language, strings.whats_this(), action_help_key(def.action), &mut tooltip_targets);
            action_buttons.push((button, def.action));
        }

        let base_radios = [
            make_radio(&scientific_panel, &font, 20, SCI_SELECTOR_Y, 58, "Hex", true),
            make_radio(&scientific_panel, &font, 84, SCI_SELECTOR_Y, 58, "Dec", false),
            make_radio(&scientific_panel, &font, 148, SCI_SELECTOR_Y, 58, "Oct", false),
            make_radio(&scientific_panel, &font, 212, SCI_SELECTOR_Y, 50, "Bin", false),
        ];
        base_radios[1].set_value(true);
        for (radio, key) in base_radios.iter().zip(["hex", "dec", "oct", "bin"]) {
            attach_context_help(radio, &tooltips, settings.language, strings.whats_this(), key, &mut tooltip_targets);
        }

        let inv = CheckBox::builder(&scientific_panel)
            .with_label("Inv")
            .with_pos(Point::new(classic_metric(&scientific_panel, 20), classic_metric(&scientific_panel, SCI_CHECK_Y)))
            .with_size(Size::new(classic_metric(&scientific_panel, SCI_CHECK_W), classic_metric(&scientific_panel, SCI_CHECK_H)))
            .build();
        inv.set_font(&font);
        frontend::apply_classic_theme(&inv);
        inv.raise();
        attach_context_help(&inv, &tooltips, settings.language, strings.whats_this(), "inv", &mut tooltip_targets);

        let hyp = CheckBox::builder(&scientific_panel)
            .with_label("Hyp")
            .with_pos(Point::new(classic_metric(&scientific_panel, 86), classic_metric(&scientific_panel, SCI_CHECK_Y)))
            .with_size(Size::new(classic_metric(&scientific_panel, SCI_CHECK_W), classic_metric(&scientific_panel, SCI_CHECK_H)))
            .build();
        hyp.set_font(&font);
        frontend::apply_classic_theme(&hyp);
        hyp.raise();
        attach_context_help(&hyp, &tooltips, settings.language, strings.whats_this(), "hyp", &mut tooltip_targets);

        let angle_radios = [
            make_radio(&scientific_panel, &font, 292, SCI_SELECTOR_Y, 60, "Deg", true),
            make_radio(&scientific_panel, &font, 359, SCI_SELECTOR_Y, 60, "Rad", false),
            make_radio(&scientific_panel, &font, 425, SCI_SELECTOR_Y, 52, "Grad", false),
        ];
        angle_radios[0].set_value(true);
        for (radio, key) in angle_radios.iter().zip(["deg", "rad", "grad"]) {
            attach_context_help(radio, &tooltips, settings.language, strings.whats_this(), key, &mut tooltip_targets);
        }

        let ui = Rc::new(Ui {
            frame,
            root_surface,
            splitter,
            graph_panel,
            graph_width: Cell::new(GRAPH_W),
            calculator_host,
            standard_panel,
            scientific_panel,
            history_panel,
            history_split: Cell::new(history_split),
            splitter_adjusting: Cell::new(false),
            standard_display,
            scientific_display,
            standard_memory,
            scientific_memory,
            scientific_parens,
            inv,
            hyp,
            base_radios,
            angle_radios,
            menus: RefCell::new(menu_handles),
            calc: RefCell::new(calculator),
            history: RefCell::new(History::default()),
            calculation_log: RefCell::new(CalculationLog::default()),
            history_entry_ranges: RefCell::new(Vec::new()),
            settings: RefCell::new(settings),
            tooltips,
            tooltip_targets,
            action_buttons,
            stats_box: RefCell::new(None),
            main_was_minimized: Cell::new(false),
        });

        for (button, action) in &ui.action_buttons {
            let ui_c = Rc::clone(&ui);
            let action = *action;
            button.on_click(move |_| {
                perform(&ui_c, action);
                // The original CALC.EXE dispatches its accelerator table before
                // dialog/control processing, so clicking a calculator key never
                // strands subsequent keyboard input on that child control.
                ui_c.frame.set_focus();
            });
        }

        {
            let ui_c = Rc::clone(&ui);
            ui.history_panel.clear_button.on_click(move |_| {
                clear_calculation_history(&ui_c);
                ui_c.frame.set_focus();
            });
        }

        bind_history_recall(&ui);
        bind_scientific_selectors(&ui);
        bind_menu(&ui);
        bind_keyboard(&ui);
        bind_splitter(&ui);
        bind_graph(&ui);
        frontend::install_panel_resizing(&ui);
        bind_companion_tracking(&ui);
        refresh(&ui);
        refresh_calculation_history(&ui);

        // The native DPI fitter needs a realized HWND.  Once the single
        // Calculator+History frame exists, fit the splitter surface exactly and
        // center the whole application on the current work area.
        ui.frame.show(true);
        let initial_mode = ui.calc.borrow().mode;
        sync_mode_surface(&ui, initial_mode);
        if !platform::center_window_on_work_area(ui.frame.get_handle()) {
            ui.frame.centre();
        }
        // CALC.EXE's accelerator table is active as soon as the top-level
        // calculator becomes the active window. wxWidgets otherwise leaves
        // initial keyboard focus on an arbitrary child/default control until
        // the user clicks one of our buttons. Put focus on the Calculator
        // frame immediately so numeric/operator typing works from startup.
        ui.frame.set_focus();
    })
    .map_err(|error| format!("wxDragon failed to start: {error:?}"))?;
    Ok(())
}

fn install_menu_bar(
    frame: &Frame,
    strings: Strings,
    mode: Mode,
    language: Language,
    separator: DecimalSeparator,
    history_visible: bool,
    graph_visible: bool,
) -> MenuHandles {
    let edit_menu = Menu::builder().build();
    let undo_item = edit_menu
        .append(ID_UNDO, strings.undo(), strings.undo_help(), ItemKind::Normal)
        .expect("Undo menu item");
    let redo_item = edit_menu
        .append(ID_REDO, strings.redo(), strings.redo_help(), ItemKind::Normal)
        .expect("Redo menu item");
    edit_menu.append_separator();
    edit_menu
        .append(ID_COPY, strings.copy(), strings.copy_help(), ItemKind::Normal)
        .expect("Copy menu item");
    edit_menu
        .append(ID_PASTE, strings.paste(), strings.paste_help(), ItemKind::Normal)
        .expect("Paste menu item");

    // No history exists at startup, so both commands begin disabled.
    undo_item.enable(false);
    redo_item.enable(false);

    let view_menu = Menu::builder().build();
    let scientific_item = view_menu
        .append(ID_SCIENTIFIC, strings.scientific(), strings.scientific_help(), ItemKind::Radio)
        .expect("Scientific menu item");
    let standard_item = view_menu
        .append(ID_STANDARD, strings.standard(), strings.standard_help(), ItemKind::Radio)
        .expect("Standard menu item");
    view_menu.append_separator();
    let graph_item = view_menu
        .append(ID_GRAPH_PANEL, strings.graph(), strings.graph_help(), ItemKind::Check)
        .expect("Graph menu item");
    let history_item = view_menu
        .append(ID_HISTORY_PANEL, strings.history(), strings.history_help(), ItemKind::Check)
        .expect("History menu item");
    view_menu.append_separator();

    let separator_menu = Menu::builder().build();
    let period_item = separator_menu
        .append(ID_SEPARATOR_PERIOD, strings.period_separator(), strings.separator_help(), ItemKind::Radio)
        .expect("Period separator menu item");
    let comma_item = separator_menu
        .append(ID_SEPARATOR_COMMA, strings.comma_separator(), strings.separator_help(), ItemKind::Radio)
        .expect("Comma separator menu item");
    let separator_submenu_item = view_menu
        .append_submenu(separator_menu, strings.decimal_separator_menu(), strings.separator_help())
        .expect("Decimal separator submenu");

    let language_menu = Menu::builder().build();
    let english_item = language_menu
        .append(ID_LANGUAGE_ENGLISH, Language::English.autonym(), strings.language_help(), ItemKind::Radio)
        .expect("English language menu item");
    let portuguese_item = language_menu
        .append(ID_LANGUAGE_PORTUGUESE, Language::Portuguese.autonym(), strings.language_help(), ItemKind::Radio)
        .expect("Portuguese language menu item");
    let spanish_item = language_menu
        .append(ID_LANGUAGE_SPANISH, Language::Spanish.autonym(), strings.language_help(), ItemKind::Radio)
        .expect("Spanish language menu item");
    let language_submenu_item = view_menu
        .append_submenu(language_menu, strings.language_menu(), strings.language_help())
        .expect("Language submenu");

    let help_menu = Menu::builder()
        .append_item(ID_HELP_TOPICS, strings.help_topics(), strings.help_topics_help())
        .append_separator()
        .append_item(ID_ABOUT, strings.about_opencalc(), strings.about_title())
        .build();

    let menu_bar = MenuBar::builder()
        .append(edit_menu, strings.edit_menu())
        .append(view_menu, strings.view_menu())
        .append(help_menu, strings.help_menu())
        .build();
    frame.set_menu_bar(menu_bar);

    standard_item.check(mode == Mode::Standard);
    scientific_item.check(mode == Mode::Scientific);
    english_item.check(language == Language::English);
    portuguese_item.check(language == Language::Portuguese);
    spanish_item.check(language == Language::Spanish);
    period_item.check(separator == DecimalSeparator::Period);
    comma_item.check(separator == DecimalSeparator::Comma);
    history_item.check(history_visible);
    graph_item.check(graph_visible);

    MenuHandles {
        undo_item,
        redo_item,
        standard_item,
        scientific_item,
        history_item,
        graph_item,
        separator_submenu_item,
        language_submenu_item,
        separator_items: [period_item, comma_item],
    }
}

fn attach_context_help(
    widget: &impl WxWidget,
    catalog: &TooltipCatalog,
    language: Language,
    whats_this: &str,
    key: &'static str,
    targets: &mut Vec<TooltipTarget>,
) {
    let hwnd = widget.get_handle();
    if let Some(text) = catalog.get(language, key) {
        platform::install_context_help(hwnd, text, whats_this);
    }
    targets.push(TooltipTarget { hwnd, key });
}

fn refresh_context_help(ui: &Ui) {
    let language = ui.settings.borrow().language;
    let strings = Strings::new(language);
    for target in &ui.tooltip_targets {
        if let Some(text) = ui.tooltips.get(language, target.key) {
            platform::install_context_help(target.hwnd, text, strings.whats_this());
        }
    }
}

fn action_help_key(action: Action) -> &'static str {
    match action {
        Action::Digit(ch) if ch.is_ascii_alphabetic() => "hex_digit",
        Action::Digit(_) => "digit",
        Action::Dot => "decimal_point",
        Action::Back => "back",
        Action::CE => "ce",
        Action::C => "clear",
        Action::Sign => "sign",
        Action::Eq => "equals",
        Action::Percent => "percent",
        Action::Bin(BinaryOp::Add) => "add",
        Action::Bin(BinaryOp::Sub) => "subtract",
        Action::Bin(BinaryOp::Mul) | Action::KeyboardStar => "multiply",
        Action::Bin(BinaryOp::Div) => "divide",
        Action::Bin(BinaryOp::Mod) => "mod",
        Action::Bin(BinaryOp::Pow) | Action::Bin(BinaryOp::Root) => "pow",
        Action::Bin(BinaryOp::And) => "and",
        Action::Bin(BinaryOp::Or) => "or",
        Action::Bin(BinaryOp::Xor) => "xor",
        Action::Bin(BinaryOp::Lsh) => "lsh",
        Action::Unary("sqrt") => "sqrt",
        Action::Unary("recip") => "reciprocal",
        Action::Unary("dms") => "dms",
        Action::Unary("sin") => "sin",
        Action::Unary("cos") => "cos",
        Action::Unary("tan") => "tan",
        Action::Unary("exp") => "exp",
        Action::Unary("cube") => "cube",
        Action::Unary("square") => "square",
        Action::Unary("ln") => "ln",
        Action::Unary("log") => "log",
        Action::Unary("factorial") => "factorial",
        Action::Unary("not") => "not",
        Action::Unary("int") => "int",
        Action::Unary(_) => "display",
        Action::MemC => "mc",
        Action::MemR => "mr",
        Action::MemS => "ms",
        Action::MemAdd => "mplus",
        Action::Pi => "pi",
        Action::Open => "open_paren",
        Action::Close => "close_paren",
        Action::StatsOpen => "sta",
        Action::StatsDat => "dat",
        Action::StatsAvg => "ave",
        Action::StatsSum => "sum",
        Action::StatsDev => "stddev",
        Action::ToggleFE => "fe",
        Action::Copy | Action::Paste | Action::About | Action::Help => "display",
    }
}

fn make_display(parent: &Panel, x: i32, y: i32, width: i32, font: &Font, initial: &str) -> TextCtrl {
    let display = TextCtrl::builder(parent)
        .with_value(initial)
        .with_pos(Point::new(dp(x), dp(y)))
        .with_size(Size::new(dp(width), dp(24)))
        .with_style(TextCtrlStyle::ReadOnly | TextCtrlStyle::Right)
        .build();
    display.set_font(font);
    frontend::apply_classic_theme(&display);
    display.set_background_color(WHITE);
    platform::install_classic_display_painter(display.get_handle());
    display.set_can_focus(false);
    display
}

fn build_history_panel<W: WxWidget>(
    parent: &W,
    width: i32,
    height: i32,
    font: &Font,
    button_font: &Font,
    strings: Strings,
) -> HistoryPanel {
    let panel = Panel::builder(parent)
        .with_pos(Point::new(0, 0))
        .with_size(Size::new(width, height))
        .build();
    panel.set_font(font);
    frontend::apply_surface(&panel);
    platform::install_context_help_dismissal(panel.get_handle());

    // wxSplitterWindow's native sash can blend into the pane face or inherit
    // a desktop-theme colour. Windows keeps the established visible overlay;
    // Linux styles the real GTK sash directly and leaves this decoration hidden
    // so native hit-testing and drag feedback remain fully functional.
    let separator = frontend::build_history_separator(parent, &panel, height);
    platform::install_classic_vertical_separator_painter(separator.get_handle());
    // Where this overlay is shown, it is decoration only. Pointer input must
    // reach the real wxSplitterWindow sash underneath. Linux does not show it.
    platform::make_pointer_passthrough(separator.get_handle());
    separator.show(false);

    // Keep the side pane deliberately plain: a small native heading, a
    // read-only multiline field with the platform vertical scrollbar, and a
    // classic pushbutton at the bottom.  This belongs beside the Win95
    // calculator rather than looking like a modern floating card.
    let title = StaticText::builder(&panel)
        .with_label(strings.history_title())
        .with_pos(Point::new(HISTORY_MARGIN, HISTORY_MARGIN))
        .with_size(Size::new(width - 2 * HISTORY_MARGIN, HISTORY_HEADER_H))
        .build();
    title.set_font(button_font);
    title.set_foreground_color(BLACK);
    platform::install_context_help_dismissal(title.get_handle());

    let text_y = HISTORY_MARGIN + HISTORY_HEADER_H + HISTORY_GAP;
    let button_y = height - HISTORY_BUTTON_BOTTOM - HISTORY_BUTTON_H;
    let text_height = button_y - HISTORY_GAP - text_y;
    let text = TextCtrl::builder(&panel)
        .with_value("")
        .with_pos(Point::new(HISTORY_MARGIN, text_y))
        .with_size(Size::new(width - 2 * HISTORY_MARGIN, text_height))
        .with_style(
            TextCtrlStyle::MultiLine
                | TextCtrlStyle::ReadOnly
                | TextCtrlStyle::Right
                | TextCtrlStyle::WordWrap,
        )
        .build();
    text.set_font(font);
    frontend::apply_classic_theme(&text);
    text.set_background_color(WHITE);
    frontend::style_history_text(&text);
    text.set_can_focus(false);
    platform::install_context_help_dismissal(text.get_handle());

    let clear_button = Button::builder(&panel)
        .with_label(strings.clear_history())
        .with_pos(Point::new(
            width - HISTORY_MARGIN - HISTORY_BUTTON_W,
            button_y,
        ))
        .with_size(Size::new(HISTORY_BUTTON_W, HISTORY_BUTTON_H))
        .build();
    clear_button.set_font(button_font);
    platform::install_classic_button_painter(clear_button.get_handle(), 0, 0, 0);
    clear_button.set_can_focus(false);
    platform::install_context_help_dismissal(clear_button.get_handle());

    HistoryPanel {
        panel,
        separator,
        title,
        text,
        clear_button,
    }
}

fn build_graph_panel<W: WxWidget>(
    parent: &W,
    width: i32,
    height: i32,
    font: &Font,
    button_font: &Font,
    strings: Strings,
) -> GraphPanel {
    let panel = Panel::builder(parent)
        .with_pos(Point::new(0, 0))
        .with_size(Size::new(width, height))
        .build();
    panel.set_font(font);
    frontend::apply_surface(&panel);
    platform::install_context_help_dismissal(panel.get_handle());

    let separator_width = frontend::graph_separator_width();
    let separator = StaticText::builder(&panel)
        .with_label("")
        .with_pos(Point::new(width - separator_width, 0))
        .with_size(Size::new(separator_width, height))
        .build();
    platform::install_classic_vertical_separator_painter(separator.get_handle());

    let function_label = StaticText::builder(&panel)
        .with_label(strings.graph_function())
        .with_pos(Point::new(GRAPH_MARGIN, GRAPH_MARGIN))
        .with_size(Size::new(width - 2 * GRAPH_MARGIN, GRAPH_LABEL_H))
        .build();
    function_label.set_font(button_font);

    let field_y = GRAPH_MARGIN + GRAPH_LABEL_H;
    let expression_w = width - 2 * GRAPH_MARGIN - GRAPH_PLOT_W - GRAPH_GAP - separator_width;
    let expression = TextCtrl::builder(&panel)
        .with_value("")
        .with_pos(Point::new(GRAPH_MARGIN, field_y))
        .with_size(Size::new(expression_w, GRAPH_FIELD_H))
        .with_style(TextCtrlStyle::ProcessEnter)
        .build();
    expression.set_font(font);
    frontend::apply_classic_theme(&expression);
    expression.set_background_color(WHITE);
    frontend::style_graph_expression(&expression);

    let plot_button = Button::builder(&panel)
        .with_label(strings.graph_plot())
        .with_pos(Point::new(GRAPH_MARGIN + expression_w + GRAPH_GAP, field_y))
        .with_size(Size::new(GRAPH_PLOT_W, GRAPH_FIELD_H))
        .build();
    plot_button.set_font(button_font);
    platform::install_classic_button_painter(plot_button.get_handle(), 0, 0, 0);
    plot_button.set_can_focus(false);

    let canvas_frame = StaticText::builder(&panel)
        .with_label("")
        .with_pos(Point::new(GRAPH_MARGIN, field_y + GRAPH_FIELD_H + GRAPH_GAP))
        .with_size(Size::new(width - 2 * GRAPH_MARGIN - separator_width, dp(110)))
        .build();
    platform::install_classic_sunken_field_painter(canvas_frame.get_handle());
    canvas_frame.lower();

    let canvas = Panel::builder(&panel)
        .with_pos(Point::new(GRAPH_MARGIN + 2, field_y + GRAPH_FIELD_H + GRAPH_GAP + 2))
        .with_size(Size::new(width - 2 * GRAPH_MARGIN - separator_width - 4, dp(106)))
        .build();
    canvas.set_background_color(WHITE);
    canvas.set_background_style(BackgroundStyle::Paint);
    canvas.set_can_focus(false);

    let roots = StaticText::builder(&panel)
        .with_label(strings.graph_roots_not_plotted())
        .with_pos(Point::new(GRAPH_MARGIN, height - GRAPH_MARGIN - GRAPH_BUTTON_H - GRAPH_GAP - GRAPH_ROOTS_H))
        .with_size(Size::new(width - 2 * GRAPH_MARGIN - separator_width, GRAPH_ROOTS_H))
        .build();
    roots.set_font(font);

    let button_y = height - GRAPH_MARGIN - GRAPH_BUTTON_H;
    let reset_button = Button::builder(&panel)
        .with_label(strings.graph_reset_view())
        .with_pos(Point::new(GRAPH_MARGIN, button_y))
        .with_size(Size::new(GRAPH_BUTTON_W, GRAPH_BUTTON_H))
        .build();
    reset_button.set_font(button_font);
    platform::install_classic_button_painter(reset_button.get_handle(), 0, 0, 0);
    reset_button.set_can_focus(false);
    reset_button.enable(false);

    let export_button = Button::builder(&panel)
        .with_label(strings.graph_export())
        .with_pos(Point::new(width - GRAPH_MARGIN - separator_width - GRAPH_BUTTON_W, button_y))
        .with_size(Size::new(GRAPH_BUTTON_W, GRAPH_BUTTON_H))
        .build();
    export_button.set_font(button_font);
    platform::install_classic_button_painter(export_button.get_handle(), 0, 0, 0);
    export_button.set_can_focus(false);
    export_button.enable(false);

    GraphPanel {
        panel,
        separator,
        function_label,
        expression,
        plot_button,
        canvas_frame,
        canvas,
        roots,
        reset_button,
        export_button,
        model: Rc::new(RefCell::new(GraphModel::default())),
        drag: Rc::new(RefCell::new(None)),
    }
}

fn classic_metric(parent: &Panel, value: i32) -> i32 {
    platform::scale_classic_control_metric(parent.get_handle(), dp(value))
}

fn make_indicator(parent: &Panel, font: &Font, x: i32, y: i32, w: i32, h: i32) -> StaticText {
    // This is a single status well, not a StaticBox plus a second child field.
    // The native StaticText owns the control while the Windows subclass paints
    // the complete recessed Win95 edge and its centred status text.
    let label = StaticText::builder(parent)
        .with_label("")
        .with_pos(Point::new(dp(x), dp(y)))
        .with_size(Size::new(dp(w), dp(h)))
        .with_style(StaticTextStyle::AlignCenterHorizontal)
        .build();
    label.set_font(font);
    label.set_foreground_color(BLACK);
    platform::install_classic_sunken_field_painter(label.get_handle());
    label
}

fn make_button(parent: &Panel, font: &Font, def: ButtonDef, decimal_label: &str) -> Button {
    let label = if matches!(def.action, Action::Dot) { decimal_label } else { def.label };
    let button = Button::builder(parent)
        .with_label(label)
        .with_pos(Point::new(dp(def.x), dp(def.y)))
        .with_size(Size::new(dp(def.w), dp(def.h)))
        .build();
    button.set_font(font);

    let (red, green, blue) = match def.tone {
        // CALC.EXE carries an eight-entry COLORREF palette for its owner-drawn
        // button text.  In order: RGB(255,0,0), RGB(128,0,128), RGB(0,0,255),
        // RGB(0,0,128), RGB(255,0,255), RGB(128,0,0), white, black.  Navy is
        // the palette's second blue -- the darker one used for the statistics
        // column and the hex/constant keys, which reads as indigo next to the
        // bright blue of the decimal digits.
        Tone::Red => (255, 0, 0),
        Tone::Blue => (0, 0, 255),
        Tone::Navy => (0, 0, 128),
        Tone::Magenta => (128, 0, 128),
        Tone::Maroon => (128, 0, 0),
    };

    // Do not use SetWindowTheme(..., "", "") on pushbuttons: on current
    // wxWidgets/Windows combinations that can leave only the owner-drawn text
    // visible.  Keep the real wxButton and install a tiny Windows paint
    // subclass which restores the classic COLOR_BTNFACE raised/sunken frame.
    // The native control continues to own hit-testing, clicking and events.
    platform::install_classic_button_painter(
        button.get_handle(),
        red,
        green,
        blue,
    );

    // Calculator is keyboard driven; keeping the keypad buttons out of the
    // focus chain leaves character events on the frame after mouse clicks.
    button.set_can_focus(false);
    button
}

fn make_radio(
    parent: &Panel,
    font: &Font,
    x: i32,
    y: i32,
    w: i32,
    label: &'static str,
    group_start: bool,
) -> RadioButton {
    let style = if group_start {
        RadioButtonStyle::GroupStart
    } else {
        RadioButtonStyle::Default
    };
    let radio = RadioButton::builder(parent)
        .with_label(label)
        .with_pos(Point::new(classic_metric(parent, x), classic_metric(parent, y)))
        .with_size(Size::new(classic_metric(parent, w), classic_metric(parent, 22)))
        .with_style(style)
        .build();
    radio.set_font(font);
    frontend::apply_classic_theme(&radio);
    // Keep the selector above the decorative group box that frames it, so the
    // frame can never sit between the pointer and this control.
    radio.raise();
    radio
}

fn bind_scientific_selectors(ui: &Rc<Ui>) {
    // Listen to the controls' own command events -- wxEVT_RADIOBUTTON and
    // wxEVT_CHECKBOX -- rather than intercepting raw input.
    //
    // The previous version hooked wxEVT_LEFT_UP / wxEVT_KEY_UP and then called
    // event.skip(false).  On wxMSW these are native BUTTON controls: they take
    // the mouse capture on WM_LBUTTONDOWN and only release it, update their
    // check state and emit BN_CLICKED when they see the matching WM_LBUTTONUP.
    // Swallowing that message left the control mid-click -- still capturing the
    // mouse, never notifying, never redrawing -- which is why the selectors
    // looked dead and stayed dead for subsequent clicks.
    //
    // Letting the control process its own input restores mouse clicks, Space,
    // arrow-key navigation within a radio group and screen-reader reporting for
    // free, on both wxMSW and wxGTK.  The model is then updated from the
    // resulting command event, and refresh() writes the authoritative state
    // back onto every selector.  set_value() never emits a command event, so
    // that write-back cannot re-enter these handlers.
    let base_actions = [Base::Hex, Base::Dec, Base::Oct, Base::Bin];
    for (index, base) in base_actions.into_iter().enumerate() {
        let ui_c = Rc::clone(ui);
        ui.base_radios[index].on_selected(move |_| select_base(&ui_c, index, base));
    }

    let angle_actions = [AngleMode::Degrees, AngleMode::Radians, AngleMode::Grads];
    for (index, angle) in angle_actions.into_iter().enumerate() {
        let ui_c = Rc::clone(ui);
        ui.angle_radios[index].on_selected(move |_| select_angle(&ui_c, index, angle));
    }

    {
        let ui_c = Rc::clone(ui);
        ui.inv.on_toggled(move |event| {
            let checked = event.is_checked();
            mutate_calculator(&ui_c, |calc| calc.inv = checked);
            refresh(&ui_c);
        });
    }
    {
        let ui_c = Rc::clone(ui);
        ui.hyp.on_toggled(move |event| {
            let checked = event.is_checked();
            mutate_calculator(&ui_c, |calc| calc.hyp = checked);
            refresh(&ui_c);
        });
    }

    // Windows: additionally watch the panel for the controls' own
    // WM_COMMAND/BN_CLICKED notifications.  Two independent attempts to drive
    // these selectors through the wx event layer (raw mouse/key interception,
    // then the wxEVT_RADIOBUTTON/wxEVT_CHECKBOX command events above) both left
    // them inert on this build, so the notification is taken straight from the
    // native control instead.  Every branch of apply_selector is idempotent and
    // reads the control's true state, so if the wx handlers *are* firing after
    // all, the duplicate delivery is harmless rather than a double-toggle.
    frontend::install_selector_bridge(ui);
}

/// Apply a selector identified by its index in the notifier table:
/// 0-3 radix, 4-6 angle, 7 Inv, 8 Hyp.  The checkbox branches copy the
/// control's real BM_GETCHECK state into the model rather than toggling a
/// cached flag, which keeps the model correct no matter how many times (or how
/// few) the notification is delivered.
fn select_base(ui: &Ui, index: usize, base: Base) {
    for (i, radio) in ui.base_radios.iter().enumerate() {
        radio.set_value(i == index);
    }
    mutate_calculator(ui, |calc| calc.set_base(base));
    refresh(ui);
}

fn select_angle(ui: &Ui, index: usize, angle: AngleMode) {
    for (i, radio) in ui.angle_radios.iter().enumerate() {
        radio.set_value(i == index);
    }
    mutate_calculator(ui, |calc| calc.angle = angle);
    // A plotted trigonometric expression follows the same Deg/Rad/Grad mode as
    // the Scientific calculator. Recompile it immediately when the mode changes.
    replot_existing_graph(ui);
    refresh(ui);
}

fn bind_menu(ui: &Rc<Ui>) {
    let ui_c = Rc::clone(ui);
    ui.frame.on_menu_selected(move |event: MenuEventData| {
        // Menus are outside the child-control subclass path, so explicitly
        // dismiss a tracking context-help popup before performing any command.
        platform::dismiss_context_tooltip();
        match event.get_id() {
            ID_UNDO => undo(&ui_c),
            ID_REDO => redo(&ui_c),
            ID_COPY => perform(&ui_c, Action::Copy),
            ID_PASTE => perform(&ui_c, Action::Paste),
            ID_SCIENTIFIC => set_mode(&ui_c, Mode::Scientific),
            ID_STANDARD => set_mode(&ui_c, Mode::Standard),
            ID_GRAPH_PANEL => {
                let visible = !ui_c.settings.borrow().graph_visible;
                set_graph_visible(&ui_c, visible);
            }
            ID_HISTORY_PANEL => {
                let visible = !ui_c.settings.borrow().history_visible;
                set_history_visible(&ui_c, visible);
            }
            ID_LANGUAGE_ENGLISH => set_language(&ui_c, Language::English),
            ID_LANGUAGE_PORTUGUESE => set_language(&ui_c, Language::Portuguese),
            ID_LANGUAGE_SPANISH => set_language(&ui_c, Language::Spanish),
            ID_SEPARATOR_PERIOD => set_decimal_separator(&ui_c, DecimalSeparator::Period),
            ID_SEPARATOR_COMMA => set_decimal_separator(&ui_c, DecimalSeparator::Comma),
            ID_HELP_TOPICS => perform(&ui_c, Action::Help),
            ID_ABOUT => perform(&ui_c, Action::About),
            _ => {}
        }
    });
}

fn strings_for(ui: &Ui) -> Strings {
    Strings::new(ui.settings.borrow().language)
}

fn refresh_history_menu(ui: &Ui) {
    let history = ui.history.borrow();
    let menus = ui.menus.borrow();
    menus.undo_item.enable(history.can_undo());
    menus.redo_item.enable(history.can_redo());
}

fn binary_history_symbol(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "mod",
        BinaryOp::Pow => "^",
        BinaryOp::Root => "root",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::Xor => "xor",
        BinaryOp::Lsh => "lsh",
    }
}

fn clean_history_number(calc: &Calculator, text: &str) -> String {
    let mut value = text.trim().to_string();
    if calc.error.is_none() && calc.base == Base::Dec {
        let separator = calc.decimal_separator();
        if value.ends_with(separator) {
            value.pop();
        }
    }
    if value.is_empty() {
        "0".to_string()
    } else {
        value
    }
}

fn localize_scientific_history_expression(calc: &Calculator, expression: String) -> String {
    if calc.decimal_separator() == ',' {
        expression.replace('.', ",")
    } else {
        expression
    }
}

fn unary_history_expression(calc: &Calculator, name: &'static str) -> Option<String> {
    if calc.error.is_some() {
        return None;
    }
    let operand = clean_history_number(calc, &calc.display);
    let actual = match (name, calc.inv, calc.hyp) {
        ("sin", true, true) => "asinh",
        ("cos", true, true) => "acosh",
        ("tan", true, true) => "atanh",
        ("sin", true, false) => "asin",
        ("cos", true, false) => "acos",
        ("tan", true, false) => "atan",
        ("sin", false, true) => "sinh",
        ("cos", false, true) => "cosh",
        ("tan", false, true) => "tanh",
        ("ln", true, _) => "exp",
        ("log", true, _) => "pow10",
        ("dms", true, _) => "dms_inv",
        _ => name,
    };

    Some(match actual {
        "factorial" => format!("{operand}!"),
        "square" => format!("sqr({operand})"),
        "cube" => format!("cube({operand})"),
        "recip" => format!("1/({operand})"),
        "pow10" => format!("10^({operand})"),
        "dms_inv" => format!("dms^-1({operand})"),
        "int" => format!("Int({operand})"),
        "not" => format!("Not({operand})"),
        other => format!("{other}({operand})"),
    })
}

/// Return the operation that will produce a user-visible calculation result.
/// Digit entry, memory bookkeeping, selectors and clearing operations are not
/// included: the side panel is a calculation log, not an input-event trace.
fn history_expression_before_action(calc: &Calculator, action: Action) -> Option<String> {
    match action {
        Action::Eq => match calc.mode {
            Mode::Standard => calc
                .pending_standard_history_parts()
                .map(|(op, lhs, rhs)| {
                    format!(
                        "{} {} {}",
                        clean_history_number(calc, &lhs),
                        binary_history_symbol(op),
                        clean_history_number(calc, &rhs)
                    )
                }),
            Mode::Scientific => calc
                .pending_scientific_history_expression()
                .map(|expression| localize_scientific_history_expression(calc, expression)),
        },
        // Standard mode evaluates the previous pending operation when a new
        // binary operator is pressed while a right-hand operand is being entered.
        Action::Bin(_) | Action::KeyboardStar if calc.mode == Mode::Standard && calc.is_entering_value() => calc
            .pending_standard_history_parts()
            .map(|(op, lhs, rhs)| {
                format!(
                    "{} {} {}",
                    clean_history_number(calc, &lhs),
                    binary_history_symbol(op),
                    clean_history_number(calc, &rhs)
                )
            }),
        Action::Unary(name) => unary_history_expression(calc, name),
        Action::Percent if calc.error.is_none() => Some(format!(
            "{}%",
            clean_history_number(calc, &calc.display)
        )),
        Action::StatsAvg => Some(format!("Ave(n={})", calc.stats.len())),
        Action::StatsSum => Some(format!("Sum(n={})", calc.stats.len())),
        Action::StatsDev => Some(format!("s(n={})", calc.stats.len())),
        _ => None,
    }
}

fn append_calculation_history(ui: &Ui, expression: String) {
    let (result, value, decimal_separator) = {
        let calc = ui.calc.borrow();
        (
            clean_history_number(&calc, &calc.display),
            calc.value().ok(),
            calc.decimal_separator(),
        )
    };
    ui.calculation_log
        .borrow_mut()
        .push_localized(expression, result, value, decimal_separator);
    refresh_calculation_history(ui);
}

fn refresh_calculation_history(ui: &Ui) {
    let strings = strings_for(ui);
    let decimal_separator = ui.settings.borrow().decimal_separator.as_char();
    let log = ui.calculation_log.borrow();
    let mut rendered = String::new();
    let mut ranges = Vec::new();
    let mut utf16_pos = 0usize;

    for (index, entry) in log.newest_first().enumerate() {
        if index != 0 {
            const SEPARATOR: &str = "\r\n\r\n";
            rendered.push_str(SEPARATOR);
            utf16_pos += SEPARATOR.encode_utf16().count();
        }

        let start = utf16_pos;
        let expression = entry.localized_expression(decimal_separator);
        let result = match strings.runtime_message(&entry.result) {
            Some(localized) => localized.to_owned(),
            None => entry.localized_result(decimal_separator),
        };
        let block = format!("{} =\r\n{}", expression, result);
        rendered.push_str(&block);
        utf16_pos += block.encode_utf16().count();
        ranges.push(HistoryEntryRange {
            start,
            end: utf16_pos,
            newest_index: index,
        });
    }
    drop(log);

    *ui.history_entry_ranges.borrow_mut() = ranges;
    ui.history_panel.text.set_value(&rendered);
    ui.history_panel.text.set_insertion_point(0);
}

fn history_index_at_text_position(ui: &Ui, position: usize) -> Option<usize> {
    ui.history_entry_ranges
        .borrow()
        .iter()
        .find(|range| range.start <= position && position < range.end)
        .map(|range| range.newest_index)
}

fn recall_calculation_history(ui: &Rc<Ui>, newest_index: usize) {
    let value = ui
        .calculation_log
        .borrow()
        .newest(newest_index)
        .and_then(|entry| entry.value);
    let Some(value) = value else {
        // Error-message entries remain visible but are not re-executed.  There
        // is no numeric result to recall from them.
        return;
    };

    mutate_calculator(ui, |calc| calc.recall_history_value(value));
    refresh(ui);
    ui.frame.raise();
    ui.frame.set_focus();
}

fn bind_history_text_recall(ui: &Rc<Ui>, text: TextCtrl) {
    let hwnd = text.get_handle();
    let ui_c = Rc::clone(ui);
    text.on_mouse_left_down(move |event: WindowEventData| {
        if let WindowEventData::MouseButton(mouse) = &event {
            if let Some(point) = mouse.get_position() {
                if let Some(position) =
                    platform::history_text_position_from_point(hwnd, point.x, point.y)
                {
                    if let Some(index) = history_index_at_text_position(&ui_c, position) {
                        recall_calculation_history(&ui_c, index);
                    }
                }
            }
        }
        // Consume the click: History behaves like a recall surface, not like a
        // selectable/editable text field with a caret.
        event.skip(false);
    });
}

fn bind_history_recall(ui: &Rc<Ui>) {
    bind_history_text_recall(ui, ui.history_panel.text.clone());
}

fn clear_calculation_history(ui: &Ui) {
    if ui.calculation_log.borrow().is_empty() {
        return;
    }
    ui.calculation_log.borrow_mut().clear();
    refresh_calculation_history(ui);
}

/// Apply a calculator-state mutation as one undoable user action. No-op input
/// (for example an invalid digit in binary mode, or selecting an already active
/// checkbox) does not create an empty history entry.
fn mutate_calculator<F>(ui: &Ui, mutation: F) -> bool
where
    F: FnOnce(&mut Calculator),
{
    let before = ui.calc.borrow().clone();
    {
        let mut calc = ui.calc.borrow_mut();
        mutation(&mut calc);
    }
    let changed = ui.calc.borrow().ne(&before);
    if changed {
        ui.history.borrow_mut().record(before);
    }
    refresh_history_menu(ui);
    changed
}

fn restore_history_state(ui: &Rc<Ui>, mut state: Calculator) {
    // Decimal punctuation is a persisted presentation preference rather than an
    // undoable calculator operation. Old snapshots are normalized to whatever
    // separator the user has currently selected before they are restored.
    let separator = ui.settings.borrow().decimal_separator;
    state.set_decimal_separator(separator.as_char());
    let mode = state.mode;
    *ui.calc.borrow_mut() = state;
    sync_mode_surface(ui, mode);
    refresh_history_menu(ui);
    refresh(ui);
}

fn undo(ui: &Rc<Ui>) {
    platform::dismiss_context_tooltip();
    let current = ui.calc.borrow().clone();
    let previous = ui.history.borrow_mut().undo(current);
    if let Some(state) = previous {
        restore_history_state(ui, state);
    } else {
        refresh_history_menu(ui);
    }
}

fn redo(ui: &Rc<Ui>) {
    platform::dismiss_context_tooltip();
    let current = ui.calc.borrow().clone();
    let next = ui.history.borrow_mut().redo(current);
    if let Some(state) = next {
        restore_history_state(ui, state);
    } else {
        refresh_history_menu(ui);
    }
}

/// Display a synchronous application-owned message.  Windows continues to use
/// the existing native MessageBoxW path.  wxGTK previously wrote these messages
/// only to stderr, which made Help-viewer failures invisible and made About look
/// as though it did nothing when OpenCalc was started from a desktop launcher.
fn persist_settings(ui: &Ui) {
    let strings = strings_for(ui);
    if let Err(error) = ui.settings.borrow_mut().save() {
        frontend::show_modal_message(
            &ui.frame,
            strings.calculator_title(),
            &format!("{}: {error}", strings.settings_error_prefix()),
        );
    }
}

fn apply_language(ui: &Ui) {
    let language = ui.settings.borrow().language;
    let strings = Strings::new(language);

    ui.frame.set_title(strings.calculator_title());
    if let Some(stats) = ui.stats_box.borrow().as_ref() {
        stats.frame.set_title(strings.statistics_box_title());
    }

    // Relabel the existing menu bar in place.  This is safer than replacing
    // the menu bar from inside the menu-selection callback that requested the
    // language change, and it preserves all radio/check state and bindings.
    if let Some(menu_bar) = ui.frame.get_menu_bar() {
        menu_bar.set_menu_label(0, strings.edit_menu());
        menu_bar.set_menu_label(1, strings.view_menu());
        menu_bar.set_menu_label(2, strings.help_menu());

        for (id, label) in [
            (ID_UNDO, strings.undo()),
            (ID_REDO, strings.redo()),
            (ID_COPY, strings.copy()),
            (ID_PASTE, strings.paste()),
            (ID_SCIENTIFIC, strings.scientific()),
            (ID_STANDARD, strings.standard()),
            (ID_GRAPH_PANEL, strings.graph()),
            (ID_HISTORY_PANEL, strings.history()),
            (ID_SEPARATOR_PERIOD, strings.period_separator()),
            (ID_SEPARATOR_COMMA, strings.comma_separator()),
            (ID_HELP_TOPICS, strings.help_topics()),
            (ID_ABOUT, strings.about_opencalc()),
        ] {
            if let Some(item) = menu_bar.find_item(id) {
                item.set_label(label);
            }
        }

        let menus = ui.menus.borrow();
        menus
            .separator_submenu_item
            .set_label(strings.decimal_separator_menu());
        menus
            .language_submenu_item
            .set_label(strings.language_menu());

        // Keep wx menu-help text localized too, even though this Calculator
        // intentionally has no status bar in which wxWidgets normally shows it.
        if let Some(edit_menu) = menu_bar.get_menu(0) {
            edit_menu.set_help_string(ID_UNDO, strings.undo_help());
            edit_menu.set_help_string(ID_REDO, strings.redo_help());
            edit_menu.set_help_string(ID_COPY, strings.copy_help());
            edit_menu.set_help_string(ID_PASTE, strings.paste_help());
        }
        if let Some(view_menu) = menu_bar.get_menu(1) {
            view_menu.set_help_string(ID_SCIENTIFIC, strings.scientific_help());
            view_menu.set_help_string(ID_STANDARD, strings.standard_help());
            view_menu.set_help_string(ID_GRAPH_PANEL, strings.graph_help());
            view_menu.set_help_string(ID_HISTORY_PANEL, strings.history_help());
        }
        if let Some(separator_menu) = menus.separator_submenu_item.get_sub_menu() {
            separator_menu.set_help_string(ID_SEPARATOR_PERIOD, strings.separator_help());
            separator_menu.set_help_string(ID_SEPARATOR_COMMA, strings.separator_help());
        }
        if let Some(language_menu) = menus.language_submenu_item.get_sub_menu() {
            language_menu.set_help_string(ID_LANGUAGE_ENGLISH, strings.language_help());
            language_menu.set_help_string(ID_LANGUAGE_PORTUGUESE, strings.language_help());
            language_menu.set_help_string(ID_LANGUAGE_SPANISH, strings.language_help());
        }
        if let Some(help_menu) = menu_bar.get_menu(2) {
            help_menu.set_help_string(ID_HELP_TOPICS, strings.help_topics_help());
            help_menu.set_help_string(ID_ABOUT, strings.about_title());
        }
    }

    ui.history_panel.title.set_label(strings.history_title());
    ui.history_panel.clear_button.set_label(strings.clear_history());
    ui.graph_panel.function_label.set_label(strings.graph_function());
    ui.graph_panel.plot_button.set_label(strings.graph_plot());
    ui.graph_panel.reset_button.set_label(strings.graph_reset_view());
    ui.graph_panel.export_button.set_label(strings.graph_export());
    refresh_graph_root_label(ui);

    refresh_context_help(ui);
    refresh_calculation_history(ui);
}

fn set_language(ui: &Rc<Ui>, language: Language) {
    if ui.settings.borrow().language == language {
        return;
    }
    platform::dismiss_context_tooltip();
    ui.settings.borrow_mut().language = language;
    apply_language(ui);
    refresh(ui);
    persist_settings(ui);
}

fn set_graph_visible(ui: &Rc<Ui>, visible: bool) {
    if ui.settings.borrow().graph_visible == visible {
        return;
    }
    platform::dismiss_context_tooltip();
    ui.settings.borrow_mut().graph_visible = visible;
    ui.menus.borrow().graph_item.check(visible);
    let mode = ui.calc.borrow().mode;
    sync_mode_surface(ui, mode);
    if visible {
        // Showing Graph must not implicitly turn the Function field into the
        // keyboard target. It receives focus naturally only when clicked.
        ui.frame.set_focus();
        ui.graph_panel.canvas.refresh(true, None);
    }
    persist_settings(ui);
}

fn set_graph_width(ui: &Rc<Ui>, width: i32) {
    let width = width.clamp(GRAPH_MIN_W, GRAPH_MAX_W);
    if ui.graph_width.replace(width) == width {
        return;
    }
    if ui.settings.borrow().graph_visible {
        let mode = ui.calc.borrow().mode;
        sync_mode_surface(ui, mode);
        ui.graph_panel.canvas.refresh(false, None);
    }
}

fn set_history_visible(ui: &Rc<Ui>, visible: bool) {
    if ui.settings.borrow().history_visible == visible {
        return;
    }
    platform::dismiss_context_tooltip();
    ui.settings.borrow_mut().history_visible = visible;
    ui.menus.borrow().history_item.check(visible);
    let mode = ui.calc.borrow().mode;
    sync_mode_surface(ui, mode);
    persist_settings(ui);
}

fn set_decimal_separator(ui: &Rc<Ui>, separator: DecimalSeparator) {
    if ui.settings.borrow().decimal_separator == separator {
        return;
    }
    platform::dismiss_context_tooltip();

    ui.calc.borrow_mut().set_decimal_separator(separator.as_char());
    ui.settings.borrow_mut().decimal_separator = separator;

    // The two keypad variants both have a decimal Action.  Updating the native
    // labels in place preserves focus/state and avoids rebuilding the controls.
    let label = separator.as_char().to_string();
    for (button, action) in &ui.action_buttons {
        if matches!(action, Action::Dot) {
            button.set_label(&label);
        }
    }

    {
        let menus = ui.menus.borrow();
        menus.separator_items[0].check(separator == DecimalSeparator::Period);
        menus.separator_items[1].check(separator == DecimalSeparator::Comma);
    }

    // Calculation-log entries are stored with invariant numeric punctuation.
    // Re-rendering here makes every existing History expression and numeric
    // result follow the newly selected decimal separator immediately.
    refresh_calculation_history(ui);
    refresh_graph_root_label(ui);
    ui.graph_panel.canvas.refresh(true, None);
    persist_settings(ui);
    refresh(ui);
}

const STANDARD_ACTION_BUTTON_COUNT: usize = 27;

/// Return the visible button action whose face should depress for a keyboard
/// accelerator. `KeyboardStar` is an input-state action used only to recognize
/// `**`; visually it is still the ordinary multiplication key.
fn keyboard_visual_action(action: Action) -> Action {
    match action {
        Action::KeyboardStar => Action::Bin(BinaryOp::Mul),
        Action::Digit(ch) if ch.is_ascii_hexdigit() => Action::Digit(ch.to_ascii_uppercase()),
        other => other,
    }
}

/// Give keyboard operation the same tactile feedback as the original CALC.EXE
/// accelerator path: the matching visible calculator button depresses briefly
/// even though the command came from the keyboard rather than the mouse.
fn animate_keyboard_action(ui: &Ui, action: Action) {
    let target = keyboard_visual_action(action);
    let mode = ui.calc.borrow().mode;
    let buttons = if mode == Mode::Standard {
        &ui.action_buttons[..STANDARD_ACTION_BUTTON_COUNT.min(ui.action_buttons.len())]
    } else {
        &ui.action_buttons[STANDARD_ACTION_BUTTON_COUNT.min(ui.action_buttons.len())..]
    };
    if let Some((button, _)) = buttons.iter().find(|(_, candidate)| *candidate == target) {
        platform::pulse_classic_button(button.get_handle());
    }
}

fn perform_from_keyboard(ui: &Rc<Ui>, action: Action) {
    animate_keyboard_action(ui, action);
    perform(ui, action);
}

fn bind_keyboard(ui: &Rc<Ui>) {
    // CALC.EXE loads accelerator resource SA and runs TranslateAcceleratorA
    // in its main message loop before ordinary TranslateMessage/DispatchMessage.
    // When the Statistics dialog exists, IsDialogMessageA gets the first chance
    // to consume its dialog-navigation keys. Consequently the Calculator
    // shortcuts belong to the top-level calculator, not to whichever ordinary
    // child happens to own focus.  wxEVT_CHAR is focus-local, so install the
    // same handler on the frame and on the only calculator child controls that
    // can intentionally take focus (the Scientific selectors).  The Graph
    // expression TextCtrl is deliberately excluded: typing there must remain
    // ordinary expression editing.
    ui.frame.on_char(calculator_char_handler(Rc::clone(ui)));
    ui.calculator_host.on_char(calculator_char_handler(Rc::clone(ui)));
    ui.standard_panel.on_char(calculator_char_handler(Rc::clone(ui)));
    ui.scientific_panel.on_char(calculator_char_handler(Rc::clone(ui)));
    ui.inv.on_char(calculator_char_handler(Rc::clone(ui)));
    ui.hyp.on_char(calculator_char_handler(Rc::clone(ui)));
    for radio in &ui.base_radios {
        radio.on_char(calculator_char_handler(Rc::clone(ui)));
    }
    for radio in &ui.angle_radios {
        radio.on_char(calculator_char_handler(Rc::clone(ui)));
    }
}

fn calculator_char_handler(ui: Rc<Ui>) -> impl Fn(WindowEventData) {
    move |event: WindowEventData| {
        let WindowEventData::Keyboard(key) = &event else {
            event.skip(true);
            return;
        };

        let unicode = key
            .get_unicode_key()
            .and_then(|code| u32::try_from(code).ok())
            .and_then(char::from_u32);
        let raw_code = key.get_key_code().unwrap_or(0);

        // Alt belongs to menu navigation and must never accidentally become a
        // Calculator accelerator when combined with another modifier.
        if key.alt_down() {
            event.skip(true);
            return;
        }

        // The Win95 SA accelerator table uses Ctrl+L/R/M/P for memory and
        // Ctrl+S/A/T/D for Statistics.  Keep modern Edit accelerators owned by
        // wxWidgets so Ctrl+C/V/Z/Y continue through the menu commands.
        if key.control_down() || key.cmd_down() {
            // Under wxMSW, get_unicode_key() can expose the control character
            // (for example Ctrl+L as U+000C), while get_key_code() still carries
            // the virtual L key. Prefer that printable virtual key here.
            let key_char = if (65..=90).contains(&raw_code) || (97..=122).contains(&raw_code) {
                u32::try_from(raw_code)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|ch| ch.to_ascii_lowercase())
            } else {
                unicode.map(|ch| ch.to_ascii_lowercase())
            };
            let scientific = ui.calc.borrow().mode == Mode::Scientific;
            let handled = match key_char {
                Some('l') => { perform_from_keyboard(&ui, Action::MemC); true }
                Some('r') => { perform_from_keyboard(&ui, Action::MemR); true }
                Some('m') => { perform_from_keyboard(&ui, Action::MemS); true }
                Some('p') => { perform_from_keyboard(&ui, Action::MemAdd); true }
                Some('s') if scientific => { perform_from_keyboard(&ui, Action::StatsOpen); true }
                Some('a') if scientific => { perform_from_keyboard(&ui, Action::StatsAvg); true }
                Some('t') if scientific => { perform_from_keyboard(&ui, Action::StatsSum); true }
                Some('d') if scientific => { perform_from_keyboard(&ui, Action::StatsDev); true }
                _ if matches!(raw_code, 322 | 384) => { perform_from_keyboard(&ui, Action::Copy); true }
                _ => false,
            };
            event.skip(!handled);
            return;
        }

        // Shift+Insert is the other clipboard accelerator present in SA.
        if key.shift_down() && matches!(raw_code, 322 | 384) {
            perform_from_keyboard(&ui, Action::Paste);
            event.skip(false);
            return;
        }

        // Non-character keys and keypad aliases.  Values are the stable
        // wxWidgets wxKeyCode numbers used by wxDragon.
        let handled_raw = match raw_code {
            8 | 314 | 376 => { perform_from_keyboard(&ui, Action::Back); true },
            27 => { perform_from_keyboard(&ui, Action::C); true },
            127 | 385 => { perform_from_keyboard(&ui, Action::CE); true },
            13 | 370 | 386 => { perform_from_keyboard(&ui, Action::Eq); true },
            322 | 384 if ui.calc.borrow().mode == Mode::Scientific => {
                perform_from_keyboard(&ui, Action::StatsDat);
                true
            },
            324..=333 => {
                let digit = char::from(b'0' + (raw_code - 324) as u8);
                perform_from_keyboard(&ui, Action::Digit(digit));
                true
            }
            334 | 387 => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Mul)); true },
            335 | 388 => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Add)); true },
            336 | 338 | 389 | 391 => { perform_from_keyboard(&ui, Action::Dot); true },
            337 | 390 => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Sub)); true },
            339 | 392 => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Div)); true },
            // F1 remains the Help menu accelerator.
            341 if ui.calc.borrow().mode == Mode::Scientific => {
                if ui.calc.borrow().base == Base::Dec {
                    select_angle(&ui, 0, AngleMode::Degrees);
                    true
                } else {
                    false
                }
            }
            // The original F3 selects Word in non-decimal modes. OpenCalc does
            // not expose the Win95 Dword/Word/Byte width selector, so leave F3
            // unused rather than inventing a hidden state.
            342 => false,
            343 if ui.calc.borrow().mode == Mode::Scientific => {
                if ui.calc.borrow().base == Base::Dec {
                    select_angle(&ui, 2, AngleMode::Grads);
                    true
                } else {
                    false
                }
            }
            344 if ui.calc.borrow().mode == Mode::Scientific => {
                select_base(&ui, 0, Base::Hex);
                true
            },
            345 if ui.calc.borrow().mode == Mode::Scientific => {
                if ui.calc.borrow().base == Base::Dec {
                    select_angle(&ui, 1, AngleMode::Radians);
                } else {
                    select_base(&ui, 1, Base::Dec);
                }
                true
            }
            346 if ui.calc.borrow().mode == Mode::Scientific => {
                select_base(&ui, 2, Base::Oct);
                true
            },
            347 if ui.calc.borrow().mode == Mode::Scientific => {
                select_base(&ui, 3, Base::Bin);
                true
            },
            348 => { perform_from_keyboard(&ui, Action::Sign); true },
            _ => false,
        };
        if handled_raw {
            event.skip(false);
            return;
        }

        let Some(ch) = unicode.or_else(|| {
            u32::try_from(raw_code)
                .ok()
                .and_then(char::from_u32)
        }) else {
            event.skip(true);
            return;
        };

        let lower = ch.to_ascii_lowercase();
        let mode = ui.calc.borrow().mode;
        let handled = match ch {
            // Both punctuation spellings are accepted regardless of the active
            // locale, matching the recovered clipboard/keyboard behaviour.
            '.' | ',' => { perform_from_keyboard(&ui, Action::Dot); true },
            '0'..='9' => { perform_from_keyboard(&ui, Action::Digit(ch)); true },
            'a'..='f' | 'A'..='F' => { perform_from_keyboard(&ui, Action::Digit(ch)); true },
            '+' => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Add)); true },
            '-' | '−' => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Sub)); true },
            '*' => { perform_from_keyboard(&ui, Action::KeyboardStar); true },
            '×' => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Mul)); true },
            '/' | '÷' => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Div)); true },
            '%' => {
                if mode == Mode::Scientific {
                    perform_from_keyboard(&ui, Action::Bin(BinaryOp::Mod));
                } else {
                    perform_from_keyboard(&ui, Action::Percent);
                }
                true
            }
            '(' => { perform_from_keyboard(&ui, Action::Open); true },
            ')' => { perform_from_keyboard(&ui, Action::Close); true },
            '=' | '\r' | '\n' => { perform_from_keyboard(&ui, Action::Eq); true },
            '\u{8}' => { perform_from_keyboard(&ui, Action::Back); true },
            '\u{1b}' => { perform_from_keyboard(&ui, Action::C); true },
            '@' => {
                if mode == Mode::Scientific {
                    perform_from_keyboard(&ui, Action::Unary("square"));
                } else {
                    perform_from_keyboard(&ui, Action::Unary("sqrt"));
                }
                true
            }
            '!' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("factorial")); true },
            '#' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("cube")); true },
            '&' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::And)); true },
            '|' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Or)); true },
            '^' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Xor)); true },
            '<' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Lsh)); true },
            '~' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("not")); true },
            ';' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("int")); true },
            _ => match lower {
                'r' => { perform_from_keyboard(&ui, Action::Unary("recip")); true },
                's' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("sin")); true },
                'o' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("cos")); true },
                't' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("tan")); true },
                'n' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("ln")); true },
                'l' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("log")); true },
                'm' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("dms")); true },
                'x' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Unary("exp")); true },
                'y' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Bin(BinaryOp::Pow)); true },
                'p' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::Pi); true },
                'i' if mode == Mode::Scientific => {
                    let checked = !ui.calc.borrow().inv;
                    ui.inv.set_value(checked);
                    mutate_calculator(&ui, |calc| calc.inv = checked);
                    refresh(&ui);
                    true
                }
                'h' if mode == Mode::Scientific => {
                    let checked = !ui.calc.borrow().hyp;
                    ui.hyp.set_value(checked);
                    mutate_calculator(&ui, |calc| calc.hyp = checked);
                    refresh(&ui);
                    true
                }
                'v' if mode == Mode::Scientific => { perform_from_keyboard(&ui, Action::ToggleFE); true },
                _ => false,
            },
        };
        event.skip(!handled);
    }
}

fn perform(ui: &Rc<Ui>, action: Action) {
    match action {
        Action::Copy => {
            let strings = strings_for(ui);
            let graph_editor = ui.graph_panel.expression.get_handle();
            let value = if platform::editable_owns_clipboard(graph_editor) {
                // Match native TextCtrl behavior: copy only the selected
                // function text and do nothing when there is no selection.
                let Some(selected) = platform::selected_text(graph_editor) else {
                    return;
                };
                selected
            } else {
                let raw = ui.calc.borrow().display.clone();
                strings.runtime_message(&raw).unwrap_or(raw.as_str()).to_string()
            };
            if let Err(error) = platform::copy_text(&value) {
                let localized = strings.runtime_message(&error).unwrap_or(error.as_str());
                frontend::show_modal_message(&ui.frame, strings.calculator_title(), localized);
            }
            return;
        }
        Action::Paste => {
            match platform::paste_text() {
                Ok(Some(text)) => {
                    // Frame-level Paste accelerators must not steal Paste from
                    // the editable Function field on either native frontend. When that TextCtrl owns
                    // focus, preserve ordinary editor semantics: replace the
                    // current selection (or insert at the caret) and do not
                    // evaluate/log the polynomial as a numeric calculation.
                    let graph_editor = ui.graph_panel.expression.get_handle();
                    if platform::editable_owns_clipboard(graph_editor) {
                        if !platform::insert_text_at_selection(graph_editor, &text) {
                            // Portable backends without a native editable bridge
                            // still get a safe full-field replacement.
                            ui.graph_panel.expression.set_value(&text);
                        }
                        return;
                    }

                    let expression = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    mutate_calculator(ui, |calc| calc.paste_expression(&text));
                    if !expression.is_empty() {
                        append_calculation_history(ui, expression);
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    let strings = strings_for(ui);
                    let localized = strings.runtime_message(&error).unwrap_or(error.as_str());
                    frontend::show_modal_message(&ui.frame, strings.calculator_title(), localized);
                }
            }
            refresh(ui);
            return;
        }
        Action::About => {
            let strings = strings_for(ui);
            frontend::show_modal_message(&ui.frame, strings.about_title(), strings.about_body());
            return;
        }
        Action::Help => {
            let language = ui.settings.borrow().language;
            if let Err(error) = platform::launch_help(language) {
                let strings = strings_for(ui);
                let localized = strings.runtime_message(&error).unwrap_or(error.as_str());
                frontend::show_modal_message(&ui.frame, strings.help_title(), localized);
            }
            return;
        }
        Action::StatsOpen => {
            toggle_stats_box(ui);
            return;
        }
        _ => {}
    }

    let history_expression = {
        let calc = ui.calc.borrow();
        history_expression_before_action(&calc, action)
    };

    mutate_calculator(ui, |calc| {
        match action {
            Action::Digit(ch) => calc.digit(ch),
            Action::Dot => calc.decimal_point(),
            Action::Back => calc.backspace(),
            Action::CE => calc.clear_entry(),
            Action::C => calc.clear_all(),
            Action::Sign => calc.sign(),
            Action::Eq => calc.equals(),
            Action::Percent => calc.percent(),
            Action::Bin(op) => calc.binary(op),
            Action::KeyboardStar => calc.keyboard_star(),
            Action::Unary(name) => calc.unary(name),
            Action::MemC => calc.memory_clear(),
            Action::MemR => calc.memory_recall(),
            Action::MemS => calc.memory_store(),
            Action::MemAdd => calc.memory_add(),
            Action::Pi => calc.set_value(std::f64::consts::PI),
            Action::Open => calc.open_paren(),
            Action::Close => calc.close_paren(),
            Action::StatsDat => calc.stat_dat(),
            Action::StatsAvg => calc.stat_avg(),
            Action::StatsSum => calc.stat_sum(),
            Action::StatsDev => calc.stat_stddev(),
            Action::ToggleFE => calc.toggle_fe(),
            Action::Copy | Action::Paste | Action::About | Action::Help | Action::StatsOpen => {}
        }
    });
    if let Some(expression) = history_expression {
        append_calculation_history(ui, expression);
    }
    refresh(ui);
}

fn set_mode(ui: &Rc<Ui>, mode: Mode) {
    mutate_calculator(ui, |calc| calc.set_mode(mode));
    sync_mode_surface(ui, mode);
    refresh(ui);
}

fn mode_surface_size(mode: Mode) -> (i32, i32) {
    if mode == Mode::Scientific {
        (SCI_W, SCI_H)
    } else {
        (STD_W, STD_H)
    }
}

fn configured_history_width(ui: &Ui) -> i32 {
    dp(ui.settings.borrow().history_width.clamp(
        MIN_HISTORY_WIDTH,
        MAX_HISTORY_WIDTH,
    ))
}

fn splitter_metric(ui: &Ui, logical: i32) -> i32 {
    platform::scale_classic_control_metric(ui.splitter.get_handle(), logical)
}

/// Convert an actual splitter-pixel width back to the 96-DPI source units
/// persisted in OpenCalc.cfg.  Using a large forward-scaled reference
/// avoids introducing a second DPI code path while keeping rounding stable.
fn history_source_width_from_splitter_pixels(ui: &Ui, pixels: i32) -> i32 {
    let reference = splitter_metric(ui, dp(1000)).max(1) as i64;
    ((pixels.max(1) as i64 * 1000 + reference / 2) / reference)
        .clamp(MIN_HISTORY_WIDTH as i64, MAX_HISTORY_WIDTH as i64) as i32
}

fn size_calculator_panel(ui: &Ui, mode: Mode) {
    let (width, height) = mode_surface_size(mode);
    let panel = if mode == Mode::Scientific {
        &ui.scientific_panel
    } else {
        &ui.standard_panel
    };
    let pixel_width = platform::scale_classic_control_metric(panel.get_handle(), width);
    let pixel_height = platform::scale_classic_control_metric(panel.get_handle(), height);
    if !platform::set_window_rect_pixels(
        panel.get_handle(),
        0,
        0,
        pixel_width,
        pixel_height,
    ) {
        panel.set_size_with_pos(0, 0, width, height);
    }
}

fn sync_mode_surface(ui: &Ui, mode: Mode) {
    let scientific = mode == Mode::Scientific;
    let history_visible = ui.settings.borrow().history_visible;
    let graph_visible = ui.settings.borrow().graph_visible;
    let (calculator_width, height) = mode_surface_size(mode);
    let history_width = configured_history_width(ui);
    let history_gutter = if history_visible {
        frontend::history_leading_gutter()
    } else {
        0
    };
    let sash_extent = if history_visible {
        frontend::history_sash_extent()
    } else {
        0
    };
    let calculator_pane_width = calculator_width + history_gutter;
    let splitter_width = calculator_pane_width
        + if history_visible {
            sash_extent + history_width
        } else {
            0
        };
    let graph_panel_width = ui.graph_width.get().clamp(GRAPH_MIN_W, GRAPH_MAX_W);
    let graph_width = if graph_visible { graph_panel_width } else { 0 };
    let total_width = graph_width + splitter_width;

    ui.standard_panel.show(!scientific);
    ui.scientific_panel.show(scientific);

    {
        let menus = ui.menus.borrow();
        menus.standard_item.check(!scientific);
        menus.scientific_item.check(scientific);
        menus.history_item.check(history_visible);
        menus.graph_item.check(graph_visible);
    }

    // Every programmatic split/resize is guarded because wxSplitterWindow also
    // emits SASH_POS_CHANGED for SetSashPosition.  User drags are handled only
    // after this guard is released.
    ui.splitter_adjusting.set(true);
    unlock_frame_size(&ui.frame);
    platform::enable_frame_resizing(ui.frame.get_handle());
    let fit_before_children = frontend::fit_frame_before_child_layout();

    if !history_visible && ui.history_split.get() {
        if ui.splitter.unsplit(Some(&ui.history_panel.panel)) {
            ui.history_split.set(false);
        }
        ui.history_panel.panel.show(false);
        ui.history_panel.separator.show(false);
    }

    // The root surface owns the optional Graph pane plus the existing
    // Calculator/History splitter. The Calculator pane remains fixed-size.
    // wxMSW's native DPI fitter must run before child HWND positioning, while
    // wxGTK must first shrink hidden/active children or GTK will preserve the
    // previous Scientific-mode minimum allocation and leave empty space.
    if fit_before_children {
        fit_frame_to_surface(&ui.frame, &ui.root_surface, total_width, height);
    }

    let graph_x_pixels = platform::scale_classic_control_metric(
        ui.root_surface.get_handle(),
        graph_width,
    );
    let splitter_width_pixels = platform::scale_classic_control_metric(
        ui.root_surface.get_handle(),
        splitter_width,
    );
    let height_pixels = platform::scale_classic_control_metric(
        ui.root_surface.get_handle(),
        height,
    );
    if graph_visible {
        ui.graph_panel.panel.show(true);
        let graph_positioned = platform::set_window_rect_pixels(
            ui.graph_panel.panel.get_handle(),
            0,
            0,
            graph_x_pixels.max(1),
            height_pixels.max(1),
        );
        if !graph_positioned {
            frontend::position_graph_panel(&ui.graph_panel.panel, graph_panel_width, height);
        }
        if !layout_graph_panel_pixels(&ui.graph_panel) {
            layout_graph_panel_logical(&ui.graph_panel, graph_panel_width, height);
        }
    } else {
        ui.graph_panel.panel.show(false);
    }
    let splitter_positioned = platform::set_window_rect_pixels(
        ui.splitter.get_handle(),
        graph_x_pixels,
        0,
        splitter_width_pixels.max(1),
        height_pixels.max(1),
    );
    if !splitter_positioned {
        frontend::position_splitter(
            &ui.splitter,
            graph_width,
            splitter_width,
            height,
        );
    }

    if history_visible {
        ui.history_panel.panel.show(true);
        let use_native_sash = frontend::history_uses_native_sash();
        ui.history_panel.separator.show(!use_native_sash);
        let sash = splitter_metric(ui, calculator_pane_width);
        if !ui.history_split.get() {
            if ui
                .splitter
                .split_vertically(&ui.calculator_host, &ui.history_panel.panel, sash)
            {
                ui.history_split.set(true);
            }
        }
        if ui.history_split.get() {
            ui.splitter.set_sash_position(sash, true);
        }
        // Windows retains its custom pointer-transparent boundary painter.
        // Linux exposes the real, CSS-neutral GTK sash so native hit-testing and
        // drag feedback cannot be blocked by a child decoration.
        if !use_native_sash {
            ui.history_panel.separator.raise();
        }
    }

    // wxSplitterWindow owns calculator_host's rectangle.  Only the active
    // recovered Calculator panel is explicitly sized inside that host, so its
    // controls keep their original Standard/Scientific coordinates at high DPI.
    size_calculator_panel(ui, mode);

    if history_visible && ui.history_split.get() {
        if !layout_history_panel_pixels(ui, &ui.history_panel) {
            let size = ui.history_panel.panel.get_size();
            layout_history_panel_logical(
                ui,
                &ui.history_panel,
                size.width.max(1),
                size.height.max(1),
            );
        }
    }

    // Force the nested splitter allocation before asking GTK to shrink the
    // top-level frame.  This is the critical reverse transition: without it,
    // the old Scientific child request can keep the Standard frame oversized.
    ui.splitter.layout();
    ui.root_surface.layout();
    if !fit_before_children {
        fit_frame_to_surface(&ui.frame, &ui.root_surface, total_width, height);
        ui.splitter.layout();
        ui.root_surface.layout();
    }

    platform::disable_frame_resizing(ui.frame.get_handle());
    lock_frame_size(&ui.frame);
    ui.splitter_adjusting.set(false);
}

/// The Calculator pane itself is intentionally fixed.  A sash drag therefore
/// means "make History wider/narrower": after wx reports the user's final sash
/// position, grow/shrink the outer frame by the same delta and put the sash back
/// on the recovered Calculator boundary.  This keeps every Calculator control
/// untouched while retaining a real native draggable splitter.
fn accept_history_sash_change(ui: &Rc<Ui>) {
    if ui.splitter_adjusting.get() || !ui.settings.borrow().history_visible {
        return;
    }

    let mode = ui.calc.borrow().mode;
    let (calculator_width, _) = mode_surface_size(mode);
    let canonical_sash = splitter_metric(
        ui,
        calculator_width + frontend::history_leading_gutter(),
    );
    let actual_sash = ui.splitter.sash_position();
    if actual_sash <= 0 || actual_sash == canonical_sash {
        return;
    }

    let old_history_pixels = splitter_metric(ui, configured_history_width(ui));
    let min_history_pixels = splitter_metric(ui, dp(MIN_HISTORY_WIDTH));
    let max_history_pixels = splitter_metric(ui, dp(MAX_HISTORY_WIDTH));
    let new_history_pixels = (old_history_pixels + canonical_sash - actual_sash)
        .clamp(min_history_pixels, max_history_pixels);
    let new_source_width = history_source_width_from_splitter_pixels(ui, new_history_pixels);

    if ui.settings.borrow().history_width != new_source_width {
        ui.settings.borrow_mut().history_width = new_source_width;
        persist_settings(ui);
    }

    // Normalize even when clamping/rounding produced the same persisted value:
    // the transient user drag may still have changed the left pane by pixels.
    sync_mode_surface(ui, mode);
}


fn plot_graph(ui: &Ui) {
    platform::dismiss_context_tooltip();
    let expression = ui.graph_panel.expression.get_value();
    let context = ui.calc.borrow().eval_context();
    let succeeded = ui
        .graph_panel
        .model
        .borrow_mut()
        .plot(&expression, context)
        .is_ok();
    ui.graph_panel.reset_button.enable(succeeded);
    ui.graph_panel.export_button.enable(succeeded);
    refresh_graph_root_label(ui);
    ui.graph_panel.canvas.refresh(false, None);
}

fn replot_existing_graph(ui: &Ui) {
    if !ui.graph_panel.model.borrow().has_plot() {
        return;
    }
    let expression = ui.graph_panel.expression.get_value();
    let context = ui.calc.borrow().eval_context();
    let succeeded = ui
        .graph_panel
        .model
        .borrow_mut()
        .plot(&expression, context)
        .is_ok();
    ui.graph_panel.reset_button.enable(succeeded);
    ui.graph_panel.export_button.enable(succeeded);
    refresh_graph_root_label(ui);
    ui.graph_panel.canvas.refresh(false, None);
}

fn graph_root_label(ui: &Ui) -> String {
    let strings = strings_for(ui);
    let separator = ui.settings.borrow().decimal_separator.as_char();
    let model = ui.graph_panel.model.borrow();
    if let Some(error) = model.error() {
        return format!("{}: {error}", strings.graph_plot_error());
    }
    match model.root_result() {
        RootResult::NotPlotted => strings.graph_roots_not_plotted().to_string(),
        RootResult::Roots(roots) => format!(
            "{}{}",
            strings.graph_roots_visible(),
            format_root_values(roots, separator)
        ),
        RootResult::None => strings.graph_no_roots().to_string(),
        RootResult::Infinite => strings.graph_infinite_roots().to_string(),
        RootResult::Unreliable => strings.graph_roots_unreliable().to_string(),
    }
}

fn refresh_graph_root_label(ui: &Ui) {
    let label = wrap_graph_status(&graph_root_label(ui), 46, 3);
    ui.graph_panel.roots.set_label(&label);
    // wxStaticText normally resizes itself when SetLabel is called. Restore
    // the authored graph-pane rectangle immediately so a long error cannot
    // extend beyond the pane even before the next frame resize event.
    if !layout_graph_panel_pixels(&ui.graph_panel) {
        let size = ui.graph_panel.panel.get_size();
        layout_graph_panel_logical(&ui.graph_panel, size.width, size.height);
    }
}

fn wrap_graph_status(text: &str, max_chars: usize, max_lines: usize) -> String {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if !current.is_empty()
            && current.chars().count() + separator + word.chars().count() > max_chars
        {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                if let Some(last) = lines.last_mut() {
                    last.push('…');
                }
                return lines.join("\n");
            }
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    lines.join("\n")
}

fn graph_export_format(path: &Path, selected_filter: i32) -> (PathBuf, ExportFormat) {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase);
    let detected = match extension.as_deref() {
        Some("png") => Some(ExportFormat::Png),
        Some("jpg") | Some("jpeg") => Some(ExportFormat::Jpeg),
        Some("svg") => Some(ExportFormat::Svg),
        _ => None,
    };
    if let Some(format) = detected {
        return (path.to_path_buf(), format);
    }

    let (extension, format) = match selected_filter {
        1 => ("jpg", ExportFormat::Jpeg),
        2 => ("svg", ExportFormat::Svg),
        _ => ("png", ExportFormat::Png),
    };
    let mut completed = path.to_path_buf();
    completed.set_extension(extension);
    (completed, format)
}

fn export_graph(ui: &Ui) {
    platform::dismiss_context_tooltip();
    if !ui.graph_panel.model.borrow().has_plot() {
        refresh_graph_root_label(ui);
        return;
    }

    let strings = strings_for(ui);
    let dialog = FileDialog::builder(&ui.frame)
        .with_message(strings.graph_export_title())
        .with_wildcard(
            "PNG image (*.png)|*.png|JPEG image (*.jpg;*.jpeg)|*.jpg;*.jpeg|SVG vector image (*.svg)|*.svg",
        )
        .with_style(FileDialogStyle::Save | FileDialogStyle::OverwritePrompt)
        .build();
    dialog.set_filename("graph.png");
    dialog.set_filter_index(0);
    if dialog.show_modal() != ID_OK {
        return;
    }
    let Some(selected_path) = dialog.get_path() else {
        return;
    };
    let (path, format) = graph_export_format(Path::new(&selected_path), dialog.get_filter_index());
    let (width, height) = platform::client_size_pixels(ui.graph_panel.canvas.get_handle())
        .map(|(width, height)| (width.max(1) as u32, height.max(1) as u32))
        .unwrap_or_else(|| {
            let size = ui.graph_panel.canvas.get_client_size();
            (size.width.max(1) as u32, size.height.max(1) as u32)
        });
    let separator = ui.settings.borrow().decimal_separator.as_char();
    let root_summary = graph_root_label(ui);
    let export_result = ui.graph_panel.model.borrow().export(
        &path,
        format,
        (width, height),
        separator,
        &root_summary,
    );
    if let Err(error) = export_result {
        frontend::show_modal_message(
            &ui.frame,
            strings.calculator_title(),
            &format!("{}: {error}", strings.graph_export_error()),
        );
    }
}

fn bind_graph(ui: &Rc<Ui>) {
    {
        let ui_c = Rc::clone(ui);
        ui.graph_panel.plot_button.on_click(move |_| plot_graph(&ui_c));
    }
    {
        let ui_c = Rc::clone(ui);
        ui.graph_panel.expression.on_enter_pressed(move |_| plot_graph(&ui_c));
    }
    {
        let ui_c = Rc::clone(ui);
        ui.graph_panel.reset_button.on_click(move |_| {
            ui_c.graph_panel.model.borrow_mut().reset_view();
            refresh_graph_root_label(&ui_c);
            ui_c.graph_panel.canvas.refresh(false, None);
        });
    }
    {
        let ui_c = Rc::clone(ui);
        ui.graph_panel.export_button.on_click(move |_| export_graph(&ui_c));
    }

    // Plotters draws into a fresh wxAutoBufferedPaintDC for each native paint
    // event. This keeps the graph flicker-free while preserving a real wxPanel.
    {
        let canvas = ui.graph_panel.canvas.clone();
        let ui_c = Rc::clone(ui);
        ui.graph_panel.canvas.on_paint(move |event| {
            let dc = AutoBufferedPaintDC::new(&canvas);
            let area = WxBackend::new(&dc).into_drawing_area();
            let separator = ui_c.settings.borrow().decimal_separator.as_char();
            let _ = ui_c.graph_panel.model.borrow().draw(&area, separator);
            event.skip(false);
        });
    }
    // Suppress the ordinary erase pass; the buffered paint handler fills the
    // complete canvas itself.
    ui.graph_panel.canvas.on_erase_background(|event| event.skip(false));

    {
        let canvas = ui.graph_panel.canvas.clone();
        let drag = Rc::clone(&ui.graph_panel.drag);
        let model = Rc::clone(&ui.graph_panel.model);
        ui.graph_panel.canvas.on_mouse_left_down(move |event| {
            if let WindowEventData::MouseButton(mouse) = &event {
                if let Some(position) = mouse.get_position() {
                    *drag.borrow_mut() = Some((position, model.borrow().viewport()));
                    canvas.capture_mouse();
                }
            }
            event.skip(false);
        });
    }
    {
        let canvas = ui.graph_panel.canvas.clone();
        let drag = Rc::clone(&ui.graph_panel.drag);
        let ui_c = Rc::clone(ui);
        ui.graph_panel.canvas.on_mouse_motion(move |event| {
            let WindowEventData::MouseMotion(mouse) = &event else {
                event.skip(true);
                return;
            };
            let Some((start, initial)) = *drag.borrow() else {
                event.skip(true);
                return;
            };
            let Some(position) = mouse.get_position() else {
                event.skip(true);
                return;
            };
            let size = canvas.get_client_size();
            ui_c.graph_panel.model.borrow_mut().pan_from(
                initial,
                position.x - start.x,
                position.y - start.y,
                size.width,
                size.height,
            );
            refresh_graph_root_label(&ui_c);
            canvas.refresh(false, None);
            event.skip(false);
        });
    }
    {
        let canvas = ui.graph_panel.canvas.clone();
        let drag = Rc::clone(&ui.graph_panel.drag);
        ui.graph_panel.canvas.on_mouse_left_up(move |event| {
            *drag.borrow_mut() = None;
            if canvas.has_capture() {
                canvas.release_mouse();
            }
            event.skip(false);
        });
    }
    {
        let canvas = ui.graph_panel.canvas.clone();
        let ui_c = Rc::clone(ui);
        ui.graph_panel.canvas.on_mouse_wheel(move |event| {
            let WindowEventData::MouseButton(mouse) = &event else {
                event.skip(true);
                return;
            };
            let rotation = mouse.event.get_wheel_rotation();
            let position = mouse.get_position().unwrap_or(Point::new(0, 0));
            let size = canvas.get_client_size();
            let focus_x = if size.width > 0 {
                position.x as f64 / size.width as f64
            } else {
                0.5
            };
            let focus_y = if size.height > 0 {
                position.y as f64 / size.height as f64
            } else {
                0.5
            };
            ui_c
                .graph_panel
                .model
                .borrow_mut()
                .zoom(rotation, focus_x, focus_y);
            refresh_graph_root_label(&ui_c);
            canvas.refresh(false, None);
            event.skip(false);
        });
    }

    // Refit canvas children after a DPI transition or mode-height change.
    {
        let ui_c = Rc::clone(ui);
        ui.graph_panel.panel.on_size(move |event| {
            if !layout_graph_panel_pixels(&ui_c.graph_panel) {
                let size = ui_c.graph_panel.panel.get_size();
                layout_graph_panel_logical(
                    &ui_c.graph_panel,
                    size.width.max(1),
                    size.height.max(1),
                );
            }
            ui_c.graph_panel.canvas.refresh(false, None);
            event.skip(true);
        });
    }
}

fn bind_splitter(ui: &Rc<Ui>) {
    {
        let ui_c = Rc::clone(ui);
        ui.splitter.on_sash_position_changed(move |_| {
            accept_history_sash_change(&ui_c);
        });
    }

    // Child controls in the History pane need to follow its native client size
    // after a sash release, DPI transition, or Standard/Scientific height change.
    {
        let ui_c = Rc::clone(ui);
        ui.history_panel.panel.on_size(move |event: WindowEventData| {
            if !layout_history_panel_pixels(&ui_c, &ui_c.history_panel) {
                let size = ui_c.history_panel.panel.get_size();
                layout_history_panel_logical(
                    &ui_c,
                    &ui_c.history_panel,
                    size.width.max(1),
                    size.height.max(1),
                );
            }
            event.skip(true);
        });
    }
}

fn restore_open_companions(ui: &Rc<Ui>) {
    // History restores naturally as a Calculator child. The owned Statistics
    // window may be temporarily hidden while Calculator is minimized, so show
    // it again without changing the position chosen by the user.
    show_open_stats_box(ui);
}

fn bind_companion_tracking(ui: &Rc<Ui>) {
    // An already-open Statistics Box keeps its screen position. Calculator move,
    // resize, mode, History-width and activation events must not recenter it.
    // Only application-group activation and minimize/restore need tracking.
    {
        let ui_c = Rc::clone(ui);
        ui.frame.on_activate(move |event: WindowEventData| {
            if let WindowEventData::Activate(activation) = &event {
                // Treat Calculator + Statistics as one active window group.
                // This updates Statistics' active appearance/z-order without
                // moving it or stealing real keyboard focus.
                set_statistics_application_active(&ui_c, activation.is_active());
                frontend::restore_main_keyboard_focus(&ui_c, activation.is_active());
            }
            event.skip(true);
        });
    }

    // The native restore observer is still useful for Statistics because owned
    // top-level windows are temporarily suppressed when their owner minimizes.
    {
        let ui_c = Rc::clone(ui);
        platform::install_window_state_notifier(
            ui.frame.get_handle(),
            Box::new(move |minimized| {
                if minimized {
                    ui_c.main_was_minimized.set(true);
                } else if ui_c.main_was_minimized.replace(false) {
                    restore_open_companions(&ui_c);
                }
            }),
        );
    }
}

fn layout_graph_panel_logical(graph: &GraphPanel, width: i32, height: i32) {
    let separator_width = frontend::graph_separator_width();
    let content_w = (width - 2 * GRAPH_MARGIN - separator_width).max(1);
    let field_y = GRAPH_MARGIN + GRAPH_LABEL_H;
    let expression_w = (content_w - GRAPH_PLOT_W - GRAPH_GAP).max(1);
    let button_y = (height - GRAPH_MARGIN - GRAPH_BUTTON_H).max(field_y + GRAPH_FIELD_H);
    let roots_y = (button_y - GRAPH_GAP - GRAPH_ROOTS_H).max(field_y + GRAPH_FIELD_H + GRAPH_GAP + 8);
    let canvas_y = field_y + GRAPH_FIELD_H + GRAPH_GAP;
    let canvas_h = (roots_y - GRAPH_GAP - canvas_y).max(8);
    let border = 2;

    graph.separator.set_size_with_pos((width - separator_width).max(0), 0, separator_width, height.max(1));
    graph.function_label.set_size_with_pos(GRAPH_MARGIN, GRAPH_MARGIN, content_w, GRAPH_LABEL_H);
    graph.expression.set_size_with_pos(GRAPH_MARGIN, field_y, expression_w, GRAPH_FIELD_H);
    graph.plot_button.set_size_with_pos(GRAPH_MARGIN + expression_w + GRAPH_GAP, field_y, GRAPH_PLOT_W, GRAPH_FIELD_H);
    graph.canvas_frame.set_size_with_pos(GRAPH_MARGIN, canvas_y, content_w, canvas_h);
    graph.canvas.set_size_with_pos(GRAPH_MARGIN + border, canvas_y + border, (content_w - 2 * border).max(1), (canvas_h - 2 * border).max(1));
    graph.roots.set_size_with_pos(GRAPH_MARGIN, roots_y, content_w, GRAPH_ROOTS_H);
    graph.reset_button.set_size_with_pos(GRAPH_MARGIN, button_y, GRAPH_BUTTON_W, GRAPH_BUTTON_H);
    graph.export_button.set_size_with_pos((width - GRAPH_MARGIN - separator_width - GRAPH_BUTTON_W).max(GRAPH_MARGIN), button_y, GRAPH_BUTTON_W, GRAPH_BUTTON_H);
}

fn layout_graph_panel_pixels(graph: &GraphPanel) -> bool {
    let Some((width, height)) = platform::client_size_pixels(graph.panel.get_handle()) else {
        return false;
    };
    let scale = |logical| platform::scale_classic_control_metric(graph.panel.get_handle(), logical);
    let margin = scale(GRAPH_MARGIN);
    let label_h = scale(GRAPH_LABEL_H);
    let field_h = scale(GRAPH_FIELD_H);
    let plot_w = scale(GRAPH_PLOT_W);
    let button_w = scale(GRAPH_BUTTON_W);
    let button_h = scale(GRAPH_BUTTON_H);
    let roots_h = scale(GRAPH_ROOTS_H);
    let gap = scale(GRAPH_GAP);
    let separator_w = scale(frontend::graph_separator_width()).max(2);
    let content_w = (width - 2 * margin - separator_w).max(1);
    let field_y = margin + label_h;
    let expression_w = (content_w - plot_w - gap).max(1);
    let button_y = (height - margin - button_h).max(field_y + field_h);
    let roots_y = (button_y - gap - roots_h).max(field_y + field_h + gap + 8);
    let canvas_y = field_y + field_h + gap;
    let canvas_h = (roots_y - gap - canvas_y).max(8);
    let border = platform::scale_classic_control_metric(graph.panel.get_handle(), 2).max(2);

    let rects = [
        (graph.separator.get_handle(), width - separator_w, 0, separator_w, height),
        (graph.function_label.get_handle(), margin, margin, content_w, label_h),
        (graph.expression.get_handle(), margin, field_y, expression_w, field_h),
        (graph.plot_button.get_handle(), margin + expression_w + gap, field_y, plot_w, field_h),
        (graph.canvas_frame.get_handle(), margin, canvas_y, content_w, canvas_h),
        (graph.canvas.get_handle(), margin + border, canvas_y + border, (content_w - 2 * border).max(1), (canvas_h - 2 * border).max(1)),
        (graph.roots.get_handle(), margin, roots_y, content_w, roots_h),
        (graph.reset_button.get_handle(), margin, button_y, button_w, button_h),
        (graph.export_button.get_handle(), (width - margin - separator_w - button_w).max(margin), button_y, button_w, button_h),
    ];
    rects.into_iter().all(|(hwnd, x, y, w, h)| {
        platform::set_window_rect_pixels(hwnd, x, y, w.max(1), h.max(1))
    })
}

fn layout_history_panel_logical(ui: &Ui, history: &HistoryPanel, width: i32, height: i32) {
    let margin = HISTORY_MARGIN;
    let header_h = HISTORY_HEADER_H;
    let gap = HISTORY_GAP;
    let button_w = HISTORY_BUTTON_W;
    let button_h = HISTORY_BUTTON_H;
    let button_bottom = HISTORY_BUTTON_BOTTOM;
    let content_w = (width - 2 * margin).max(1);
    let text_y = margin + header_h + gap;
    let button_y = (height - button_bottom - button_h).max(text_y + gap + 1);
    let text_h = (button_y - gap - text_y).max(1);
    let button_x = (width - margin - button_w).max(margin);

    // Centre the decorative rule over the native sash boundary instead of
    // placing it entirely on the Calculator side. At high DPI this moves it
    // roughly two physical pixels to the right without changing the sash.
    let separator_width = frontend::history_separator_width();
    let separator_x = frontend::history_separator_x(
        ui.splitter.sash_position(),
        separator_width,
    );
    history.separator.set_size_with_pos(
        separator_x,
        0,
        separator_width,
        height.max(1),
    );
    history.title.set_size_with_pos(margin, margin, content_w, header_h);
    history.text.set_size_with_pos(margin, text_y, content_w, text_h);
    history.clear_button.set_size_with_pos(button_x, button_y, button_w, button_h);
}

fn layout_history_panel_pixels(ui: &Ui, history: &HistoryPanel) -> bool {
    let Some((width, height)) = platform::client_size_pixels(history.panel.get_handle()) else {
        return false;
    };
    let scale = |logical| platform::scale_classic_control_metric(history.panel.get_handle(), logical);
    let margin = scale(HISTORY_MARGIN);
    let header_h = scale(HISTORY_HEADER_H);
    let gap = scale(HISTORY_GAP);
    let button_w = scale(HISTORY_BUTTON_W);
    let button_h = scale(HISTORY_BUTTON_H);
    let button_bottom = scale(HISTORY_BUTTON_BOTTOM);
    let content_w = (width - 2 * margin).max(1);
    let text_y = margin + header_h + gap;
    let button_y = (height - button_bottom - button_h).max(text_y + gap + 1);
    let text_h = (button_y - gap - text_y).max(1);
    let button_x = (width - margin - button_w).max(margin);
    let separator_w = scale(frontend::history_separator_width()).max(2);
    let separator_x = frontend::history_separator_x(
        ui.splitter.sash_position(),
        separator_w,
    );

    platform::set_window_rect_pixels(
        history.separator.get_handle(),
        separator_x,
        0,
        separator_w,
        height.max(1),
    ) && platform::set_window_rect_pixels(
        history.title.get_handle(),
        margin,
        margin,
        content_w,
        header_h,
    ) && platform::set_window_rect_pixels(
        history.text.get_handle(),
        margin,
        text_y,
        content_w,
        text_h,
    ) && platform::set_window_rect_pixels(
        history.clear_button.get_handle(),
        button_x,
        button_y,
        button_w,
        button_h,
    )
}

fn unlock_frame_size(frame: &Frame) {
    frame.set_min_size(Size::new(-1, -1));
    frame.set_max_size(Size::new(-1, -1));
}

fn lock_frame_size(frame: &Frame) {
    frontend::lock_frame_size(frame);
}

fn fit_frame_to_surface<W: WxWidget>(
    frame: &Frame,
    surface: &W,
    logical_width: i32,
    logical_height: i32,
) {
    // wxDragon's public Size values are not a reliable way to infer the final
    // physical HWND size during Per-Monitor-V2 realization on wxMSW.  Let the
    // Windows helper work entirely in HWND/client pixels after realization,
    // using GetDpiForWindow and GetClientRect.  The surface may be either an
    // ordinary Panel (Statistics Box) or the Calculator's wxSplitterWindow.
    frame.set_min_size(Size::new(-1, -1));
    frame.set_max_size(Size::new(-1, -1));

    if !platform::fit_calculator_surface(
        frame.get_handle(),
        surface.get_handle(),
        logical_width,
        logical_height,
    ) {
        frontend::fit_frame_fallback(frame, surface, logical_width, logical_height);
    }
}

fn fit_frame_to_panel(frame: &Frame, panel: &Panel, logical_width: i32, logical_height: i32) {
    fit_frame_to_surface(frame, panel, logical_width, logical_height);
}

fn refresh(ui: &Ui) {
    let calc = ui.calc.borrow();
    let strings = strings_for(ui);
    let display = strings
        .runtime_message(&calc.display)
        .unwrap_or(calc.display.as_str());
    ui.standard_display.set_value(display);
    ui.scientific_display.set_value(display);
    let memory = if calc.memory_set { "M" } else { "" };
    ui.standard_memory.set_label(memory);
    ui.scientific_memory.set_label(memory);
    let parens = if calc.paren_depth() == 0 {
        String::new()
    } else {
        format!("(={}", calc.paren_depth())
    };
    ui.scientific_parens.set_label(&parens);

    ui.inv.set_value(calc.inv);
    ui.hyp.set_value(calc.hyp);

    let base_index = match calc.base {
        Base::Hex => 0,
        Base::Dec => 1,
        Base::Oct => 2,
        Base::Bin => 3,
    };
    for (i, radio) in ui.base_radios.iter().enumerate() {
        radio.set_value(i == base_index);
    }

    let angle_index = match calc.angle {
        AngleMode::Degrees => 0,
        AngleMode::Radians => 1,
        AngleMode::Grads => 2,
    };
    for (i, radio) in ui.angle_radios.iter().enumerate() {
        radio.set_value(i == angle_index);
    }

    refresh_stats(ui);
}

fn standard_button_defs() -> Vec<ButtonDef> {
    // Coordinates are the recovered layout with the old fake 43 px title/menu
    // band removed.  Widths/heights are unchanged.
    // Back/CE/C span the same band as the five keypad columns to their right:
    // Back starts on the "7" column (x=64) and C ends on the sqrt/%/1-over-x
    // column's right edge (227 + 28 = 255), which is also the display's right
    // edge (10 + 245 = 255).  The previous widths (60/55/55) stopped C at 244,
    // so the command row was 11 units short of the display and the keypad.
    // Scientific mode already follows this rule: its Back/CE/C are equal width
    // and C ends at 436 + 48 = 484, exactly the scientific display's right edge.
    let mut defs = vec![
        def(64, 39, 60, 27, "Back", Action::Back, Tone::Maroon),
        def(129, 39, 60, 27, "CE", Action::CE, Tone::Maroon),
        def(195, 39, 60, 27, "C", Action::C, Tone::Maroon),
    ];

    let rows = [('7', '8', '9'), ('4', '5', '6'), ('1', '2', '3')];
    let mem = [
        ("MC", Action::MemC),
        ("MR", Action::MemR),
        ("MS", Action::MemS),
        ("M+", Action::MemAdd),
    ];
    let ops = [
        ("/", Action::Bin(BinaryOp::Div)),
        ("*", Action::Bin(BinaryOp::Mul)),
        ("-", Action::Bin(BinaryOp::Sub)),
        ("+", Action::Bin(BinaryOp::Add)),
    ];
    let funcs = [
        ("sqrt", Action::Unary("sqrt")),
        ("%", Action::Percent),
        ("1/x", Action::Unary("recip")),
        ("=", Action::Eq),
    ];

    for r in 0usize..4 {
        let y = 71 + r as i32 * 32;
        defs.push(def(10, y, 38, 27, mem[r].0, mem[r].1, Tone::Red));
        if r < 3 {
            let (a, b, c) = rows[r];
            defs.push(def(64, y, 38, 27, digit_label(a), Action::Digit(a), Tone::Blue));
            defs.push(def(107, y, 38, 27, digit_label(b), Action::Digit(b), Tone::Blue));
            defs.push(def(150, y, 38, 27, digit_label(c), Action::Digit(c), Tone::Blue));
        } else {
            defs.push(def(64, y, 38, 27, "0", Action::Digit('0'), Tone::Blue));
            defs.push(def(107, y, 38, 27, "+/-", Action::Sign, Tone::Blue));
            defs.push(def(150, y, 38, 27, ".", Action::Dot, Tone::Blue));
        }
        defs.push(def(193, y, 29, 27, ops[r].0, ops[r].1, Tone::Red));
        defs.push(def(227, y, 28, 27, funcs[r].0, funcs[r].1, Tone::Red));
    }
    defs
}

fn scientific_button_defs() -> Vec<ButtonDef> {
    // The original scientific keypad is five horizontal rows by eleven
    // columns.  Buildfix2 accidentally transposed the first 20 commands into
    // a modern-looking 4x5 block and moved logic/hex keys to extra rows.
    // This table follows the Win95 visual order shown by the reference binary.
    let mut defs = vec![
        def(330, SCI_COMMAND_Y, 48, 27, "Back", Action::Back, Tone::Maroon),
        def(383, SCI_COMMAND_Y, 48, 27, "CE", Action::CE, Tone::Maroon),
        def(436, SCI_COMMAND_Y, 48, 27, "C", Action::C, Tone::Maroon),
    ];

    let rows: [[(&str, Action, Tone); 11]; 5] = [
        [
            ("Sta", Action::StatsOpen, Tone::Navy),
            ("F-E", Action::ToggleFE, Tone::Magenta),
            ("(", Action::Open, Tone::Magenta),
            (")", Action::Close, Tone::Magenta),
            ("MC", Action::MemC, Tone::Red),
            ("7", Action::Digit('7'), Tone::Blue),
            ("8", Action::Digit('8'), Tone::Blue),
            ("9", Action::Digit('9'), Tone::Blue),
            ("/", Action::Bin(BinaryOp::Div), Tone::Red),
            ("Mod", Action::Bin(BinaryOp::Mod), Tone::Red),
            ("And", Action::Bin(BinaryOp::And), Tone::Red),
        ],
        [
            ("Ave", Action::StatsAvg, Tone::Navy),
            ("dms", Action::Unary("dms"), Tone::Magenta),
            ("Exp", Action::Unary("exp"), Tone::Magenta),
            ("ln", Action::Unary("ln"), Tone::Magenta),
            ("MR", Action::MemR, Tone::Red),
            ("4", Action::Digit('4'), Tone::Blue),
            ("5", Action::Digit('5'), Tone::Blue),
            ("6", Action::Digit('6'), Tone::Blue),
            ("*", Action::Bin(BinaryOp::Mul), Tone::Red),
            ("Or", Action::Bin(BinaryOp::Or), Tone::Red),
            ("Xor", Action::Bin(BinaryOp::Xor), Tone::Red),
        ],
        [
            ("Sum", Action::StatsSum, Tone::Navy),
            ("sin", Action::Unary("sin"), Tone::Magenta),
            ("x^y", Action::Bin(BinaryOp::Pow), Tone::Magenta),
            ("log", Action::Unary("log"), Tone::Magenta),
            ("MS", Action::MemS, Tone::Red),
            ("1", Action::Digit('1'), Tone::Blue),
            ("2", Action::Digit('2'), Tone::Blue),
            ("3", Action::Digit('3'), Tone::Blue),
            ("-", Action::Bin(BinaryOp::Sub), Tone::Red),
            ("Lsh", Action::Bin(BinaryOp::Lsh), Tone::Red),
            ("Not", Action::Unary("not"), Tone::Red),
        ],
        [
            ("s", Action::StatsDev, Tone::Navy),
            ("cos", Action::Unary("cos"), Tone::Magenta),
            ("x^3", Action::Unary("cube"), Tone::Magenta),
            ("n!", Action::Unary("factorial"), Tone::Magenta),
            ("M+", Action::MemAdd, Tone::Red),
            ("0", Action::Digit('0'), Tone::Blue),
            ("+/-", Action::Sign, Tone::Blue),
            (".", Action::Dot, Tone::Blue),
            ("+", Action::Bin(BinaryOp::Add), Tone::Red),
            ("=", Action::Eq, Tone::Red),
            ("Int", Action::Unary("int"), Tone::Red),
        ],
        [
            ("Dat", Action::StatsDat, Tone::Navy),
            ("tan", Action::Unary("tan"), Tone::Magenta),
            ("x^2", Action::Unary("square"), Tone::Magenta),
            ("1/x", Action::Unary("recip"), Tone::Magenta),
            ("PI", Action::Pi, Tone::Navy),
            ("A", Action::Digit('A'), Tone::Navy),
            ("B", Action::Digit('B'), Tone::Navy),
            ("C", Action::Digit('C'), Tone::Navy),
            ("D", Action::Digit('D'), Tone::Navy),
            ("E", Action::Digit('E'), Tone::Navy),
            ("F", Action::Digit('F'), Tone::Navy),
        ],
    ];

    let xs = [13, 65, 105, 145, 197, 249, 289, 330, 370, 409, 449];
    let widths = [35, 35, 35, 35, 35, 35, 35, 35, 35, 35, 35];
    let ys = [
        SCI_KEYPAD_Y,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP * 2,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP * 3,
        SCI_KEYPAD_Y + SCI_KEYPAD_STEP * 4,
    ];

    for (r, row) in rows.iter().enumerate() {
        for (c, (label, action, tone)) in row.iter().copied().enumerate() {
            defs.push(def(xs[c], ys[r], widths[c], 27, label, action, tone));
        }
    }
    defs
}

const fn def(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    label: &'static str,
    action: Action,
    tone: Tone,
) -> ButtonDef {
    ButtonDef {
        x,
        y,
        w,
        h,
        label,
        action,
        tone,
    }
}

fn digit_label(ch: char) -> &'static str {
    match ch {
        '0' => "0",
        '1' => "1",
        '2' => "2",
        '3' => "3",
        '4' => "4",
        '5' => "5",
        '6' => "6",
        '7' => "7",
        '8' => "8",
        '9' => "9",
        'A' => "A",
        'B' => "B",
        'C' => "C",
        'D' => "D",
        'E' => "E",
        'F' => "F",
        _ => "",
    }
}


// ---------------------------------------------------------------------------
// Statistics Box
//
// Geometry recovered from the "SB" DIALOG resource in the original CALC.EXE:
//
//   SB  "Statistics Box"   146 x 86 dialog units, 8pt MS Sans Serif
//     id=407  LISTBOX            3,  3, 140, 50   (WS_VSCROLL|WS_BORDER|LBS_NOTIFY)
//     id=411  BUTTON  "&RET"     4, 58,  28, 14   (BS_DEFPUSHBUTTON)
//     id=410  BUTTON  "&LOAD"   40, 58,  28, 14
//     id=404  BUTTON  "&CD"     76, 58,  28, 14
//     id=405  BUTTON  "C&AD"   112, 58,  28, 14
//     id=409  STATIC  "n="      66, 76,   8,  8
//     id=408  STATIC  "0"       74, 76,  32,  8
//
// The template is expressed in dialog units.  8pt MS Sans Serif has 6x13
// dialog base units, so one horizontal DLU is 6/4 pixels and one vertical DLU
// is 13/8 pixels.  The converted values then go through dp() like every other
// control, so the box follows the calculator's design scale.
const fn sb_x(dlu: i32) -> i32 {
    dp((dlu * 6 + 2) / 4)
}

const fn sb_y(dlu: i32) -> i32 {
    dp((dlu * 13 + 4) / 8)
}

struct StatsBox {
    frame: Frame,
    list: ListBox,
    count: StaticText,
}

fn toggle_stats_box(ui: &Rc<Ui>) {
    // Sta is a true toggle on every frontend. Take the record first, then
    // destroy the native utility so no stale window handle remains cached.
    let existing = { ui.stats_box.borrow_mut().take() };
    if let Some(stats) = existing {
        stats.frame.destroy();
        ui.frame.raise();
        ui.frame.set_focus();
        return;
    }

    let built = build_stats_box(ui);
    *ui.stats_box.borrow_mut() = Some(built);

    // A newly-created box is centered once. Later owner activation, movement,
    // pane resizing, and mode changes preserve the position chosen by the user.
    center_stats_box(ui);

    if let Some(stats) = ui.stats_box.borrow().as_ref() {
        stats.frame.show(true);
        stats.frame.raise();
        frontend::focus_statistics(ui, stats);
    }
    set_statistics_application_active(ui, true);
    refresh_stats(ui);
}

/// Mirror the dataset into the list box and the "n=" counter.  Safe to call
/// from anywhere: it does nothing when the box has never been opened, and
/// try_borrow keeps it inert if it is somehow re-entered from a control event.
fn refresh_stats(ui: &Ui) {
    let Ok(guard) = ui.stats_box.try_borrow() else {
        return;
    };
    let Some(stats) = guard.as_ref() else {
        return;
    };
    let calc = ui.calc.borrow();
    stats.list.clear();
    for value in calc.stats.iter() {
        stats.list.append(&calc.format_decimal_value(*value));
    }
    stats.count.set_label(&calc.stats.len().to_string());
}

/// Index currently highlighted in the list box, if any.
fn stats_selection(ui: &Ui) -> Option<usize> {
    let guard = ui.stats_box.borrow();
    let stats = guard.as_ref()?;
    stats.list.get_selection().map(|index| index as usize)
}

/// LOAD: copy the highlighted datum back into the calculator display.
fn stats_load(ui: &Rc<Ui>) {
    let Some(index) = stats_selection(ui) else {
        return;
    };
    let value = ui.calc.borrow().stats.get(index).copied();
    let Some(value) = value else {
        return;
    };
    mutate_calculator(ui, |calc| calc.set_value(value));
    refresh(ui);
}

/// CD: clear the highlighted datum.
fn stats_clear_datum(ui: &Rc<Ui>) {
    let Some(index) = stats_selection(ui) else {
        return;
    };
    if index >= ui.calc.borrow().stats.len() {
        return;
    }
    mutate_calculator(ui, |calc| {
        calc.stats.remove(index);
    });
    refresh_stats(ui);
}

/// CAD: clear all data.
fn stats_clear_all(ui: &Rc<Ui>) {
    mutate_calculator(ui, |calc| calc.stats.clear());
    refresh_stats(ui);
}

fn set_statistics_application_active(ui: &Ui, active: bool) {
    let Ok(guard) = ui.stats_box.try_borrow() else {
        return;
    };
    let Some(stats) = guard.as_ref() else {
        return;
    };
    platform::set_companion_application_active(stats.frame.get_handle(), active);
}

fn center_stats_box(ui: &Ui) {
    let Ok(guard) = ui.stats_box.try_borrow() else {
        return;
    };
    let Some(stats) = guard.as_ref() else {
        return;
    };

    let positioned = platform::position_statistics_companion(
        ui.frame.get_handle(),
        stats.frame.get_handle(),
    );
    if !positioned {
        let main_pos = ui.frame.get_position();
        let main_size = ui.frame.get_size();
        let stats_size = stats.frame.get_size();
        stats.frame.set_size_with_pos(
            main_pos.x + (main_size.width - stats_size.width) / 2,
            main_pos.y + (main_size.height - stats_size.height) / 2,
            stats_size.width,
            stats_size.height,
        );
    }
}

fn show_open_stats_box(ui: &Ui) {
    let Ok(guard) = ui.stats_box.try_borrow() else {
        return;
    };
    let Some(stats) = guard.as_ref() else {
        return;
    };

    if !ui.frame.is_iconized() {
        stats.frame.show(true);
    }
}

fn build_stats_box(ui: &Rc<Ui>) -> StatsBox {
    let width = sb_x(146);
    let height = sb_y(86);
    let font = frontend::classic_font(FontWeight::Normal);

    let frame = Frame::builder()
        .with_parent(&ui.frame)
        .with_title(strings_for(ui).statistics_box_title())
        .with_size(Size::new(width, height))
        .build();
    // Same fixed-size utility-window policy as the calculator itself.
    frame.remove_style(
        WindowStyle::ThickFrame | WindowStyle::MaximizeBox | WindowStyle::MinimizeBox,
    );
    frame.set_font(&font);
    frontend::apply_surface(&frame);
    platform::set_calculator_icon(frame.get_handle());
    platform::install_context_help_dismissal(frame.get_handle());
    platform::install_companion_activation_guard(ui.frame.get_handle(), frame.get_handle());

    let panel = Panel::builder(&frame)
        .with_pos(Point::new(0, 0))
        .with_size(Size::new(width, height))
        .build();
    panel.set_font(&font);
    frontend::apply_surface(&panel);
    platform::install_context_help_dismissal(panel.get_handle());

    let list = ListBox::builder(&panel)
        .with_pos(Point::new(sb_x(3), sb_y(3)))
        .with_size(Size::new(sb_x(140), sb_y(50)))
        .build();
    list.set_font(&font);

    // CALC.EXE's accelerator loop continued to process Calculator input while
    // the modeless Statistics dialog was active. wx events are focus-local, so
    // bind the same calculator handler to the utility and its focusable list on
    // both Windows and Linux. Handled keys stop here; unhandled list navigation
    // still follows the native control path.
    frame.on_char(calculator_char_handler(Rc::clone(ui)));
    panel.on_char(calculator_char_handler(Rc::clone(ui)));
    list.on_char(calculator_char_handler(Rc::clone(ui)));

    let make_button = |label: &str, dlu_x: i32| {
        let button = Button::builder(&panel)
            .with_label(label)
            .with_pos(Point::new(sb_x(dlu_x), sb_y(58)))
            .with_size(Size::new(sb_x(28), sb_y(14)))
            .build();
        button.set_font(&font);
        platform::install_classic_button_painter(button.get_handle(), 0, 0, 0);
        button.set_can_focus(false);
        button
    };
    let ret = make_button("RET", 4);
    let load = make_button("LOAD", 40);
    let clear_datum = make_button("CD", 76);
    let clear_all = make_button("CAD", 112);

    let n_label = StaticText::builder(&panel)
        .with_label("n=")
        .with_pos(Point::new(sb_x(66), sb_y(76)))
        .with_size(Size::new(sb_x(8), sb_y(8)))
        .build();
    n_label.set_font(&font);

    let count = StaticText::builder(&panel)
        .with_label("0")
        .with_pos(Point::new(sb_x(74), sb_y(76)))
        .with_size(Size::new(sb_x(32), sb_y(8)))
        .build();
    count.set_font(&font);

    // Size the frame through the same helper the calculator window uses.
    // wxMSW realizes child coordinates at the monitor scale under
    // Per-Monitor-V2, so at 200% every control inside this box is laid out at
    // twice the values passed to the builders.  Setting the client area to the
    // unscaled 146x86-dialog-unit size therefore left the list box overflowing
    // and pushed the RET/LOAD/CD/CAD row and the "n=" counter off the bottom
    // edge entirely.  fit_calculator_surface works in real HWND/client pixels
    // and scales the frame and panel by GetDpiForWindow/96 to match.
    fit_frame_to_panel(&frame, &panel, width, height);

    // RET returns to the calculator without closing the box, matching the
    // original: the dataset stays visible while further values are entered.
    {
        let ui_c = Rc::clone(ui);
        ret.on_click(move |_| {
            ui_c.frame.raise();
            ui_c.frame.set_focus();
        });
    }
    {
        let ui_c = Rc::clone(ui);
        load.on_click(move |_| stats_load(&ui_c));
    }
    {
        let ui_c = Rc::clone(ui);
        clear_datum.on_click(move |_| stats_clear_datum(&ui_c));
    }
    {
        let ui_c = Rc::clone(ui);
        clear_all.on_click(move |_| stats_clear_all(&ui_c));
    }

    // Windows keeps real Statistics focus through the owner-side
    // WM_MOUSEACTIVATE/MA_NOACTIVATE guard, so no wx activation repaint hook is
    // installed there. wxGTK still uses its activation event to update the
    // keep-above hint used for the companion window.
    frontend::install_statistics_activation_hook(&frame);

    // Closing the box destroys the wx window, which would leave the cached
    // handles in ui.stats_box dangling.  Drop the record instead so the next
    // "Sta" press builds a fresh one; the dataset itself lives in Calculator
    // and is deliberately preserved across a close.
    {
        let ui_c = Rc::clone(ui);
        frame.on_close(move |event| {
            *ui_c.stats_box.borrow_mut() = None;
            ui_c.frame.raise();
            ui_c.frame.set_focus();
            event.skip(true);
        });
    }

    StatsBox { frame, list, count }
}
